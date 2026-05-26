mod analysis;
mod email;
mod export;
mod input;
mod lambda;
mod models;
pub mod output;
mod processor;
mod storage;
mod validate;

use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use clap::{Parser, Subcommand};
use futures::stream::{self, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};

use input::{fetch_email, resolve_sources};
use output::{Output, Verbosity};

#[derive(Parser)]
#[command(name = "parmail", about = "USPS Informed Delivery mail image processor")]
struct Cli {
    /// Increase verbosity (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true, conflicts_with = "quiet")]
    verbose: u8,
    /// Decrease verbosity (-q, -qq)
    #[arg(short, long, action = clap::ArgAction::Count, global = true, conflicts_with = "verbose")]
    quiet: u8,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as an AWS Lambda function (S3 event trigger)
    Lambda,
    /// Validate AWS credentials and Bedrock access
    Validate,
    /// Process .eml files from local paths, directories, or s3:// URIs
    Process {
        /// Directory or s3://bucket/prefix to store images and metadata
        #[arg(short, long, default_value = "./data")]
        storage_dir: String,
        /// Concurrent email processing limit
        #[arg(short, long, default_value = "10")]
        concurrency: usize,
        /// Paths to .eml files, directories, or s3://bucket/prefix URIs
        #[arg(required = true, trailing_var_arg = true)]
        paths: Vec<String>,
    },
    /// Export processed manifests to CSV
    Export {
        /// Data directory containing manifest subdirectories
        #[arg(short, long, default_value = "./data")]
        data_dir: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .without_time()
        .init();

    let cli = Cli::parse();

    let verbosity = Verbosity::from_flags(cli.verbose, cli.quiet);

    match cli.command {
        Commands::Lambda => {
            lambda::run_lambda().await?;
        }
        Commands::Validate => {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            validate::validate_aws(&bedrock_client).await?;
            eprintln!("All AWS resources validated successfully.");
        }
        Commands::Process { paths, storage_dir, concurrency } => {
            let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);

            let needs_s3 = paths.iter().any(|p| p.starts_with("s3://"))
                || storage_dir.starts_with("s3://");
            let s3_client = if needs_s3 {
                Some(S3Client::new(&config))
            } else {
                None
            };

            let storage = storage::Storage::from_uri(&storage_dir, s3_client.clone())?;
            let sources = resolve_sources(&paths, s3_client.as_ref()).await?;
            let out = Output::new(verbosity, std::io::IsTerminal::is_terminal(&std::io::stderr()), sources.len() as u64);
            let errors = AtomicU64::new(0);
            let total = sources.len() as u64;

            stream::iter(sources.iter())
                .for_each_concurrent(concurrency, |source| {
                    let bedrock = &bedrock_client;
                    let store = &storage;
                    let s3 = &s3_client;
                    let out = &out;
                    let errors = &errors;
                    async move {
                        let name = source.short_name();
                        let raw = match fetch_email(source, s3.as_ref()).await {
                            Ok(data) => data,
                            Err(e) => {
                                out.error(&format!("{source}: {e}"));
                                errors.fetch_add(1, Ordering::Relaxed);
                                return;
                            }
                        };

                        match processor::process_raw_email(bedrock, store, &name, &raw).await {
                            Ok(manifest) => {
                                out.file_done(&manifest.received_date.to_string(), &manifest.email_message_id, manifest.mail_pieces.len(), true);
                            }
                            Err(e) => {
                                out.file_done(&name, "", 0, false);
                                out.error(&format!("{source}: {e}"));
                                errors.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                })
                .await;

            out.finish(total, errors.load(Ordering::Relaxed));
        }
        Commands::Export { data_dir, output } => {
            let data_path = std::path::Path::new(&data_dir);
            let output_path = output.unwrap_or_else(|| format!("{}/parmail.csv", data_dir));
            let out_path = std::path::Path::new(&output_path);

            let rows = export::export_csv(data_path, out_path)?;
            eprintln!("Exported {rows} rows to {output_path}");
        }
    }

    Ok(())
}
