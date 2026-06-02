use anyhow::{Context, Result};
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_s3::Client as S3Client;
use futures::future::join_all;
use xxhash_rust::xxh3::xxh3_64;

use crate::analysis::{analyze_image, ModelConfig};
use crate::email::{group_images_by_piece, is_content_image, parse_email, ExtractedImage};
use crate::models::{Address, AddressField, AddressStatus, ContentHash, EmailManifest, MailImage, MailPiece, MailType, TokenUsage};
use crate::storage::Storage;

pub async fn process_s3_email(
    s3_client: &S3Client,
    bedrock_client: &BedrockClient,
    model: &ModelConfig,
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

    process_raw_email(bedrock_client, model, storage, key, &raw_email).await
}

pub async fn process_raw_email(
    bedrock_client: &BedrockClient,
    model: &ModelConfig,
    storage: &Storage,
    source_file: &str,
    raw_email: &[u8],
) -> Result<EmailManifest> {
    let parsed = parse_email(raw_email)?;
    tracing::info!(
        subject = %parsed.info.subject,
        images = parsed.images.len(),
        "Parsed email"
    );

    if let Some(manifest) = storage.load_valid_manifest(&parsed.info).await {
        tracing::info!(subject = %parsed.info.subject, "Skipping - valid manifest exists");
        return Ok(manifest);
    }

    let dir = storage.ensure_email_dir(&parsed.info).await?;
    let groups = group_images_by_piece(parsed.images);

    let all_images: Vec<&ExtractedImage> = groups.values().flatten().collect();
    let analysis_futures: Vec<_> = all_images.iter()
        .map(|image| analyze_image(bedrock_client, model, &image.data, &image.content_type))
        .collect();
    let analysis_results = join_all(analysis_futures).await;

    let mut result_iter = analysis_results.into_iter();
    let mut mail_pieces = Vec::new();
    let mut to_address: Option<Address> = None;
    let mut to_status = AddressStatus::Redacted;
    let mut total_usage = TokenUsage::default();

    for (piece_id, images) in &groups {
        let mut mailer: Option<MailImage> = None;
        let mut content: Option<MailImage> = None;
        let mut best_from: Option<Address> = None;
        let mut from_status = AddressStatus::Unreadable;
        let mut best_mail_type: MailType = "unknown".to_string();
        let mut best_confidence: f32 = 0.0;
        let mut postmark_date: Option<chrono::NaiveDate> = None;


        for image in images {
            let image_hash = ContentHash {
                value: format!("{:016x}", xxh3_64(&image.data)),
                hash_type: "xxh3".to_string(),
            };
            storage.store_image(&dir, &image.data, &image.filename).await?;

            let analysis_result = result_iter.next().unwrap();
            let (full_text, error) = match analysis_result {
                Ok((analysis, usage)) => {
                    total_usage.input_tokens += usage.input_tokens;
                    total_usage.output_tokens += usage.output_tokens;
                    if to_address.is_none() {
                        if let Some(addr) = analysis.to_address {
                            if addr.street.is_some() {
                                to_address = Some(addr);
                                to_status = AddressStatus::Resolved;
                            }
                        }
                    }
                    if best_from.is_none() {
                        if let Some(addr) = analysis.from_address {
                            if addr.street.is_some() {
                                best_from = Some(addr);
                                from_status = AddressStatus::Resolved;
                            }
                        }
                    }
                    if analysis.confidence.unwrap_or(0.0) > best_confidence {
                        best_confidence = analysis.confidence.unwrap_or(0.0);
                        best_mail_type = analysis.mail_type;
                    }
                    if postmark_date.is_none() {
                        postmark_date = analysis.postmark_date;
                    }
                    (analysis.full_text, None)
                }
                Err(e) => {
                    tracing::warn!(image = %image.filename, error = %e, "Analysis failed");
                    from_status = AddressStatus::NotAnalyzed;
                    if to_address.is_none() {
                        to_status = AddressStatus::NotAnalyzed;
                    }
                    (String::new(), Some(format!("{e}")))
                }
            };

            let mail_image = MailImage {
                filename: image.filename.clone(),
                hash: image_hash,
                full_text,
                error,
            };

            if is_content_image(&image.filename) {
                content = Some(mail_image);
            } else {
                mailer = Some(mail_image);
            }
        }

        let piece_hash = xxh3_64(piece_id.as_bytes());
        mail_pieces.push(MailPiece {
            id: format!("{:016x}", piece_hash),
            from_address: AddressField { address: best_from, status: from_status },
            mail_type: best_mail_type,
            confidence: best_confidence,
            postmark_date,
            mailer,
            content,
        });
    }

    let received_date = chrono::NaiveDate::parse_from_str(&parsed.info.date_folder(), "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Utc::now().date_naive());
    let email_id = format!("{:016x}", xxh3_64(parsed.info.message_id.as_bytes()));
    let manifest = EmailManifest {
        id: email_id,
        model_id: model.model_id.clone(),
        source_file: source_file.to_string(),
        email_subject: parsed.info.subject,
        email_from: parsed.info.from,
        email_date: parsed.info.date,
        received_date,
        email_message_id: parsed.info.message_id,
        processed_at: chrono::Utc::now().to_rfc3339(),
        to_address: AddressField { address: to_address, status: to_status },
        mail_pieces,
        usage: total_usage,
    };

    storage.store_manifest(&dir, &manifest).await?;
    tracing::info!(count = manifest.mail_pieces.len(), "Processing complete");

    Ok(manifest)
}
