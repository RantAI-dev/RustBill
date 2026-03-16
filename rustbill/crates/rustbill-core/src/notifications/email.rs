//! Email sending via Resend HTTP API (matching existing integration).

use reqwest::Client;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct EmailSender {
    http: Client,
    api_key: String,
    from: String,
}

#[derive(Debug, Serialize)]
struct ResendRequest<'a> {
    from: &'a str,
    to: &'a [&'a str],
    subject: &'a str,
    html: &'a str,
}

impl EmailSender {
    pub fn new(api_key: String, from: String) -> Self {
        Self {
            http: Client::new(),
            api_key,
            from,
        }
    }

    /// Try to create from environment variables. Returns None if RESEND_API_KEY is not set.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("RESEND_API_KEY").ok()?;
        let from = std::env::var("BILLING_FROM_EMAIL")
            .unwrap_or_else(|_| "billing@rantai.com".to_string());
        Some(Self::new(api_key, from))
    }

    /// Send an email. Returns true on success, false on failure (non-blocking).
    pub async fn send(&self, to: &str, subject: &str, html: &str) -> bool {
        let body = ResendRequest {
            from: &self.from,
            to: &[to],
            subject,
            html,
        };

        match self
            .http
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => true,
            Ok(resp) => {
                tracing::warn!(status = %resp.status(), "Email send failed");
                false
            }
            Err(e) => {
                tracing::warn!(error = %e, "Email send error");
                false
            }
        }
    }
}
