mod analysis;
mod email;
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
        /// Paths to .eml files, directories, or s3://bucket/prefix URIs
        #[arg(required = true)]
        paths: Vec<String>,
        /// Directory to store images and metadata
        #[arg(short, long, default_value = "./data")]
        storage_dir: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
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
        Commands::Process { paths, storage_dir } => {
            let needs_s3 = paths.iter().any(|p| p.starts_with("s3://"))
                || storage_dir.starts_with("s3://");
            let s3_client = if needs_s3 {
                let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
                Some(S3Client::new(&config))
            } else {
                None
            };

            // let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            // let bedrock_client = aws_sdk_bedrockruntime::Client::new(&config);
            // validate::validate_aws(&bedrock_client).await?;

            let storage = storage::Storage::from_uri(&storage_dir, s3_client.clone())?;
            let sources = resolve_sources(&paths, s3_client.as_ref()).await?;
            let out = Output::new(verbosity, true, sources.len() as u64);
            let mut errors = 0u64;

            for source in &sources {
                let name = source.short_name();
                out.start_file(&name);

                let raw = match fetch_email(source, s3_client.as_ref()).await {
                    Ok(data) => data,
                    Err(e) => {
                        out.error(&format!("{source}: {e}"));
                        errors += 1;
                        out.file_done(&name, 0);
                        break;
                    }
                };

                match email::parse_email(&raw) {
                    Ok(parsed) => {
                        out.step(&format!("Parsed: {}", parsed.info.subject));
                        out.step(&format!("Images: {}", parsed.images.len()));

                        match storage.ensure_email_dir(&parsed.info).await {
                            Ok(dir) => {
                                for image in &parsed.images {
                                    out.step(&format!("Storing: {}", image.filename));
                                    if let Err(e) = storage.store_image(&dir, &image.data, &image.filename).await {
                                        out.error(&format!("Failed to store {}: {e}", image.filename));
                                        errors += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                out.error(&format!("Failed to create dir: {e}"));
                                errors += 1;
                            }
                        }

                        out.file_done(&parsed.info.subject, parsed.images.len());
                    }
                    Err(e) => {
                        out.error(&format!("{source}: {e}"));
                        errors += 1;
                        out.file_done(&name, 0);
                    }
                }
            }

            out.finish(sources.len() as u64, errors);
        }
    }

    Ok(())
}
