//! SMTP email delivery service implementation powered by `lettre`.
//!
//! ### Connection Pooling & Transport Reuse
//!
//! `LettreSMTPService` maintains a single, persistent [`lettre::AsyncSmtpTransport`] instance.
//! All email dispatches reuse this underlying transport connection pool rather than opening a new TCP/TLS socket on every send operation.

use std::env;

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use secrecy::{ExposeSecret, SecretString};
use tracing::{info, warn};

use crate::{
    NotifyError,
    traits::{EmailMessage, EmailService},
};

/// Configuration options for SMTP email delivery via `lettre`.
#[derive(Clone, Debug)]
pub struct SmtpConfig {
    /// SMTP relay server hostname (e.g. `"smtp.mailgun.org"`).
    pub host: String,
    /// SMTP relay port (default: `587`).
    pub port: u16,
    /// SMTP authentication username.
    pub username: String,
    /// SMTP authentication password wrapped in [`SecretString`].
    pub password: SecretString,
    /// Default `From` sender email address (e.g. `"noreply@example.com"`).
    pub from: String,
    /// Whether to perform an active connection/handshake test during initialization (default: `true`).
    pub test_connection_on_init: bool,
}

impl SmtpConfig {
    /// Constructs an [`SmtpConfig`] with required credentials and default port 587.
    pub fn new(
        host: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        from: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port: 587,
            username: username.into(),
            password: SecretString::from(password.into()),
            from: from.into(),
            test_connection_on_init: true,
        }
    }

    /// Constructs an [`SmtpConfig`] from standard environment variables:
    /// - `SMTP_HOST` (Required)
    /// - `SMTP_PORT` (Optional, defaults to `587`)
    /// - `SMTP_USERNAME` (Required)
    /// - `SMTP_PASSWORD` (Required)
    /// - `SMTP_FROM` (Required)
    /// - `SMTP_TEST_ON_INIT` (Optional, defaults to `"true"`)
    pub fn from_env() -> Result<Self, NotifyError> {
        let host = env::var("SMTP_HOST")
            .map_err(|_| NotifyError::Configuration("SMTP_HOST must be set".into()))?;
        let port: u16 = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .map_err(|e| {
                NotifyError::Configuration(format!("SMTP_PORT must be a valid u16: {e}"))
            })?;
        let username = env::var("SMTP_USERNAME")
            .map_err(|_| NotifyError::Configuration("SMTP_USERNAME must be set".into()))?;
        let password = SecretString::from(
            env::var("SMTP_PASSWORD")
                .map_err(|_| NotifyError::Configuration("SMTP_PASSWORD must be set".into()))?,
        );
        let from = env::var("SMTP_FROM")
            .map_err(|_| NotifyError::Configuration("SMTP_FROM must be set".into()))?;

        let test_connection_on_init = env::var("SMTP_TEST_ON_INIT")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        Ok(Self {
            host,
            port,
            username,
            password,
            from,
            test_connection_on_init,
        })
    }

    /// Sets the SMTP server port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Configures whether to run a connection test during initialization.
    pub fn with_test_connection_on_init(mut self, test: bool) -> Self {
        self.test_connection_on_init = test;
        self
    }

    /// Builds a [`LettreSMTPService`] instance from this configuration.
    pub async fn build(self) -> Result<LettreSMTPService, NotifyError> {
        LettreSMTPService::from_config(self).await
    }
}

/// SMTP email delivery service implementation using `lettre`.
///
/// Wraps a persistent StartTLS-relayed [`AsyncSmtpTransport`] pool and default `from` sender email address.
/// Reuses transport connections across all send operations.
pub struct LettreSMTPService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl LettreSMTPService {
    /// Constructs a [`LettreSMTPService`] from a given [`SmtpConfig`].
    pub async fn from_config(config: SmtpConfig) -> Result<Self, NotifyError> {
        let creds = Credentials::new(
            config.username,
            config.password.expose_secret().to_string(),
        );
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)?
            .port(config.port)
            .credentials(creds)
            .build();

        if config.test_connection_on_init {
            let connected = transport.test_connection().await?;
            if !connected {
                return Err(NotifyError::Configuration(
                    "SMTP server returned negative response during connection test".into(),
                ));
            }
        }

        Ok(Self {
            transport,
            from: config.from,
        })
    }

    /// Constructs a [`LettreSMTPService`] instance by reading environment variables:
    /// - `SMTP_HOST` (e.g. `"smtp.mailgun.org"`)
    /// - `SMTP_PORT` (e.g. `587`)
    /// - `SMTP_USERNAME` (e.g. `"postmaster@mg.example.com"`)
    /// - `SMTP_PASSWORD` (sensitive, wrapped in [`SecretString`])
    /// - `SMTP_FROM` (e.g. `"noreply@example.com"`)
    ///
    /// For programmatic configuration, use [`SmtpConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Configuration`] if environment variables are missing, invalid, or if the SMTP connection test fails.
    pub async fn from_env() -> Result<Self, NotifyError> {
        let config = SmtpConfig::from_env()?;
        Self::from_config(config).await
    }

    /// Exposes a reference to the inner persistent [`AsyncSmtpTransport`].
    pub fn transport(&self) -> &AsyncSmtpTransport<Tokio1Executor> {
        &self.transport
    }
}

impl EmailService for LettreSMTPService {
    /// Sends an [`EmailMessage`] asynchronously to all listed recipients using the shared transport pool.
    ///
    /// Automatically builds a `multipart/alternative` email payload containing both HTML and plaintext bodies
    /// if `msg.text` is present; otherwise builds a `text/html` singlepart email.
    /// Attempts delivery to each recipient in `msg.to`, logging per-recipient outcomes and capturing errors.
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError`] if building the `lettre::Message` fails, if sender configuration is invalid,
    /// or if delivery to all recipients fails.
    async fn send(&self, msg: EmailMessage) -> Result<(), NotifyError> {
        let from_mailbox: Mailbox = self.from.parse()?;
        let mut errors = Vec::new();

        for recipient in &msg.to {
            let to_mailbox = match recipient.parse() {
                Ok(addr) => addr,
                Err(e) => {
                    warn!(recipient = %recipient, error = %e, "Invalid recipient email address format");
                    errors.push(format!("Invalid address '{recipient}': {e}"));
                    continue;
                }
            };

            let builder = Message::builder()
                .from(from_mailbox.clone())
                .to(to_mailbox)
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

            match self.transport.send(email).await {
                Ok(_) => {
                    info!(recipient = %recipient, subject = %msg.subject, "Email sent successfully");
                }
                Err(e) => {
                    warn!(recipient = %recipient, subject = %msg.subject, error = %e, "Failed to send email to recipient");
                    errors.push(format!("Failed to send to '{recipient}': {e}"));
                }
            }
        }

        if !errors.is_empty() && errors.len() == msg.to.len() {
            return Err(NotifyError::Internal(format!(
                "Failed to deliver email to all recipients: {}",
                errors.join("; ")
            )));
        } else if !errors.is_empty() {
            warn!(
                subject = %msg.subject,
                "Email delivered with partial failures ({}/{} failed): {}",
                errors.len(),
                msg.to.len(),
                errors.join("; ")
            );
        }

        Ok(())
    }
}
