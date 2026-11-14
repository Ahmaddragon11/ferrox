// SPDX-License-Identifier: MIT OR Apache-2.0

use ed25519_dalek::{Signature, VerifyingKey};
use ferrox_core::{FerroxError, Result};
use sha2::{Digest, Sha256};

/// Canonical payload that must be signed by the release signing key.
pub fn signing_payload(asset_url: &str, sha256_hex: &str) -> String {
    format!(
        "ferrox-update-v1\n{asset_url}\n{}",
        sha256_hex.to_ascii_lowercase()
    )
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> Result<()> {
    let actual = sha256_hex(bytes);
    if actual.eq_ignore_ascii_case(expected_hex.trim()) {
        Ok(())
    } else {
        Err(FerroxError::Update(format!(
            "SHA-256 mismatch: expected {expected_hex}, got {actual}"
        )))
    }
}

pub fn verify_signature(
    public_key_hex: &str,
    asset_url: &str,
    sha256_hex: &str,
    signature_hex: &str,
) -> Result<()> {
    let key_bytes = decode_fixed::<32>(public_key_hex, "public key")?;
    let verifying_key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| FerroxError::Update(format!("invalid public key: {e}")))?;

    let sig_bytes = decode_fixed::<64>(signature_hex, "signature")?;
    let signature = Signature::from_bytes(&sig_bytes);

    let payload = signing_payload(asset_url, sha256_hex);
    verifying_key
        .verify_strict(payload.as_bytes(), &signature)
        .map_err(|e| FerroxError::Update(format!("signature verification failed: {e}")))
}

fn decode_fixed<const N: usize>(raw: &str, label: &str) -> Result<[u8; N]> {
    let bytes = hex::decode(raw.trim())
        .map_err(|e| FerroxError::Update(format!("invalid {label} hex: {e}")))?;
    bytes
        .try_into()
        .map_err(|_| FerroxError::Update(format!("{label} must be {N} bytes")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn test_keypair() -> (SigningKey, String) {
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let public_hex = hex::encode(signing.verifying_key().to_bytes());
        (signing, public_hex)
    }

    #[test]
    fn hash_and_signature_roundtrip() {
        let (signing, public_hex) = test_keypair();
        let url = "https://example.com/ferrox-1.2.0.zip";
        let sha = sha256_hex(b"ferrox-binary");
        let signature = signing.sign(signing_payload(url, &sha).as_bytes());
        let sig_hex = hex::encode(signature.to_bytes());

        verify_sha256(b"ferrox-binary", &sha).unwrap();
        verify_signature(&public_hex, url, &sha, &sig_hex).unwrap();
    }

    #[test]
    fn rejects_tampered_payload_and_hash() {
        let (signing, public_hex) = test_keypair();
        let url = "https://example.com/ferrox-1.2.0.zip";
        let sha = sha256_hex(b"ferrox-binary");
        let signature = signing.sign(signing_payload(url, &sha).as_bytes());
        let sig_hex = hex::encode(signature.to_bytes());

        assert!(
            verify_signature(&public_hex, "https://evil.example/x.zip", &sha, &sig_hex).is_err()
        );
        assert!(verify_signature(&public_hex, url, &sha256_hex(b"other"), &sig_hex).is_err());
        assert!(verify_sha256(b"other", &sha).is_err());
    }
}
