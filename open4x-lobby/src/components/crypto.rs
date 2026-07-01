//! Browser-side Ed25519 keypair for the pubkey challenge-response login.
//!
//! The keypair is the user's portable identity: the same key authenticates
//! against the lobby (this flow) and, in future, the in-game `open4x-server`
//! bearer surface. We persist the 32-byte seed in `localStorage` (hex) and
//! re-derive the `SigningKey` on demand. Signing happens in pure Rust/wasm
//! via `ed25519-dalek` — no WebCrypto, no JS shim.
//!
//! Note on storage: the seed lives in `localStorage`, readable by same-origin
//! JS, so this is "secure by possession" against the network, not against a
//! local XSS compromise. That's an acceptable v1 posture for a self-hosted
//! lobby; a future hardening pass can move to a non-extractable WebCrypto
//! `CryptoKey` in IndexedDB.

use ed25519_dalek::{Signer, SigningKey};

/// localStorage key holding the hex-encoded 32-byte Ed25519 seed.
const SEED_KEY: &str = "open4x_ed25519_seed";

fn local_storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or_else(|| "no window".to_string())?
        .local_storage()
        .map_err(|_| "localStorage unavailable".to_string())?
        .ok_or_else(|| "localStorage is null".to_string())
}

/// Load the persisted signing key, or generate + persist a fresh one on
/// first use.
pub fn load_or_create_key() -> Result<SigningKey, String> {
    let store = local_storage()?;
    if let Ok(Some(hex_seed)) = store.get_item(SEED_KEY) {
        if let Ok(bytes) = hex::decode(&hex_seed) {
            if let Ok(seed) = <[u8; 32]>::try_from(bytes.as_slice()) {
                return Ok(SigningKey::from_bytes(&seed));
            }
        }
        // Corrupt entry — fall through and regenerate.
    }
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| format!("getrandom failed: {e}"))?;
    store
        .set_item(SEED_KEY, &hex::encode(seed))
        .map_err(|_| "localStorage write failed".to_string())?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Lowercase-hex 32-byte public key for the wire.
pub fn public_key_hex(sk: &SigningKey) -> String {
    hex::encode(sk.verifying_key().to_bytes())
}

/// Sign a hex-encoded challenge nonce, returning the 64-byte signature as
/// lowercase hex.
pub fn sign_challenge(sk: &SigningKey, nonce_hex: &str) -> Result<String, String> {
    let nonce = hex::decode(nonce_hex).map_err(|_| "invalid nonce hex".to_string())?;
    Ok(hex::encode(sk.sign(&nonce).to_bytes()))
}
