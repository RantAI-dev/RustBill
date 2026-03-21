//! LemonSqueezy payment compatibility layer.

use crate::error::Result;
use crate::settings::provider_settings::ProviderSettings;
use reqwest::Client;

pub use super::schema::{LsCheckoutParams, LsCheckoutResult};

pub async fn create_checkout(
    http: &Client,
    settings: &ProviderSettings,
    params: LsCheckoutParams,
) -> Result<LsCheckoutResult> {
    super::service::create_ls_checkout(http, settings, params).await
}

pub fn verify_webhook(raw_body: &str, signature: Option<&str>, secret: &str) -> bool {
    super::service::verify_ls_webhook(raw_body, signature, secret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    #[test]
    fn test_verify_webhook_valid() {
        let secret = "test_secret";
        let body = r#"{"event":"order_created"}"#;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac");
        mac.update(body.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());

        assert!(verify_webhook(body, Some(&sig), secret));
    }

    #[test]
    fn test_verify_webhook_invalid() {
        assert!(!verify_webhook("body", Some("wrong"), "secret"));
        assert!(!verify_webhook("body", None, "secret"));
    }
}
