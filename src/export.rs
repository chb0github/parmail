use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use arrow::array::{ArrayRef, Float32Builder, StringBuilder};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

use crate::models::EmailManifest;

fn schema() -> Schema {
    Schema::new(vec![
        Field::new("email_id", DataType::Utf8, false),
        Field::new("source_file", DataType::Utf8, false),
        Field::new("received_date", DataType::Utf8, false),
        Field::new("email_subject", DataType::Utf8, false),
        Field::new("email_message_id", DataType::Utf8, false),
        Field::new("to_name", DataType::Utf8, true),
        Field::new("to_street", DataType::Utf8, true),
        Field::new("to_city", DataType::Utf8, true),
        Field::new("to_state", DataType::Utf8, true),
        Field::new("to_zip", DataType::Utf8, true),
        Field::new("to_status", DataType::Utf8, false),
        Field::new("piece_id", DataType::Utf8, false),
        Field::new("from_name", DataType::Utf8, true),
        Field::new("from_street", DataType::Utf8, true),
        Field::new("from_city", DataType::Utf8, true),
        Field::new("from_state", DataType::Utf8, true),
        Field::new("from_zip", DataType::Utf8, true),
        Field::new("from_status", DataType::Utf8, false),
        Field::new("mail_type", DataType::Utf8, false),
        Field::new("confidence", DataType::Float32, false),
        Field::new("postmark_date", DataType::Utf8, true),
        Field::new("mailer_filename", DataType::Utf8, true),
        Field::new("mailer_text", DataType::Utf8, true),
        Field::new("content_filename", DataType::Utf8, true),
        Field::new("content_text", DataType::Utf8, true),
    ])
}

pub fn export_parquet(data_dir: &Path, output: &Path) -> Result<u64> {
    let manifests = load_manifests(data_dir)?;
    let schema = Arc::new(schema());

    let file = fs::File::create(output)
        .with_context(|| format!("Failed to create {}", output.display()))?;

    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();

    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    let mut row_count: u64 = 0;
    let batch_size = 1024;
    let mut rows = Vec::with_capacity(batch_size);

    for manifest in &manifests {
        for piece in &manifest.mail_pieces {
            rows.push((manifest, piece));
            if rows.len() >= batch_size {
                let batch = build_batch(&schema, &rows)?;
                writer.write(&batch)?;
                row_count += rows.len() as u64;
                rows.clear();
            }
        }
    }

    if !rows.is_empty() {
        let batch = build_batch(&schema, &rows)?;
        writer.write(&batch)?;
        row_count += rows.len() as u64;
    }

    writer.close()?;
    Ok(row_count)
}

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

fn build_batch(schema: &Arc<Schema>, rows: &[(&EmailManifest, &crate::models::MailPiece)]) -> Result<RecordBatch> {
    let mut email_id = StringBuilder::new();
    let mut source_file = StringBuilder::new();
    let mut received_date = StringBuilder::new();
    let mut email_subject = StringBuilder::new();
    let mut email_message_id = StringBuilder::new();
    let mut to_name = StringBuilder::new();
    let mut to_street = StringBuilder::new();
    let mut to_city = StringBuilder::new();
    let mut to_state = StringBuilder::new();
    let mut to_zip = StringBuilder::new();
    let mut to_status = StringBuilder::new();
    let mut piece_id = StringBuilder::new();
    let mut from_name = StringBuilder::new();
    let mut from_street = StringBuilder::new();
    let mut from_city = StringBuilder::new();
    let mut from_state = StringBuilder::new();
    let mut from_zip = StringBuilder::new();
    let mut from_status = StringBuilder::new();
    let mut mail_type = StringBuilder::new();
    let mut confidence = Float32Builder::new();
    let mut postmark_date = StringBuilder::new();
    let mut mailer_filename = StringBuilder::new();
    let mut mailer_text = StringBuilder::new();
    let mut content_filename = StringBuilder::new();
    let mut content_text = StringBuilder::new();

    for (manifest, piece) in rows {
        let to_addr = manifest.to_address.address.as_ref();
        let from_addr = piece.from_address.address.as_ref();

        email_id.append_value(&manifest.id);
        source_file.append_value(&manifest.source_file);
        received_date.append_value(manifest.received_date.to_string());
        email_subject.append_value(&manifest.email_subject);
        email_message_id.append_value(&manifest.email_message_id);
        append_opt(&mut to_name, to_addr.and_then(|a| a.name.as_deref()));
        append_opt(&mut to_street, to_addr.and_then(|a| a.street.as_deref()));
        append_opt(&mut to_city, to_addr.and_then(|a| a.city.as_deref()));
        append_opt(&mut to_state, to_addr.and_then(|a| a.state.as_deref()));
        append_opt(&mut to_zip, to_addr.and_then(|a| a.zip.as_deref()));
        to_status.append_value(format!("{:?}", manifest.to_address.status).to_lowercase());
        piece_id.append_value(&piece.id);
        append_opt(&mut from_name, from_addr.and_then(|a| a.name.as_deref()));
        append_opt(&mut from_street, from_addr.and_then(|a| a.street.as_deref()));
        append_opt(&mut from_city, from_addr.and_then(|a| a.city.as_deref()));
        append_opt(&mut from_state, from_addr.and_then(|a| a.state.as_deref()));
        append_opt(&mut from_zip, from_addr.and_then(|a| a.zip.as_deref()));
        from_status.append_value(format!("{:?}", piece.from_address.status).to_lowercase());
        mail_type.append_value(format!("{:?}", piece.mail_type).to_lowercase());
        confidence.append_value(piece.confidence);
        append_opt(&mut postmark_date, piece.postmark_date.map(|d| d.to_string()).as_deref());
        append_opt(&mut mailer_filename, piece.mailer.as_ref().map(|m| m.filename.as_str()));
        append_opt(&mut mailer_text, piece.mailer.as_ref().map(|m| m.full_text.as_str()));
        append_opt(&mut content_filename, piece.content.as_ref().map(|c| c.filename.as_str()));
        append_opt(&mut content_text, piece.content.as_ref().map(|c| c.full_text.as_str()));
    }

    let columns: Vec<ArrayRef> = vec![
        Arc::new(email_id.finish()),
        Arc::new(source_file.finish()),
        Arc::new(received_date.finish()),
        Arc::new(email_subject.finish()),
        Arc::new(email_message_id.finish()),
        Arc::new(to_name.finish()),
        Arc::new(to_street.finish()),
        Arc::new(to_city.finish()),
        Arc::new(to_state.finish()),
        Arc::new(to_zip.finish()),
        Arc::new(to_status.finish()),
        Arc::new(piece_id.finish()),
        Arc::new(from_name.finish()),
        Arc::new(from_street.finish()),
        Arc::new(from_city.finish()),
        Arc::new(from_state.finish()),
        Arc::new(from_zip.finish()),
        Arc::new(from_status.finish()),
        Arc::new(mail_type.finish()),
        Arc::new(confidence.finish()),
        Arc::new(postmark_date.finish()),
        Arc::new(mailer_filename.finish()),
        Arc::new(mailer_text.finish()),
        Arc::new(content_filename.finish()),
        Arc::new(content_text.finish()),
    ];

    Ok(RecordBatch::try_new(schema.clone(), columns)?)
}

fn append_opt(builder: &mut StringBuilder, val: Option<&str>) {
    match val {
        Some(s) => builder.append_value(s),
        None => builder.append_null(),
    }
}
