use anyhow::{Context, Result};
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_s3::Client as S3Client;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::analysis::analyze_image;
use crate::email::parse_email;
use crate::models::{EmailManifest, MailMetadata, MailType};
use crate::storage::Storage;

pub async fn process_s3_email(
    s3_client: &S3Client,
    bedrock_client: &BedrockClient,
    storage: &Storage,
    bucket: &str,
    key: &str,
) -> Result<EmailManifest> {
    tracing::info!(bucket, key, "Fetching email from S3");

    let resp = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .context("Failed to fetch email from S3")?;

    let raw_email = resp
        .body
        .collect()
        .await
        .context("Failed to read S3 object body")?
        .into_bytes()
        .to_vec();

    process_raw_email(bedrock_client, storage, &raw_email).await
}

pub async fn process_raw_email(
    bedrock_client: &BedrockClient,
    storage: &Storage,
    raw_email: &[u8],
) -> Result<EmailManifest> {
    let parsed = parse_email(raw_email)?;
    tracing::info!(
        subject = %parsed.info.subject,
        images = parsed.images.len(),
        "Parsed email"
    );

    let dir = storage.ensure_email_dir(&parsed.info).await?;
    let mut items = Vec::new();

    for image in &parsed.images {
        let image_sha256 = hex::encode(Sha256::digest(&image.data));
        let stored_filename = storage
            .store_image(&dir, &image.data, &image.filename)
            .await?;

        let metadata = match analyze_image(bedrock_client, &image.data, &image.content_type).await
        {
            Ok((from_address, to_address, mail_type, full_text, confidence)) => MailMetadata {
                id: Uuid::new_v4().to_string(),
                image_filename: stored_filename,
                image_sha256,
                from_address,
                to_address,
                mail_type,
                full_text,
                confidence,
                error: None,
            },
            Err(e) => {
                tracing::warn!(
                    image = %image.filename,
                    error = %e,
                    "Analysis failed, storing image with empty metadata"
                );
                MailMetadata {
                    id: Uuid::new_v4().to_string(),
                    image_filename: stored_filename,
                    image_sha256,
                    from_address: None,
                    to_address: None,
                    mail_type: MailType::Unknown,
                    full_text: String::new(),
                    confidence: 0.0,
                    error: Some(format!("{e}")),
                }
            }
        };

        items.push(metadata);
    }

    let manifest = EmailManifest {
        email_subject: parsed.info.subject,
        email_from: parsed.info.from,
        email_date: parsed.info.date,
        email_message_id: parsed.info.message_id,
        processed_at: chrono::Utc::now().to_rfc3339(),
        items,
    };

    storage.store_manifest(&dir, &manifest).await?;
    tracing::info!(count = manifest.items.len(), "Processing complete");

    Ok(manifest)
}
