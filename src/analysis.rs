use anyhow::{Context, Result};
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ConversationRole, ImageBlock, ImageFormat, ImageSource, Message,
    Tool, ToolChoice, ToolConfiguration, ToolInputSchema, ToolSpecification,
    SpecificToolChoice,
};
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_smithy_types::Document;

use crate::models::{Address, MailType};

const MODEL_ID: &str = "us.anthropic.claude-haiku-4-5-20251001-v1:0";

const ANALYSIS_PROMPT: &str = "Analyze this scanned image of a piece of physical mail. \
Extract the sender address, recipient address, classify the mail type, transcribe all visible text, \
and extract the postmark date if visible. USPS often redacts the recipient address with a white rectangle — \
if you see that, set to_address fields to null.";

fn tool_schema() -> Document {
    Document::Object(
        [
            ("type".into(), Document::String("object".into())),
            ("properties".into(), Document::Object(
                [
                    ("from_address".into(), address_schema("The sender's return address")),
                    ("to_address".into(), address_schema("The recipient's delivery address")),
                    ("mail_type".into(), Document::Object(
                        [
                            ("type".into(), Document::String("string".into())),
                            ("enum".into(), Document::Array(vec![
                                Document::String("advertising".into()),
                                Document::String("political".into()),
                                Document::String("personal".into()),
                                Document::String("financial".into()),
                                Document::String("government".into()),
                                Document::String("unknown".into()),
                            ])),
                            ("description".into(), Document::String("Mail classification".into())),
                        ].into_iter().collect()
                    )),
                    ("full_text".into(), Document::Object(
                        [
                            ("type".into(), Document::String("string".into())),
                            ("description".into(), Document::String("ALL text visible on the image, transcribed exactly".into())),
                        ].into_iter().collect()
                    )),
                    ("confidence".into(), Document::Object(
                        [
                            ("type".into(), Document::String("number".into())),
                            ("description".into(), Document::String("Confidence in classification, 0.0 to 1.0".into())),
                        ].into_iter().collect()
                    )),
                    ("postmark_date".into(), Document::Object(
                        [
                            ("type".into(), Document::String("string".into())),
                            ("description".into(), Document::String("Postmark date in YYYY-MM-DD format, or null if not visible".into())),
                        ].into_iter().collect()
                    )),
                ].into_iter().collect()
            )),
            ("required".into(), Document::Array(vec![
                Document::String("from_address".into()),
                Document::String("to_address".into()),
                Document::String("mail_type".into()),
                Document::String("full_text".into()),
                Document::String("confidence".into()),
            ])),
        ].into_iter().collect()
    )
}

fn address_schema(description: &str) -> Document {
    Document::Object(
        [
            ("type".into(), Document::String("object".into())),
            ("description".into(), Document::String(description.into())),
            ("properties".into(), Document::Object(
                [
                    ("name".into(), Document::Object([("type".into(), Document::String("string".into()))].into_iter().collect())),
                    ("street".into(), Document::Object([("type".into(), Document::String("string".into()))].into_iter().collect())),
                    ("city".into(), Document::Object([("type".into(), Document::String("string".into()))].into_iter().collect())),
                    ("state".into(), Document::Object([("type".into(), Document::String("string".into()))].into_iter().collect())),
                    ("zip".into(), Document::Object([("type".into(), Document::String("string".into()))].into_iter().collect())),
                ].into_iter().collect()
            )),
        ].into_iter().collect()
    )
}

#[derive(serde::Deserialize)]
pub struct AnalysisResponse {
    pub from_address: Option<Address>,
    pub to_address: Option<Address>,
    pub mail_type: MailType,
    pub full_text: String,
    pub confidence: f32,
    pub postmark_date: Option<chrono::NaiveDate>,
}

pub async fn analyze_image(
    client: &BedrockClient,
    image_data: &[u8],
    content_type: &str,
) -> Result<AnalysisResponse> {
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
            .source(ImageSource::Bytes(
                aws_sdk_bedrockruntime::primitives::Blob::new(image_data.to_vec()),
            ))
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

    let tool_config = ToolConfiguration::builder()
        .tools(Tool::ToolSpec(
            ToolSpecification::builder()
                .name("analyze_mail")
                .description("Extract structured information from a scanned mail image")
                .input_schema(ToolInputSchema::Json(tool_schema()))
                .build()
                .context("Failed to build tool spec")?,
        ))
        .tool_choice(ToolChoice::Tool(
            SpecificToolChoice::builder()
                .name("analyze_mail")
                .build()
                .context("Failed to build tool choice")?,
        ))
        .build()
        .context("Failed to build tool config")?;

    let response = client
        .converse()
        .model_id(MODEL_ID)
        .messages(message)
        .tool_config(tool_config)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(model_id = MODEL_ID, error = ?e, "Bedrock converse failed");
            e
        })
        .context("Bedrock converse API call failed")?;

    let output = response
        .output()
        .context("No output in response")?;

    let tool_json = match output {
        aws_sdk_bedrockruntime::types::ConverseOutput::Message(msg) => {
            msg.content().iter().find_map(|block| {
                if let ContentBlock::ToolUse(tool_use) = block {
                    Some(tool_use.input().clone())
                } else {
                    None
                }
            })
        }
        _ => None,
    }
    .context("No tool use in response")?;

    let json_value = document_to_value(&tool_json);
    let parsed: AnalysisResponse = serde_json::from_value(json_value.clone()).map_err(|e| {
        tracing::error!(
            raw_response = %json_value,
            error = %e,
            "Failed to parse tool response"
        );
        anyhow::anyhow!("Failed to parse tool response: {e}")
    })?;

    Ok(parsed)
}

fn document_to_value(doc: &Document) -> serde_json::Value {
    match doc {
        Document::Object(map) => {
            let obj: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), document_to_value(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        Document::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(document_to_value).collect())
        }
        Document::Number(n) => {
            serde_json::Number::from_f64(n.to_f64_lossy())
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        Document::String(s) => serde_json::Value::String(s.clone()),
        Document::Bool(b) => serde_json::Value::Bool(*b),
        Document::Null => serde_json::Value::Null,
    }
}
