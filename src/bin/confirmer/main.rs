use anyhow::Result;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sesv2::Client as SesClient;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
use clap::Parser;
use lambda_runtime::{service_fn, LambdaEvent};

mod template;

use parmail::email::parse_email;
use parmail::models::S3Event;

type LambdaError = Box<dyn std::error::Error + Send + Sync>;

const FROM_ADDRESS: &str = "noreply@parmail.thetaone.io";
const SUBJECT_PREFIX: &str = "Parmail Forwarding Confirmation - Action Required";

#[derive(Parser)]
#[command(name = "parmail-confirmer", about = "Forwarding confirmation Lambda")]
enum Cli {
    /// Run as AWS Lambda (default when deployed)
    Lambda,
    /// Process a local email file (dry-run mode, prints instead of sending)
    Process {
        /// Path to a raw email file
        path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("parmail_confirmer=info".parse().unwrap()),
        )
        .json()
        .init();

    let cli = Cli::parse();
    match cli {
        Cli::Lambda => run_lambda().await,
        Cli::Process { path } => run_local(&path).await,
    }
}

async fn run_lambda() -> Result<()> {
    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let s3_client = S3Client::new(&config);
    let ses_client = SesClient::new(&config);

    let handler = service_fn(move |event: LambdaEvent<S3Event>| {
        let s3 = s3_client.clone();
        let ses = ses_client.clone();
        async move { handle_s3_event(&s3, &ses, event).await }
    });

    lambda_runtime::run(handler)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

async fn handle_s3_event(
    s3_client: &S3Client,
    ses_client: &SesClient,
    event: LambdaEvent<S3Event>,
) -> std::result::Result<serde_json::Value, LambdaError> {
    let (s3_event, _context) = event.into_parts();

    for record in &s3_event.records {
        let bucket = &record.s3.bucket.name;
        let key = &record.s3.object.key;

        let raw_email = fetch_from_s3(s3_client, bucket, key).await?;
        let parsed = match parse_email(&raw_email) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, bucket, key, "Failed to parse email, skipping");
                continue;
            }
        };

        let (name, provider, confirmation) = match template::get_forward_confirm(&parsed) {
            Some(result) => result,
            None => {
                tracing::info!(bucket, key, "Not a forwarding confirmation, skipping");
                continue;
            }
        };

        tracing::info!(
            provider = name,
            originator = confirmation.originator.as_str(),
            bucket,
            key,
            "Detected forwarding confirmation"
        );

        let body = provider.render(name, &confirmation);
        send_confirmation(ses_client, &confirmation.originator, &body).await?;

        tracing::info!(
            originator = confirmation.originator.as_str(),
            "Confirmation email sent successfully"
        );
    }

    Ok(serde_json::json!({"status": "ok"}))
}

async fn fetch_from_s3(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
) -> std::result::Result<Vec<u8>, LambdaError> {
    let resp = s3_client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    let bytes = resp.body.collect().await?.into_bytes().to_vec();
    Ok(bytes)
}

async fn send_confirmation(
    ses_client: &SesClient,
    to: &str,
    body_text: &str,
) -> std::result::Result<(), LambdaError> {
    let destination = Destination::builder()
        .to_addresses(to)
        .build();

    let subject = Content::builder()
        .data(SUBJECT_PREFIX)
        .charset("UTF-8")
        .build()
        .expect("subject content");

    let body_content = Content::builder()
        .data(body_text)
        .charset("UTF-8")
        .build()
        .expect("body content");

    let body = Body::builder().text(body_content).build();

    let message = Message::builder()
        .subject(subject)
        .body(body)
        .build();

    let email_content = EmailContent::builder().simple(message).build();

    ses_client
        .send_email()
        .from_email_address(FROM_ADDRESS)
        .destination(destination)
        .content(email_content)
        .send()
        .await?;

    Ok(())
}

/// Local dry-run mode: parse a file and print what would be sent.
async fn run_local(path: &str) -> Result<()> {
    let raw_email = std::fs::read(path)?;
    let parsed = parse_email(&raw_email)?;

    let (name, provider, confirmation) = match template::get_forward_confirm(&parsed) {
        Some(result) => result,
        None => {
            println!("Not a forwarding confirmation email.");
            println!("  From: {} <{}>", parsed.info.from, parsed.info.from_address);
            println!("  Subject: {}", parsed.info.subject);
            return Ok(());
        }
    };

    println!("=== Detected Forwarding Confirmation ===");
    println!("Provider:    {}", name);
    println!("Originator:  {}", confirmation.originator);
    println!("Confirm URL: {}", confirmation.confirm_url);
    println!();
    println!("=== Email that would be sent ===");
    println!("From: {FROM_ADDRESS}");
    println!("To:   {}", confirmation.originator);
    println!("Subject: {SUBJECT_PREFIX}");
    println!();
    println!("{}", provider.render(name, &confirmation));

    Ok(())
}
