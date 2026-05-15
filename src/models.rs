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
pub struct MailMetadata {
    pub id: String,
    pub image_filename: String,
    pub image_sha256: String,
    pub from_address: Option<Address>,
    pub to_address: Option<Address>,
    pub mail_type: MailType,
    pub full_text: String,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailManifest {
    pub email_subject: String,
    pub email_from: String,
    pub email_date: String,
    pub email_message_id: String,
    pub processed_at: String,
    pub items: Vec<MailMetadata>,
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
