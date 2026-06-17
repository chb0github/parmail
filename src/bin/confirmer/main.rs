use anyhow::Result;
use clap::Parser;
use futures::stream::{self, StreamExt};
use lambda_runtime::{service_fn, LambdaEvent};
use parmail::ses::SeS;

mod provider;

use aws_lambda_events::event::sns::SnsEvent;
use parmail::email::parse_email;
use parmail::input::{fetch_email, EmailSource};
use parmail::models::S3Event;

type LambdaError = Box<dyn std::error::Error + Send + Sync>;

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


    let handler = service_fn(move |event: LambdaEvent<SnsEvent>| {
        async move { handle_sns_event(event).await }
    });

    lambda_runtime::run(handler)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

async fn handle_sns_event(
    event: LambdaEvent<SnsEvent>,
) -> std::result::Result<serde_json::Value, LambdaError> {
    let ses = SeS::new().await;
    let (sns_event, _context) = event.into_parts();

    let sent = stream::iter(
        sns_event.records.iter()
            .filter_map(|sns_record| serde_json::from_str::<S3Event>(&sns_record.sns.message).ok())
            .flat_map(|s3_event| s3_event.records)
    )
    .then(|record| async move {
        let bucket = &record.s3.bucket.name;
        let key = &record.s3.object.key;
        let source = EmailSource::S3 { bucket: bucket.clone(), key: key.clone() };
        let raw = fetch_email(&source).await
            .map_err(|e| tracing::warn!(error = %e, bucket, key, "Failed to fetch email"))
            .ok()?;
        parse_email(&raw)
            .map_err(|e| tracing::warn!(error = %e, bucket, key, "Failed to parse email"))
            .ok()
    })
    .filter_map(|opt| async { opt })
    .filter(|parsed| std::future::ready(provider::is_forwarding_request(parsed)))
    .filter_map(|parsed| async move {
        let (name, fwd_provider) = provider::get_forwarding_provider(&parsed)?;
        let confirmation = (fwd_provider.extract)(&parsed)?;
        tracing::info!(provider = name, originator = confirmation.originator.as_str(), "Detected forwarding confirmation");
        let body = fwd_provider.render(name, &confirmation);
        Some((name, confirmation.originator, body))
    })
    .then(|(name, originator, body)| {
        let ses = &ses;
        async move {
            let result = ses.send_email(&originator, SUBJECT_PREFIX, &body).await;
            tracing::info!(provider = name, originator = originator.as_str(), "Confirmation email sent");
            result
        }
    })
    .fold(0u64, |count, _| async move { count + 1 })
    .await;

    Ok(serde_json::json!({"status": "ok", "sent": sent}))
}



/// Local dry-run mode: parse a file and print what would be sent.
async fn run_local(path: &str) -> Result<()> {
    let raw_email = std::fs::read(path)?;
    let parsed = parse_email(&raw_email)?;

    if !provider::is_forwarding_request(&parsed) {
        println!("Not a forwarding confirmation email.");
        println!("  From: {} <{}>", parsed.info.from, parsed.info.from_address);
        println!("  Subject: {}", parsed.info.subject);
        return Ok(());
    }

    let (name, fwd_provider) = provider::get_forwarding_provider(&parsed)
        .expect("is_forwarding_request was true but no provider matched");
    let confirmation = (fwd_provider.extract)(&parsed)
        .expect("provider matched but extraction failed");

    println!("=== Detected Forwarding Confirmation ===");
    println!("Provider:    {}", name);
    println!("Originator:  {}", confirmation.originator);
    println!("Confirm URL: {}", confirmation.confirm_url);
    println!();
    println!("=== Email that would be sent ===");
    println!("From: {}", parmail::ses::FROM_ADDRESS);
    println!("To:   {}", confirmation.originator);
    println!("Subject: {SUBJECT_PREFIX}");
    println!();
    println!("{}", fwd_provider.render(name, &confirmation));

    Ok(())
}
