use anyhow::{Context, Result};
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, ImageBlock, ImageFormat, ImageSource, Message,
};
use aws_sdk_bedrockruntime::Client as BedrockClient;
use base64::Engine;

use crate::models::{Address, MailType};

const MODEL_ID: &str = "anthropic.claude-sonnet-4-20250514";

const ANALYSIS_PROMPT: &str = r#"You are analyzing a scanned image of a piece of physical mail (letter/envelope). Extract the following information:

1. **from_address**: The sender's address (name, street, city, state, zip). Use null for fields you cannot read.
2. **to_address**: The recipient's address (name, street, city, state, zip). Use null for fields you cannot read.
3. **mail_type**: Classify as one of: "advertising", "political", "personal", "financial", "government", "unknown"
4. **full_text**: ALL text visible on the mail piece, transcribed exactly as shown.
5. **confidence**: Your confidence in the classification (0.0 to 1.0).

Respond ONLY with valid JSON in this exact format:
{
  "from_address": {"name": "...", "street": "...", "city": "...", "state": "...", "zip": "..."},
  "to_address": {"name": "...", "street": "...", "city": "...", "state": "...", "zip": "..."},
  "mail_type": "...",
  "full_text": "...",
  "confidence": 0.0
}
"#;

#[derive(serde::Deserialize)]
struct AnalysisResponse {
    from_address: Option<Address>,
    to_address: Option<Address>,
    mail_type: MailType,
    full_text: String,
    confidence: f32,
}

pub async fn analyze_image(
    client: &BedrockClient,
    image_data: &[u8],
    content_type: &str,
) -> Result<(Option<Address>, Option<Address>, MailType, String, f32)> {
    let format = match content_type {
        "image/jpeg" | "image/jpg" => ImageFormat::Jpeg,
        "image/png" => ImageFormat::Png,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::Webp,
        _ => ImageFormat::Jpeg,
    };

    let image_block = ContentBlock::Image(
        ImageBlock::builder()
            .format(format)
            .source(
                ImageSource::Bytes(
                    aws_sdk_bedrockruntime::primitives::Blob::new(image_data.to_vec()),
                ),
            )
            .build()
            .context("Failed to build image block")?,
    );

    let text_block = ContentBlock::Text(ANALYSIS_PROMPT.to_string());

    let message = Message::builder()
        .role(ConversationRole::User)
        .content(image_block)
        .content(text_block)
        .build()
        .context("Failed to build message")?;

    let response = client
        .converse()
        .model_id(MODEL_ID)
        .messages(message)
        .send()
        .await
        .context("Bedrock converse API call failed")?;

    let output = response
        .output()
        .context("No output in response")?;

    let response_text = match output {
        aws_sdk_bedrockruntime::types::ConverseOutput::Message(msg) => {
            msg.content().iter().find_map(|block| {
                if let ContentBlock::Text(text) = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
        }
        _ => None,
    }
    .context("No text in response")?;

    let cleaned = response_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: AnalysisResponse =
        serde_json::from_str(cleaned).context("Failed to parse Claude response as JSON")?;

    Ok((
        parsed.from_address,
        parsed.to_address,
        parsed.mail_type,
        parsed.full_text,
        parsed.confidence,
    ))
}

pub fn _encode_image_base64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}
