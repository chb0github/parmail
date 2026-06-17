use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
use crate::{LambdaError, FROM_ADDRESS, SUBJECT_PREFIX};
use aws_sdk_sesv2::Client as SesClient;

struct SeS {
    client: aws_sdk_sesv2::Client,
}

impl SeS {
    async fn new() -> Self {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Self{
            client: aws_sdk_sesv2::Client::new(&config)
        }
    }
    async fn send_email(
        &self,
        to: &str,
        body_text: &str,
    ) -> std::result::Result<(), LambdaError> {
        self.client
            .send_email()
            .from_email_address(FROM_ADDRESS)
            .destination(Destination::builder().to_addresses(to).build())
            .content(EmailContent::builder().simple(
                Message::builder()
                    .subject(
                        Content::builder().data(SUBJECT_PREFIX).charset("UTF-8").build().expect("subject")
                    )
                    .body(
                        Body::builder().text(Content::builder().data(body_text).charset("UTF-8").build().expect("body")).build()
                    )
                    .build()
            ).build())
            .send()
            .await?;
        Ok(())
    }
}


