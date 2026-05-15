use anyhow::{Context, Result};
use mail_parser::{Address as MailAddress, HeaderValue, MessageParser, MimeHeaders, PartType};
use sha2::{Digest, Sha256};

pub struct ExtractedImage {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

pub struct EmailInfo {
    pub subject: String,
    pub from: String,
    pub date: String,
    pub message_id: String,
}

impl EmailInfo {
    pub fn dir_name(&self) -> String {
        let hash = hex::encode(&Sha256::digest(self.message_id.as_bytes())[..4]);
        let slug = self
            .subject
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ')
            .collect::<String>()
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join("-")
            .to_lowercase();

        format!("{slug}-{hash}")
    }

    pub fn date_folder(&self) -> String {
        self.date
            .split('T')
            .next()
            .unwrap_or(&self.date)
            .to_string()
    }
}

pub struct ParsedEmail {
    pub info: EmailInfo,
    pub images: Vec<ExtractedImage>,
}

pub fn parse_email(raw_email: &[u8]) -> Result<ParsedEmail> {
    let message = MessageParser::default()
        .parse(raw_email)
        .context("Failed to parse email")?;

    let subject = message.subject().unwrap_or("unknown").to_string();

    let from = match message.from() {
        Some(MailAddress::List(addrs)) => addrs
            .first()
            .and_then(|a| {
                a.name
                    .as_ref()
                    .map(|n| n.to_string())
                    .or_else(|| a.address.as_ref().map(|a| a.to_string()))
            })
            .unwrap_or_else(|| "unknown".to_string()),
        Some(MailAddress::Group(groups)) => groups
            .first()
            .and_then(|g| g.name.as_ref())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    };

    let date = message
        .date()
        .map(|d| {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                d.year, d.month, d.day, d.hour, d.minute, d.second
            )
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    let message_id = message
        .message_id()
        .unwrap_or("unknown")
        .to_string();

    let mut images = Vec::new();

    for part in message.parts.iter() {
        let content_type = part.content_type().map(|ct| {
            let main = ct.ctype();
            let sub = ct.subtype().unwrap_or("octet-stream");
            format!("{main}/{sub}")
        });

        let ct = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

        if ct.starts_with("image/") {
            let filename = part
                .attachment_name()
                .or_else(|| {
                    part.content_id()
                        .map(|id| id.trim_matches(|c| c == '<' || c == '>'))
                })
                .unwrap_or("unnamed.jpg")
                .to_string();

            let data = match &part.body {
                PartType::Binary(bytes) | PartType::InlineBinary(bytes) => bytes.to_vec(),
                _ => Vec::new(),
            };

            if !data.is_empty() {
                images.push(ExtractedImage {
                    filename,
                    content_type: ct,
                    data,
                });
            }
        }
    }

    Ok(ParsedEmail {
        info: EmailInfo {
            subject,
            from,
            date,
            message_id,
        },
        images,
    })
}
