use anyhow::Result;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_s3::Client as S3Client;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use std::sync::Arc;

use crate::models::{EmailManifest, S3Event};
use crate::processor::process_s3_email;
use crate::storage::LocalStorage;

struct AppState {
    s3_client: S3Client,
    bedrock_client: BedrockClient,
    storage: LocalStorage,
}

pub async fn run_server(addr: &str, storage_dir: &str) -> Result<()> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3_client = S3Client::new(&config);
    let bedrock_client = BedrockClient::new(&config);
    let storage = LocalStorage::new(storage_dir);

    let state = Arc::new(AppState {
        s3_client,
        bedrock_client,
        storage,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/process", post(process_email))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(addr, "Server listening");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn process_email(
    State(state): State<Arc<AppState>>,
    Json(event): Json<S3Event>,
) -> Result<Json<Vec<EmailManifest>>, (StatusCode, String)> {
    let mut manifests = Vec::new();

    for record in &event.records {
        let bucket = &record.s3.bucket.name;
        let key = &record.s3.object.key;

        match process_s3_email(
            &state.s3_client,
            &state.bedrock_client,
            &state.storage,
            bucket,
            key,
        )
        .await
        {
            Ok(manifest) => manifests.push(manifest),
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to process {key}: {e}"),
                ));
            }
        }
    }

    Ok(Json(manifests))
}
