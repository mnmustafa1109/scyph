use std::env;

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    address::AddressError,
    message::{MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use scyph_core::AppError;
use secrecy::{ExposeSecret, SecretString};
use tracing::info;

use crate::traits::{EmailMessage, EmailService};

/// SMTP email delivery service implementation using `lettre`.
pub struct LettreSMTPService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl LettreSMTPService {
    /// Constructs a [`LettreSMTPService`] from environment variables (`SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_FROM`).
    ///
    /// # Panics
    ///
    /// Panics if any required environment variable is missing or if `SMTP_PORT` is invalid.
    pub fn from_env() -> Self {
        let host = env::var("SMTP_HOST").expect("SMTP_HOST must be set");
        let port: u16 = env::var("SMTP_PORT")
            .expect("SMTP_PORT must be set")
            .parse()
            .expect("SMTP_PORT must be a valid u16");
        let username = env::var("SMTP_USERNAME").expect("SMTP_USERNAME must be set");
        let password =
            SecretString::from(env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD must be set"));
        let from = env::var("SMTP_FROM").expect("SMTP_FROM must be set");

        let creds = Credentials::new(username, password.expose_secret().to_string());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .expect("Failed to create SMTP transport")
            .port(port)
            .credentials(creds)
            .build();

        Self { transport, from }
    }
}

impl EmailService for LettreSMTPService {
    async fn send(&self, msg: EmailMessage) -> Result<(), AppError> {
        for recipient in &msg.to {
            let builder =
                Message::builder()
                    .from(self.from.parse().map_err(|e: AddressError| {
                        AppError::internal_from(e, "parse from address")
                    })?)
                    .to(recipient.parse().map_err(|e: AddressError| {
                        AppError::internal_from(e, "parse to address")
                    })?)
                    .subject(&msg.subject);

            let email = if let Some(text) = &msg.text {
                builder
                    .multipart(
                        MultiPart::alternative()
                            .singlepart(
                                SinglePart::builder()
                                    .header(ContentType::TEXT_PLAIN)
                                    .body(text.clone()),
                            )
                            .singlepart(
                                SinglePart::builder()
                                    .header(ContentType::TEXT_HTML)
                                    .body(msg.html.clone()),
                            ),
                    )
                    .map_err(|e| AppError::internal_from(e, "build multipart email message"))?
            } else {
                builder
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(msg.html.clone()),
                    )
                    .map_err(|e| AppError::internal_from(e, "build singlepart email message"))?
            };

            self.transport
                .send(email)
                .await
                .map_err(|e| AppError::internal_from(e, format!("send email to {}", recipient)))?;
            info!(recipient = %recipient, subject = %msg.subject, "Email sent successfully");
        }
        Ok(())
    }
}
