//! Client-side Ed25519 keypair management.
//!
//! On first load, generates a keypair and stores the secret key in localStorage.
//! On subsequent loads, restores the keypair from localStorage.

use ed25519_dalek::{SigningKey, Signer, VerifyingKey};
use web_sys::window;

const STORAGE_KEY: &str = "open4x_signing_key";

/// Load or generate the client's Ed25519 signing key.
///
/// The 32-byte secret seed is stored as hex in `localStorage`.
/// Returns the signing key (for signing challenges) and the public key bytes
/// (for sending to the server).
pub fn load_or_generate_keypair() -> (SigningKey, [u8; 32]) {
    let storage = window()
        .and_then(|w| w.local_storage().ok().flatten());

    // Try to load existing key.
    if let Some(storage) = &storage
        && let Ok(Some(hex_str)) = storage.get_item(STORAGE_KEY)
        && let Some(seed) = hex_decode_32(&hex_str)
    {
        let signing = SigningKey::from_bytes(&seed);
        let pubkey = VerifyingKey::from(&signing).to_bytes();
        return (signing, pubkey);
    }

    // Generate new keypair.
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).expect("getrandom failed");
    let signing = SigningKey::from_bytes(&seed);
    let pubkey = VerifyingKey::from(&signing).to_bytes();

    // Persist to localStorage.
    if let Some(storage) = &storage {
        let hex_str = hex_encode(&seed);
        let _ = storage.set_item(STORAGE_KEY, &hex_str);
    }

    (signing, pubkey)
}

/// Sign a challenge nonce with the signing key.
pub fn sign_challenge(key: &SigningKey, nonce: &[u8]) -> Vec<u8> {
    let sig = key.sign(nonce);
    sig.to_bytes().to_vec()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode_32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 { return None; }
    let mut out = [0u8; 32];
    for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
