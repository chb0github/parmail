use anyhow::{Context, Result};
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::email::EmailInfo;
use crate::models::EmailManifest;

pub enum StorageDir {
    Local(PathBuf),
    S3Key(String),
}

impl StorageDir {
    pub fn as_local_path(&self) -> Option<&Path> {
        match self {
            StorageDir::Local(p) => Some(p),
            StorageDir::S3Key(_) => None,
        }
    }
}

pub enum Storage {
    Local { base_dir: PathBuf },
    S3 { client: S3Client, bucket: String, prefix: String },
}

impl Storage {
    pub fn local(base_dir: impl Into<PathBuf>) -> Self {
        Storage::Local { base_dir: base_dir.into() }
    }

    pub fn s3(client: S3Client, bucket: String, prefix: String) -> Self {
        Storage::S3 { client, bucket, prefix }
    }

    pub fn from_uri(uri: &str, s3_client: Option<S3Client>) -> Result<Self> {
        match uri.strip_prefix("s3://") {
            Some(rest) => {
                let client = s3_client.context("S3 storage URI provided but no S3 client available")?;
                let (bucket, prefix) = match rest.split_once('/') {
                    Some((b, p)) => (b.to_string(), p.trim_end_matches('/').to_string()),
                    None => (rest.to_string(), String::new()),
                };
                Ok(Storage::s3(client, bucket, prefix))
            }
            None => {
                let path = uri.strip_prefix("file://").unwrap_or(uri);
                Ok(Storage::local(path))
            }
        }
    }

    pub async fn ensure_email_dir(&self, info: &EmailInfo) -> Result<StorageDir> {
        match self {
            Storage::Local { base_dir } => {
                let dir = base_dir.join(info.date_folder()).join(info.dir_name());
                fs::create_dir_all(&dir)
                    .await
                    .context("Failed to create email storage directory")?;
                Ok(StorageDir::Local(dir))
            }
            Storage::S3 { prefix, .. } => {
                let key_prefix = match prefix.is_empty() {
                    true => format!("{}/{}", info.date_folder(), info.dir_name()),
                    false => format!("{}/{}/{}", prefix, info.date_folder(), info.dir_name()),
                };
                Ok(StorageDir::S3Key(key_prefix))
            }
        }
    }

    pub async fn store_image(
        &self,
        dir: &StorageDir,
        data: &[u8],
        filename: &str,
    ) -> Result<String> {
        match (self, dir) {
            (Storage::Local { .. }, StorageDir::Local(path)) => {
                let file_path = path.join(filename);
                fs::write(&file_path, data)
                    .await
                    .context("Failed to write image file")?;
                Ok(filename.to_string())
            }
            (Storage::S3 { client, bucket, .. }, StorageDir::S3Key(prefix)) => {
                let key = format!("{prefix}/{filename}");
                client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(ByteStream::from(data.to_vec()))
                    .content_type("image/jpeg")
                    .send()
                    .await
                    .with_context(|| format!("Failed to upload s3://{bucket}/{key}"))?;
                Ok(filename.to_string())
            }
            _ => anyhow::bail!("Mismatched storage and directory types"),
        }
    }

    pub async fn store_manifest(&self, dir: &StorageDir, manifest: &EmailManifest) -> Result<()> {
        let json = serde_json::to_string_pretty(manifest)?;

        match (self, dir) {
            (Storage::Local { .. }, StorageDir::Local(path)) => {
                let file_path = path.join("manifest.json");
                fs::write(&file_path, &json)
                    .await
                    .context("Failed to write manifest")?;
            }
            (Storage::S3 { client, bucket, .. }, StorageDir::S3Key(prefix)) => {
                let key = format!("{prefix}/manifest.json");
                client
                    .put_object()
                    .bucket(bucket)
                    .key(&key)
                    .body(ByteStream::from(json.into_bytes()))
                    .content_type("application/json")
                    .send()
                    .await
                    .with_context(|| format!("Failed to upload s3://{bucket}/{key}"))?;
            }
            _ => anyhow::bail!("Mismatched storage and directory types"),
        }

        Ok(())
    }
}
