use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::models::EmailManifest;

pub fn export_csv(data_dir: &Path, output: &Path) -> Result<u64> {
    let manifests = load_manifests(data_dir)?;

    let mut wtr = csv::Writer::from_path(output)
        .with_context(|| format!("Failed to create {}", output.display()))?;

    wtr.write_record([
        "email_id", "source_file", "received_date", "email_subject", "email_message_id",
        "to_name", "to_street", "to_city", "to_state", "to_zip", "to_status",
        "piece_id", "from_name", "from_street", "from_city", "from_state", "from_zip", "from_status",
        "mail_type", "confidence", "postmark_date",
        "mailer_filename", "mailer_text", "content_filename", "content_text",
    ])?;

    let mut row_count: u64 = 0;
    for manifest in &manifests {
        for piece in &manifest.mail_pieces {
            let to_addr = manifest.to_address.address.as_ref();
            let from_addr = piece.from_address.address.as_ref();

            wtr.write_record([
                &manifest.id,
                &manifest.source_file,
                &manifest.received_date.to_string(),
                &manifest.email_subject,
                &manifest.email_message_id,
                &opt_str(to_addr.and_then(|a| a.name.as_deref())),
                &opt_str(to_addr.and_then(|a| a.street.as_deref())),
                &opt_str(to_addr.and_then(|a| a.city.as_deref())),
                &opt_str(to_addr.and_then(|a| a.state.as_deref())),
                &opt_str(to_addr.and_then(|a| a.zip.as_deref())),
                &format!("{:?}", manifest.to_address.status).to_lowercase(),
                &piece.id,
                &opt_str(from_addr.and_then(|a| a.name.as_deref())),
                &opt_str(from_addr.and_then(|a| a.street.as_deref())),
                &opt_str(from_addr.and_then(|a| a.city.as_deref())),
                &opt_str(from_addr.and_then(|a| a.state.as_deref())),
                &opt_str(from_addr.and_then(|a| a.zip.as_deref())),
                &format!("{:?}", piece.from_address.status).to_lowercase(),
                &format!("{:?}", piece.mail_type).to_lowercase(),
                &piece.confidence.to_string(),
                &opt_str(piece.postmark_date.map(|d| d.to_string()).as_deref()),
                &opt_str(piece.mailer.as_ref().map(|m| m.filename.as_str())),
                &opt_str(piece.mailer.as_ref().map(|m| m.full_text.as_str())),
                &opt_str(piece.content.as_ref().map(|c| c.filename.as_str())),
                &opt_str(piece.content.as_ref().map(|c| c.full_text.as_str())),
            ])?;
            row_count += 1;
        }
    }

    wtr.flush()?;
    Ok(row_count)
}

fn opt_str(s: Option<&str>) -> String {
    s.unwrap_or("").to_string()
}

fn load_manifests(data_dir: &Path) -> Result<Vec<EmailManifest>> {
    let mut manifests = Vec::new();
    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let manifest_path = entry.path().join("manifest.json");
        if manifest_path.exists() {
            let content = fs::read_to_string(&manifest_path)?;
            match serde_json::from_str::<EmailManifest>(&content) {
                Ok(m) => manifests.push(m),
                Err(e) => {
                    tracing::warn!(path = %manifest_path.display(), error = %e, "Skipping invalid manifest");
                }
            }
        }
    }
    Ok(manifests)
}
