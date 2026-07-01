//! Pluggable mail-transport interface for the email magic-link flow.
//!
//! Phase 2.2 of `book/src/roadmap/accounts-and-login.md`. The default
//! [`LogMailer`] just writes the magic-link to stderr — invaluable in
//! dev / CI / self-host without an SMTP server. Production deploys
//! enable the `mailer-smtp` feature and use [`SmtpMailer`] (TODO; the
//! shape is reserved here so the trait surface is stable).


use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MailerError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("invalid recipient: {0}")]
    Recipient(String),
    #[error("backend not configured (build with --features mailer-smtp)")]
    NotConfigured,
}

/// Mail transport. The lobby owns one `Arc<dyn Mailer>` and hands it
/// to the email-auth handler.
#[async_trait]
pub trait Mailer: Send + Sync {
    /// Deliver a magic-link sign-in email to `email`. The `link` is the
    /// fully-rendered URL the user clicks (e.g.
    /// `https://lobby.example/api/v1/auth/email/verify?token=…`).
    async fn send_magic_link(&self, email: &str, link: &str) -> Result<(), MailerError>;

    /// Generic send. Reserved for invite emails / verification flows —
    /// default impl forwards to [`Mailer::send_magic_link`] when the
    /// subject contains "magic link", otherwise errors out so backends
    /// that don't support generic mail fail loudly.
    async fn send_raw(
        &self,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), MailerError> {
        let _ = (to, subject, body);
        Err(MailerError::NotConfigured)
    }
}

// ───────────────────────────── LogMailer ─────────────────────────────────────

/// Default mailer for dev: writes the magic link to stderr in a format
/// that's grep-friendly and copy/pasteable.
pub struct LogMailer;

#[async_trait]
impl Mailer for LogMailer {
    async fn send_magic_link(&self, email: &str, link: &str) -> Result<(), MailerError> {
        eprintln!("[magic-link] to={email} link={link}");
        Ok(())
    }

    async fn send_raw(&self, to: &str, subject: &str, body: &str) -> Result<(), MailerError> {
        eprintln!("[mail] to={to} subject={subject:?}");
        for line in body.lines() {
            eprintln!("[mail]   {line}");
        }
        Ok(())
    }
}

// ───────────────────────────── SmtpMailer (real) ──────────────────────────────

/// Static config for an SMTP-backed [`Mailer`]. Built from the
/// `SMTP_*` env vars at lobby boot; pass directly via `SmtpMailer::new`
/// when wiring from a CLI.
#[cfg(feature = "mailer-smtp")]
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

#[cfg(feature = "mailer-smtp")]
impl SmtpConfig {
    /// Pull config from the canonical `SMTP_HOST` / `SMTP_PORT` /
    /// `SMTP_USER` / `SMTP_PASS` / `SMTP_FROM` env vars. Returns
    /// `None` when any required field is missing or empty so the
    /// caller can fall back to [`LogMailer`].
    pub fn from_env() -> Option<Self> {
        let host = std::env::var("SMTP_HOST").ok().filter(|s| !s.is_empty())?;
        let username = std::env::var("SMTP_USER").ok().filter(|s| !s.is_empty())?;
        let password = std::env::var("SMTP_PASS").ok().filter(|s| !s.is_empty())?;
        let from = std::env::var("SMTP_FROM").ok().filter(|s| !s.is_empty())?;
        let port = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(465);
        Some(Self { host, port, username, password, from })
    }
}

/// SMTP-backed mailer using [`lettre`]'s async tokio + rustls
/// transport. Lazily builds the transport once at construction; each
/// `send_*` call reuses the connection pool lettre maintains
/// internally.
#[cfg(feature = "mailer-smtp")]
pub struct SmtpMailer {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    from: lettre::message::Mailbox,
}

#[cfg(feature = "mailer-smtp")]
impl SmtpMailer {
    pub fn new(cfg: SmtpConfig) -> Result<Self, MailerError> {
        let creds = lettre::transport::smtp::authentication::Credentials::new(
            cfg.username,
            cfg.password,
        );
        let transport =
            lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&cfg.host)
                .map_err(|e| MailerError::Transport(e.to_string()))?
                .credentials(creds)
                .port(cfg.port)
                .build();
        let from: lettre::message::Mailbox = cfg
            .from
            .parse()
            .map_err(|e: lettre::address::AddressError| {
                MailerError::Recipient(format!("SMTP_FROM: {e}"))
            })?;
        Ok(Self { transport, from })
    }
}

#[cfg(feature = "mailer-smtp")]
#[async_trait]
impl Mailer for SmtpMailer {
    async fn send_magic_link(&self, email: &str, link: &str) -> Result<(), MailerError> {
        let to: lettre::message::Mailbox = email
            .parse()
            .map_err(|e: lettre::address::AddressError| {
                MailerError::Recipient(format!("{email}: {e}"))
            })?;
        let body = format!(
            "Hi,\n\n\
             Click the link below to sign in to Open4X (valid for 15 minutes):\n\n\
             {link}\n\n\
             If you didn't request this, ignore this email.\n",
        );
        let msg = lettre::Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject("Sign in to Open4X")
            .body(body)
            .map_err(|e| MailerError::Transport(e.to_string()))?;
        use lettre::AsyncTransport as _;
        self.transport
            .send(msg)
            .await
            .map_err(|e| MailerError::Transport(e.to_string()))?;
        Ok(())
    }

    async fn send_raw(&self, to: &str, subject: &str, body: &str) -> Result<(), MailerError> {
        let to: lettre::message::Mailbox = to
            .parse()
            .map_err(|e: lettre::address::AddressError| MailerError::Recipient(e.to_string()))?;
        let msg = lettre::Message::builder()
            .from(self.from.clone())
            .to(to)
            .subject(subject.to_string())
            .body(body.to_string())
            .map_err(|e| MailerError::Transport(e.to_string()))?;
        use lettre::AsyncTransport as _;
        self.transport
            .send(msg)
            .await
            .map_err(|e| MailerError::Transport(e.to_string()))?;
        Ok(())
    }
}

// ───────────────────────────── Tests ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn log_mailer_send_magic_link_does_not_error() {
        let m = LogMailer;
        m.send_magic_link("alice@example.com", "https://lobby.test/verify?t=abc")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn log_mailer_send_raw_does_not_error() {
        let m = LogMailer;
        m.send_raw("alice@example.com", "Welcome", "Hi!\nPlease sign in.")
            .await
            .unwrap();
    }
}
