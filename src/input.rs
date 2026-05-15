use anyhow::{Context, Result};
use aws_sdk_s3::Client as S3Client;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum EmailSource {
    Local(PathBuf),
    S3 { bucket: String, key: String },
}

impl std::fmt::Display for EmailSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailSource::Local(path) => write!(f, "{}", path.display()),
            EmailSource::S3 { bucket, key } => write!(f, "s3://{bucket}/{key}"),
        }
    }
}

impl EmailSource {
    pub fn short_name(&self) -> String {
        match self {
            EmailSource::Local(path) => path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            EmailSource::S3 { key, .. } => key
                .rsplit('/')
                .next()
                .unwrap_or(key)
                .to_string(),
        }
    }
}

pub async fn resolve_sources(paths: &[String], s3_client: Option<&S3Client>) -> Result<Vec<EmailSource>> {
    let mut sources = Vec::new();

    for p in paths {
        match parse_uri(p) {
            Uri::S3 { bucket, prefix } => {
                let client = s3_client.context("S3 path provided but AWS is not configured")?;
                let keys = list_s3_objects(client, &bucket, &prefix).await?;
                for key in keys {
                    sources.push(EmailSource::S3 { bucket: bucket.clone(), key });
                }
            }
            Uri::Local(path) => {
                match (path.is_file(), path.is_dir()) {
                    (true, _) => sources.push(EmailSource::Local(path)),
                    (_, true) => walk_dir(&path, &mut sources)?,
                    _ => anyhow::bail!("Path does not exist: {}", path.display()),
                }
            }
        }
    }

    sources.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    sources.dedup_by(|a, b| a.to_string() == b.to_string());
    Ok(sources)
}

pub async fn fetch_email(source: &EmailSource, s3_client: Option<&S3Client>) -> Result<Vec<u8>> {
    match source {
        EmailSource::Local(path) => {
            tokio::fs::read(path)
                .await
                .with_context(|| format!("Failed to read {}", path.display()))
        }
        EmailSource::S3 { bucket, key } => {
            let client = s3_client.context("S3 path but AWS is not configured")?;
            let resp = client
                .get_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await
                .with_context(|| format!("Failed to fetch s3://{bucket}/{key}"))?;

            let bytes = resp
                .body
                .collect()
                .await
                .context("Failed to read S3 object body")?
                .into_bytes()
                .to_vec();

            Ok(bytes)
        }
    }
}

enum Uri {
    S3 { bucket: String, prefix: String },
    Local(PathBuf),
}

fn parse_uri(input: &str) -> Uri {
    if let Some(rest) = input.strip_prefix("s3://") {
        let (bucket, prefix) = match rest.split_once('/') {
            Some((b, p)) => (b.to_string(), p.to_string()),
            None => (rest.to_string(), String::new()),
        };
        Uri::S3 { bucket, prefix }
    } else if let Some(rest) = input.strip_prefix("file://") {
        Uri::Local(PathBuf::from(rest))
    } else {
        Uri::Local(PathBuf::from(input))
    }
}

async fn list_s3_objects(client: &S3Client, bucket: &str, prefix: &str) -> Result<Vec<String>> {
    let mut keys = Vec::new();
    let mut continuation_token: Option<String> = None;

    loop {
        let mut req = client.list_objects_v2().bucket(bucket);
        if !prefix.is_empty() {
            req = req.prefix(prefix);
        }
        if let Some(token) = &continuation_token {
            req = req.continuation_token(token);
        }

        let resp = req.send().await
            .with_context(|| format!("Failed to list s3://{bucket}/{prefix}"))?;

        if let Some(ref contents) = resp.contents {
            for obj in contents {
                if let Some(key) = obj.key() {
                    if key.ends_with(".eml") {
                        keys.push(key.to_string());
                    }
                }
            }
        }

        match resp.next_continuation_token() {
            Some(token) => continuation_token = Some(token.to_string()),
            None => break,
        }
    }

    Ok(keys)
}

fn walk_dir(dir: &Path, sources: &mut Vec<EmailSource>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, sources)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("eml") {
            sources.push(EmailSource::Local(path));
        }
    }
    Ok(())
}
