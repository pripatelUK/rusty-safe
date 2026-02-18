use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

use alloy::primitives::B256;

use rusty_safe_signing_core::{KdfAlgorithm, PortError};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct DerivedCrypto {
    pub kdf_algorithm: KdfAlgorithm,
    pub salt: [u8; 16],
    pub enc_key: [u8; 32],
    pub mac_key: [u8; 32],
}

pub fn generate_salt() -> Result<[u8; 16], PortError> {
    let mut salt = [0u8; 16];
    getrandom::getrandom(&mut salt)
        .map_err(|e| PortError::Transport(format!("salt generation failed: {e}")))?;
    Ok(salt)
}

pub fn generate_nonce() -> Result<[u8; 12], PortError> {
    let mut nonce = [0u8; 12];
    getrandom::getrandom(&mut nonce)
        .map_err(|e| PortError::Transport(format!("nonce generation failed: {e}")))?;
    Ok(nonce)
}

pub fn derive_crypto(passphrase: &[u8], salt: [u8; 16]) -> Result<DerivedCrypto, PortError> {
    let (root_key, kdf_algorithm) = derive_root_key(passphrase, &salt);
    let hk = Hkdf::<Sha256>::new(None, &root_key);
    let mut enc_key = [0u8; 32];
    let mut mac_key = [0u8; 32];
    hk.expand(b"enc_key_v1", &mut enc_key)
        .map_err(|_| PortError::Validation("hkdf expand for enc_key_v1 failed".to_owned()))?;
    hk.expand(b"mac_key_v1", &mut mac_key)
        .map_err(|_| PortError::Validation("hkdf expand for mac_key_v1 failed".to_owned()))?;
    Ok(DerivedCrypto {
        kdf_algorithm,
        salt,
        enc_key,
        mac_key,
    })
}

pub fn encrypt_aes_gcm(
    enc_key: &[u8; 32],
    nonce: [u8; 12],
    plaintext: &[u8],
) -> Result<Vec<u8>, PortError> {
    let cipher = Aes256Gcm::new_from_slice(enc_key)
        .map_err(|e| PortError::Validation(format!("aes-gcm init failed: {e}")))?;
    let nonce = Nonce::<aes_gcm::aead::consts::U12>::from(nonce);
    cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| PortError::Transport(format!("aes-gcm encrypt failed: {e}")))
}

pub fn decrypt_aes_gcm(
    enc_key: &[u8; 32],
    nonce: [u8; 12],
    ciphertext: &[u8],
) -> Result<Vec<u8>, PortError> {
    let cipher = Aes256Gcm::new_from_slice(enc_key)
        .map_err(|e| PortError::Validation(format!("aes-gcm init failed: {e}")))?;
    let nonce = Nonce::<aes_gcm::aead::consts::U12>::from(nonce);
    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|e| PortError::Validation(format!("aes-gcm decrypt failed: {e}")))
}

pub fn hmac_sha256_b256(mac_key: &[u8; 32], payload: &[u8]) -> Result<B256, PortError> {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(mac_key)
        .map_err(|e| PortError::Validation(format!("hmac init failed: {e}")))?;
    mac.update(payload);
    let out = mac.finalize().into_bytes();
    Ok(B256::from_slice(&out))
}

pub fn canonical_json_bytes(value: &serde_json::Value) -> Result<Vec<u8>, PortError> {
    let normalized = normalize_json(value);
    serde_json::to_vec(&normalized)
        .map_err(|e| PortError::Validation(format!("canonical json serialization failed: {e}")))
}

fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Map, Value};
    match value {
        Value::Object(map) => {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort_unstable();
            let mut out = Map::with_capacity(keys.len());
            for key in keys {
                if let Some(v) = map.get(&key) {
                    out.insert(key, normalize_json(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for v in items {
                out.push(normalize_json(v));
            }
            Value::Array(out)
        }
        _ => value.clone(),
    }
}

fn derive_root_key(passphrase: &[u8], salt: &[u8; 16]) -> ([u8; 32], KdfAlgorithm) {
    // PRD primary path.
    let mut root = [0u8; 32];
    let params = Params::new(65536, 3, 1, Some(32));
    if let Ok(params) = params {
        let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        if argon
            .hash_password_into(passphrase, salt, &mut root)
            .is_ok()
        {
            return (root, KdfAlgorithm::Argon2idV1);
        }
    }

    // PRD fallback path.
    pbkdf2_hmac::<Sha256>(passphrase, salt, 600_000, &mut root);
    (root, KdfAlgorithm::Pbkdf2HmacSha256V1)
}
