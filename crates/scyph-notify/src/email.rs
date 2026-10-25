//! SMTP email delivery service implementation powered by `lettre`.
//!
//! ### Connection Pooling & Transport Reuse
//!
//! `LettreSMTPService` maintains a single, persistent [`lettre::AsyncSmtpTransport`] instance.
//! All email dispatches reuse this underlying transport connection pool rather than opening a new TCP/TLS socket on every send operation.

use std::{env, time::Duration};

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

/// SMTP email delivery service implementation using `lettre`.
///
/// Wraps a persistent StartTLS-relayed [`AsyncSmtpTransport`] pool and default `from` sender email address.
/// Reuses transport connections across all send operations.
pub struct LettreSMTPService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl LettreSMTPService {
    /// Constructs a [`LettreSMTPService`] instance by reading environment variables:
    /// - `SMTP_HOST` (e.g. `"smtp.mailgun.org"`)
    /// - `SMTP_PORT` (e.g. `587`)
    /// - `SMTP_USERNAME` (e.g. `"postmaster@mg.example.com"`)
    /// - `SMTP_PASSWORD` (sensitive, wrapped in [`SecretString`])
    /// - `SMTP_FROM` (e.g. `"noreply@example.com"`)
    ///
    /// Performs an active SMTP handshake test during initialization to verify network reachability and authentication credentials.
    /// The handshake is bounded at 5s so a relay that accepts TCP and stalls cannot hold boot
    /// (lettre's own timeout is 60s per command).
    ///
    /// # Errors
    ///
    /// Returns [`NotifyError::Configuration`] if environment variables are missing, invalid, if the
    /// relay does not answer within 5s, or if the SMTP connection test fails.
    pub async fn from_env() -> Result<Self, NotifyError> {
        let host = env::var("SMTP_HOST")
            .map_err(|_| NotifyError::Configuration("SMTP_HOST must be set".into()))?;
        let port: u16 = env::var("SMTP_PORT")
            .map_err(|_| NotifyError::Configuration("SMTP_PORT must be set".into()))?
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

        let creds = Credentials::new(username, password.expose_secret().to_string());
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?
            .port(port)
            .credentials(creds)
            .build();

        // Bounded like the health probe: lettre's only timeout is 60s per command, so a
        // relay that accepts TCP and then stalls mid-handshake would hold boot for a minute.
        let connected =
            match tokio::time::timeout(Duration::from_secs(5), transport.test_connection()).await {
                Ok(Ok(connected)) => connected,
                Ok(Err(e)) => return Err(e.into()),
                Err(_) => {
                    return Err(NotifyError::Configuration(format!(
                        "SMTP server at {host}:{port} did not answer within 5s"
                    )));
                }
            };

        if !connected {
            return Err(NotifyError::Configuration(
                "SMTP server returned negative response during connection test".into(),
            ));
        }

        Ok(Self { transport, from })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[cfg(feature = "health")]
    use scyph_health::{HealthRegistry, Status};

    // shortcut: this helper is duplicated in scyph-storage/tests/common and scyph-db/tests/common;
    // merging needs a shared test crate (a layer), so it stays inlined here until that exists.
    /// Binds a loopback port and accepts connections forever without ever reading or
    /// writing, so the client's handshake hangs — what a stalled relay looks like. The
    /// sockets are held open on purpose: dropping them would close the connection and
    /// produce a fast error instead of a hang.
    async fn spawn_hung_server() -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((sock, _)) = listener.accept().await {
                held.push(sock);
            }
        });
        addr
    }

    /// The boot handshake must fail fast: lettre's only timeout is 60s per command, so
    /// without a bound a stalled relay holds startup open for a minute.
    #[tokio::test]
    async fn from_env_handshake_is_bounded_when_relay_stalls() {
        let addr = spawn_hung_server().await;

        // Safe: this test binary never reads or writes SMTP_* anywhere else.
        unsafe {
            env::set_var("SMTP_HOST", "127.0.0.1");
            env::set_var("SMTP_PORT", addr.port().to_string());
            env::set_var("SMTP_USERNAME", "user");
            env::set_var("SMTP_PASSWORD", "secret");
            env::set_var("SMTP_FROM", "boot-probe@example.com");
        }

        let err = tokio::time::timeout(Duration::from_secs(10), LettreSMTPService::from_env())
            .await
            .expect("boot must not hang on a stalled relay")
            .err()
            .expect("a stalled relay must fail the boot check");
        assert!(
            err.to_string().contains("did not answer within 5s"),
            "the boot check must be bounded by the 5s timeout: {err}"
        );
    }

    /// A relay that accepts TCP and then stops answering must cost one bounded probe:
    /// lettre's only timeout is 60s per command, which would stall a sequential probe
    /// loop (and `/readyz`'s view of the other dependencies) for minutes.
    #[cfg(feature = "health")]
    #[tokio::test]
    async fn health_probe_is_bounded_when_relay_stalls() {
        let addr = spawn_hung_server().await;

        let service = LettreSMTPService {
            transport: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
                .port(addr.port())
                .build(),
            from: "health-probe@example.com".to_string(),
        };
        let registry = HealthRegistry::new();

        tokio::time::timeout(Duration::from_secs(10), service.check_health(&registry))
            .await
            .expect("a stalled relay must cost one bounded probe, not hang the loop");
        let snap = registry.snapshot().await.get("smtp_email").unwrap().clone();
        assert!(matches!(snap.status, Status::Sick));
        assert!(
            snap.details
                .as_deref()
                .unwrap_or_default()
                .contains("timed out after 5s"),
            "the probe must be bounded by the 5s timeout, not by another error: {:?}",
            snap.details
        );
    }
}
