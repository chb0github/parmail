use anyhow::Result;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_s3::Client as S3Client;
use lambda_runtime::{service_fn, LambdaEvent};

use crate::models::S3Event;
use crate::processor::process_s3_email;
use crate::storage::LocalStorage;

type LambdaError = Box<dyn std::error::Error + Send + Sync>;

pub async fn run_lambda() -> Result<()> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3_client = S3Client::new(&config);
    let bedrock_client = BedrockClient::new(&config);

    let storage_dir =
        std::env::var("STORAGE_DIR").unwrap_or_else(|_| "/tmp/parmail".to_string());

    let handler = service_fn(move |event: LambdaEvent<S3Event>| {
        let s3 = s3_client.clone();
        let bedrock = bedrock_client.clone();
        let store = LocalStorage::new(&storage_dir);
        async move { handle_s3_event(&s3, &bedrock, &store, event).await }
    });

    lambda_runtime::run(handler)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

async fn handle_s3_event(
    s3_client: &S3Client,
    bedrock_client: &BedrockClient,
    storage: &LocalStorage,
    event: LambdaEvent<S3Event>,
) -> std::result::Result<serde_json::Value, LambdaError> {
    let (s3_event, _context) = event.into_parts();

    for record in &s3_event.records {
        let bucket = &record.s3.bucket.name;
        let key = &record.s3.object.key;

        match process_s3_email(s3_client, bedrock_client, storage, bucket, key).await {
            Ok(manifest) => {
                tracing::info!(
                    count = manifest.items.len(),
                    bucket,
                    key,
                    "Successfully processed email"
                );
            }
            Err(e) => {
                tracing::error!(error = %e, bucket, key, "Failed to process email");
                return Err(e.to_string().into());
            }
        }
    }

    Ok(serde_json::json!({"status": "ok"}))
}
