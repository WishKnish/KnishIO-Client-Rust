//! Custody-agnostic envelope encryption for KnishIO secrets
//!
//! Provides standard AES-256-GCM envelope encryption with PBKDF2-HMAC-SHA256
//! key derivation (100,000 iterations, 16-byte salt, 12-byte IV).
//! Matches the wire format shared by TS, JS, Kotlin, and Rust SDKs.

use super::{EncryptedSecretPayload, SecretStorageMetadata};
use crate::error::{KnishIOError, Result};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use rand::Rng;
use zeroize::Zeroizing;

pub const DEFAULT_ITERATIONS: u32 = 100_000;
pub const SALT_LENGTH: usize = 16;
pub const IV_LENGTH: usize = 12;

/// Derive an AES-256 key from a passphrase and salt using PBKDF2-HMAC-SHA256
pub fn derive_key(passphrase: &str, salt: &[u8], iterations: u32) -> Zeroizing<[u8; 32]> {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, iterations, &mut key);
    Zeroizing::new(key)
}

/// Seal a secret into an EncryptedSecretPayload envelope
pub fn seal(
    secret: &str,
    passphrase: &str,
    metadata: SecretStorageMetadata,
) -> Result<EncryptedSecretPayload> {
    let mut salt = [0u8; SALT_LENGTH];
    let mut iv = [0u8; IV_LENGTH];
    rand::rng().fill_bytes(&mut salt);
    rand::rng().fill_bytes(&mut iv);

    let key = derive_key(passphrase, &salt, DEFAULT_ITERATIONS);
    let cipher = Aes256Gcm::new_from_slice(&*key)
        .map_err(|e| KnishIOError::SecretStorage(format!("Cipher init failed: {}", e)))?;

    let nonce = Nonce::from(iv);
    let ciphertext = cipher
        .encrypt(&nonce, secret.as_bytes())
        .map_err(|e| KnishIOError::SecretStorage(format!("Encryption failed: {}", e)))?;

    Ok(EncryptedSecretPayload {
        version: 1,
        ciphertext: BASE64.encode(&ciphertext),
        iv: BASE64.encode(iv),
        salt: BASE64.encode(salt),
        algorithm: "AES-GCM".to_string(),
        iterations: DEFAULT_ITERATIONS,
        metadata,
    })
}

/// Open an EncryptedSecretPayload envelope with a passphrase, returning the zeroizing plaintext bytes
pub fn open(payload: &EncryptedSecretPayload, passphrase: &str) -> Result<Zeroizing<Vec<u8>>> {
    let salt = BASE64
        .decode(&payload.salt)
        .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid salt base64: {}", e)))?;
    let iv = BASE64
        .decode(&payload.iv)
        .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid IV base64: {}", e)))?;
    let ciphertext = BASE64
        .decode(&payload.ciphertext)
        .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid ciphertext base64: {}", e)))?;

    let key = derive_key(passphrase, &salt, payload.iterations);
    let cipher = Aes256Gcm::new_from_slice(&*key)
        .map_err(|e| KnishIOError::DecryptionFailed(format!("Cipher init failed: {}", e)))?;

    let nonce = Nonce::try_from(iv.as_slice())
        .map_err(|e| KnishIOError::DecryptionFailed(format!("Invalid IV length: {}", e)))?;
    let decrypted = cipher
        .decrypt(&nonce, ciphertext.as_ref())
        .map_err(|e| KnishIOError::DecryptionFailed(format!("Decryption authentication failed: {}", e)))?;

    Ok(Zeroizing::new(decrypted))
}
