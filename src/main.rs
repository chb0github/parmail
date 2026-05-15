mod analysis;
mod email;
mod lambda;
mod models;
mod processor;
mod server;
mod storage;
mod validate;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "parmail", about = "USPS Informed Delivery mail image processor")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as an AWS Lambda function (S3 event trigger)
    Lambda,
    /// Run as an HTTP server
    Serve {
        /// Address to bind to
        #[arg(short, long, default_value = "0.0.0.0:3000")]
        addr: String,
        /// Directory to store images and metadata
        #[arg(short, long, default_value = "./data")]
        storage_dir: String,
    },
    /// Validate AWS credentials and Bedrock access
    Validate,
    /// Process local .eml file(s) directly (for testing)
    Process {
        /// Path to an .eml file or directory of .eml files (scanned recursively)
        #[arg()]
        path: String,
        /// Directory to store images and metadata
        #[arg(short, long, default_value = "./data")]
        storage_dir: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("parmail=info".parse()?),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Lambda => {
            lambda::run_lambda().await?;
        }
        Commands::Serve { addr, storage_dir } => {
            server::run_server(&addr, &storage_dir).await?;
        }
        Commands::Validate => {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            validate::validate_aws(&bedrock_client).await?;
            println!("All AWS resources validated successfully.");
        }
        Commands::Process { path, storage_dir } => {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);

            validate::validate_aws(&bedrock_client).await?;

            let storage = storage::LocalStorage::new(&storage_dir);

            let files = collect_eml_files(&path)?;
            tracing::info!(count = files.len(), "Found .eml files to process");

            for file in &files {
                tracing::info!(path = %file.display(), "Processing");
                let raw = tokio::fs::read(file).await?;
                match processor::process_raw_email(&bedrock_client, &storage, &raw).await {
                    Ok(manifest) => {
                        tracing::info!(
                            items = manifest.items.len(),
                            subject = %manifest.email_subject,
                            "Processed"
                        );
                    }
                    Err(e) => {
                        tracing::error!(path = %file.display(), error = %e, "Failed to process");
                    }
                }
            }
        }
    }

    Ok(())
}

fn collect_eml_files(path: &str) -> Result<Vec<PathBuf>> {
    let path = Path::new(path);
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        walk_dir(path, &mut files)?;
    } else {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    files.sort();
    Ok(files)
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, files)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("eml") {
            files.push(path);
        }
    }
    Ok(())
}
