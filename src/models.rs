use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailType {
    Advertising,
    Political,
    Personal,
    Financial,
    Government,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub name: Option<String>,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressStatus {
    Resolved,
    Redacted,
    Unreadable,
    NotAnalyzed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressField {
    pub address: Option<Address>,
    pub status: AddressStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentHash {
    pub value: String,
    #[serde(rename = "type")]
    pub hash_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailImage {
    pub filename: String,
    pub hash: ContentHash,
    pub full_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailPiece {
    pub id: String,
    pub from_address: AddressField,
    pub mail_type: MailType,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postmark_date: Option<NaiveDate>,
    pub mailer: Option<MailImage>,
    pub content: Option<MailImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailManifest {
    pub id: String,
    pub source_file: String,
    pub email_subject: String,
    pub email_from: String,
    pub email_date: String,
    pub received_date: NaiveDate,
    pub email_message_id: String,
    pub processed_at: String,
    pub to_address: AddressField,
    pub mail_pieces: Vec<MailPiece>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3EventRecord {
    pub s3: S3Entity,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3Entity {
    pub bucket: S3Bucket,
    pub object: S3Object,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3Bucket {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3Object {
    pub key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3Event {
    #[serde(rename = "Records")]
    pub records: Vec<S3EventRecord>,
}
