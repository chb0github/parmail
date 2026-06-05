use anyhow::{Context, Result};
use indexmap::IndexMap;
use mail_parser::{Address as MailAddress, MessageParser, MimeHeaders, PartType};
use xxhash_rust::xxh3::xxh3_64;

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
    pub fn id(&self) -> String {
        format!("{:016x}", xxh3_64(self.message_id.as_bytes()))
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

    let from = extract_from(&message);

    let date = extract_date(&message);

    let message_id = message
        .message_id()
        .unwrap_or("unknown")
        .to_string();

    let images: Vec<ExtractedImage> = message
        .parts
        .iter()
        .filter(|part| extract_content_type(part).starts_with("image/"))
        .filter_map(|part| {
            let data = match &part.body {
                PartType::Binary(bytes) | PartType::InlineBinary(bytes) => bytes.to_vec(),
                _ => return None,
            };
            if data.is_empty() {
                return None;
            }
            Some(ExtractedImage {
                filename: extract_filename(part),
                content_type: extract_content_type(part),
                data,
            })
        })
        .collect();

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

fn extract_date(message: &mail_parser::Message) -> String {
    message
        .date()
        .map(|d| {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                d.year, d.month, d.day, d.hour, d.minute, d.second
            )
        })
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339())
}

fn extract_from(message: &mail_parser::Message) -> String {
    match message.from() {
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
    }
}

fn extract_content_type(part: &mail_parser::MessagePart) -> String {
    part.content_type()
        .map(|ct| {
            let main = ct.ctype();
            let sub = ct.subtype().unwrap_or("octet-stream");
            format!("{main}/{sub}")
        })
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

fn extract_filename(part: &mail_parser::MessagePart) -> String {
    part.attachment_name()
        .or_else(|| {
            part.content_id()
                .map(|id| id.trim_matches(|c| c == '<' || c == '>'))
        })
        .unwrap_or("unnamed.jpg")
        .to_string()
}

pub fn is_content_image(filename: &str) -> bool {
    filename.starts_with("ra_0_") || filename.starts_with("content-")
}

pub fn extract_piece_id(filename: &str) -> String {
    let stem = filename.strip_suffix(".jpg")
        .or_else(|| filename.strip_suffix(".jpeg"))
        .or_else(|| filename.strip_suffix(".png"))
        .unwrap_or(filename);

    if let Some(id) = stem.strip_prefix("mailer-") {
        return id.to_string();
    }
    if let Some(id) = stem.strip_prefix("content-") {
        return id.to_string();
    }
    if let Some(id) = stem.strip_prefix("ra_0_") {
        return id.to_string();
    }

    stem.to_string()
}

pub fn group_images_by_piece(images: Vec<ExtractedImage>) -> IndexMap<String, Vec<ExtractedImage>> {
    let mut groups: IndexMap<String, Vec<ExtractedImage>> = IndexMap::new();
    for image in images {
        let piece_id = extract_piece_id(&image.filename);
        groups.entry(piece_id).or_default().push(image);
    }
    groups
}

/// Extract mailer and content images for a single mail piece
/// Returns (mailer_bytes, content_bytes) where either can be None
pub fn get_images(images: &[ExtractedImage]) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let mut mailer: Option<Vec<u8>> = None;
    let mut content: Option<Vec<u8>> = None;

    for image in images {
        if is_content_image(&image.filename) {
            if content.is_none() {
                content = Some(image.data.clone());
            }
        } else {
            if mailer.is_none() {
                mailer = Some(image.data.clone());
            }
        }
    }

    (mailer, content)
}
