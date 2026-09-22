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
    /// Constructs a [`LettreSMTPService`] from environment variables (`SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_FROM`),
    /// performing an active SMTP handshake test to verify server reachability and authentication credentials.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] if environment variables are missing, invalid, or if the SMTP connection test fails.
    pub async fn from_env() -> Result<Self, AppError> {
        let host = env::var("SMTP_HOST")
            .map_err(|e| AppError::internal_from(e, "SMTP_HOST must be set"))?;
        let port: u16 = env::var("SMTP_PORT")
            .map_err(|e| AppError::internal_from(e, "SMTP_PORT must be set"))?
            .parse()
            .map_err(|e| AppError::internal_from(e, "SMTP_PORT must be a valid u16"))?;
        let username = env::var("SMTP_USERNAME")
            .map_err(|e| AppError::internal_from(e, "SMTP_USERNAME must be set"))?;
        let password = SecretString::from(
            env::var("SMTP_PASSWORD")
                .map_err(|e| AppError::internal_from(e, "SMTP_PASSWORD must be set"))?,
        );
        let from = env::var("SMTP_FROM")
            .map_err(|e| AppError::internal_from(e, "SMTP_FROM must be set"))?;

        let creds = Credentials::new(username, password.expose_secret().to_string());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)
            .map_err(|e| AppError::internal_from(e, "Failed to create SMTP transport"))?
            .port(port)
            .credentials(creds)
            .build();

        let connected = transport
            .test_connection()
            .await
            .map_err(|e| AppError::internal_from(e, "SMTP connection test failed"))?;

        if !connected {
            return Err(AppError::internal(
                "SMTP server returned negative response during connection test",
            ));
        }

        Ok(Self { transport, from })
    }

    /// Exposes a reference to the inner [`AsyncSmtpTransport`].
    pub fn transport(&self) -> &AsyncSmtpTransport<Tokio1Executor> {
        &self.transport
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
