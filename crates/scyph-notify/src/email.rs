use std::env;

use lettre::{
    message::{header::ContentType, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use secrecy::{ExposeSecret, SecretString};
use tracing::info;

use crate::{
    traits::{EmailMessage, EmailService},
    NotifyError,
};

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
    /// Returns [`NotifyError`] if environment variables are missing, invalid, or if the SMTP connection test fails.
    pub async fn from_env() -> Result<Self, NotifyError> {
        let host = env::var("SMTP_HOST")
            .map_err(|_| NotifyError::Configuration("SMTP_HOST must be set".into()))?;
        let port: u16 = env::var("SMTP_PORT")
            .map_err(|_| NotifyError::Configuration("SMTP_PORT must be set".into()))?
            .parse()
            .map_err(|e| NotifyError::Configuration(format!("SMTP_PORT must be a valid u16: {e}")))?;
        let username = env::var("SMTP_USERNAME")
            .map_err(|_| NotifyError::Configuration("SMTP_USERNAME must be set".into()))?;
        let password = SecretString::from(
            env::var("SMTP_PASSWORD")
                .map_err(|_| NotifyError::Configuration("SMTP_PASSWORD must be set".into()))?,
        );
        let from = env::var("SMTP_FROM")
            .map_err(|_| NotifyError::Configuration("SMTP_FROM must be set".into()))?;

        let creds = Credentials::new(username, password.expose_secret().to_string());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?
            .port(port)
            .credentials(creds)
            .build();

        let connected = transport.test_connection().await?;

        if !connected {
            return Err(NotifyError::Configuration(
                "SMTP server returned negative response during connection test".into(),
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
    async fn send(&self, msg: EmailMessage) -> Result<(), NotifyError> {
        for recipient in &msg.to {
            let builder = Message::builder()
                .from(self.from.parse()?)
                .to(recipient.parse()?)
                .subject(&msg.subject);

            let email = if let Some(text) = &msg.text {
                builder.multipart(
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
                )?
            } else {
                builder.singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(msg.html.clone()),
                )?
            };

            self.transport.send(email).await?;
            info!(recipient = %recipient, subject = %msg.subject, "Email sent successfully");
        }
        Ok(())
    }
}
