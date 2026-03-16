use crate::error::{BillingError, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use ed25519_dalek::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// The payload embedded inside a signed license.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    pub license_id: String,
    pub customer_id: Option<String>,
    pub customer_name: String,
    pub product_id: Option<String>,
    pub product_name: String,
    pub features: Vec<String>,
    pub max_activations: Option<i32>,
    pub issued_at: String,
    pub expires_at: String,
    pub metadata: Option<serde_json::Value>,
}

/// A license payload together with its ED25519 signature (base64-encoded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedLicense {
    pub payload: LicensePayload,
    /// Base64-encoded ED25519 signature over the canonical JSON of `payload`.
    pub signature: String,
}

/// Generate a new ED25519 keypair and return (public_key_pem, private_key_pem).
pub fn generate_keypair() -> Result<(String, String)> {
    let mut csprng = rand::thread_rng();
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let private_pem = signing_key
        .to_pkcs8_pem(ed25519_dalek::pkcs8::spki::der::pem::LineEnding::LF)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("failed to encode private key: {e}")))?;

    let public_pem = verifying_key
        .to_public_key_pem(ed25519_dalek::pkcs8::spki::der::pem::LineEnding::LF)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("failed to encode public key: {e}")))?;

    Ok((public_pem, private_pem.to_string()))
}

/// Sign a license payload with the given PEM-encoded ED25519 private key.
pub fn sign_license(payload: &LicensePayload, private_key_pem: &str) -> Result<SignedLicense> {
    let signing_key = SigningKey::from_pkcs8_pem(private_key_pem)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("invalid private key PEM: {e}")))?;

    let canonical_json = serde_json::to_vec(payload)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("failed to serialize payload: {e}")))?;

    let signature = signing_key.sign(&canonical_json);
    let signature_b64 = BASE64.encode(signature.to_bytes());

    Ok(SignedLicense {
        payload: payload.clone(),
        signature: signature_b64,
    })
}

/// Verify the signature on a signed license using the given PEM-encoded ED25519 public key.
/// Returns `Ok(true)` if the signature is valid, `Ok(false)` if it is invalid.
pub fn verify_license(signed: &SignedLicense, public_key_pem: &str) -> Result<bool> {
    let verifying_key = VerifyingKey::from_public_key_pem(public_key_pem)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("invalid public key PEM: {e}")))?;

    let sig_bytes = BASE64
        .decode(&signed.signature)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("invalid signature base64: {e}")))?;

    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("invalid signature bytes: {e}")))?;

    let canonical_json = serde_json::to_vec(&signed.payload)
        .map_err(|e| BillingError::Internal(anyhow::anyhow!("failed to serialize payload: {e}")))?;

    Ok(verifying_key.verify(&canonical_json, &signature).is_ok())
}

/// Serialize a signed license into a human-readable license file format.
///
/// ```text
/// -----BEGIN LICENSE-----
/// <base64-encoded JSON payload>
/// -----END LICENSE-----
/// -----BEGIN SIGNATURE-----
/// <base64-encoded signature>
/// -----END SIGNATURE-----
/// ```
pub fn to_license_file(signed: &SignedLicense) -> String {
    let payload_json = serde_json::to_vec(&signed.payload).expect("payload serialization");
    let payload_b64 = BASE64.encode(&payload_json);

    // Wrap base64 at 76 chars for readability
    let wrap = |s: &str| -> String {
        s.as_bytes()
            .chunks(76)
            .map(|chunk| std::str::from_utf8(chunk).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "-----BEGIN LICENSE-----\n{}\n-----END LICENSE-----\n-----BEGIN SIGNATURE-----\n{}\n-----END SIGNATURE-----\n",
        wrap(&payload_b64),
        wrap(&signed.signature),
    )
}

/// Parse a license file (as produced by [`to_license_file`]) back into a [`SignedLicense`].
pub fn parse_license_file(content: &str) -> Result<SignedLicense> {
    let err = || BillingError::bad_request("invalid license file format");

    // Extract content between markers
    let extract = |begin: &str, end: &str| -> std::result::Result<String, BillingError> {
        let start_idx = content.find(begin).ok_or_else(err)? + begin.len();
        let end_idx = content.find(end).ok_or_else(err)?;
        let block = &content[start_idx..end_idx];
        // Strip whitespace/newlines from the base64 block
        Ok(block.chars().filter(|c| !c.is_whitespace()).collect())
    };

    let payload_b64 = extract("-----BEGIN LICENSE-----", "-----END LICENSE-----")?;
    let signature_b64 = extract("-----BEGIN SIGNATURE-----", "-----END SIGNATURE-----")?;

    let payload_bytes = BASE64
        .decode(&payload_b64)
        .map_err(|_| BillingError::bad_request("invalid base64 in license payload"))?;

    let payload: LicensePayload = serde_json::from_slice(&payload_bytes)
        .map_err(|_| BillingError::bad_request("invalid JSON in license payload"))?;

    Ok(SignedLicense {
        payload,
        signature: signature_b64,
    })
}
