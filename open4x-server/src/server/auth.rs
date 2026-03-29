//! Ed25519 challenge-response authentication.

use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use rand::Rng;

/// A pending auth challenge.
pub struct AuthChallenge {
    pub nonce: [u8; 32],
}

impl AuthChallenge {
    /// Generate a new random challenge nonce.
    pub fn generate() -> Self {
        let mut rng = rand::rng();
        let mut nonce = [0u8; 32];
        rng.fill(&mut nonce);
        Self { nonce }
    }
}

#[derive(Debug)]
pub enum AuthError {
    InvalidKey,
    InvalidSignature,
    VerificationFailed,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKey => write!(f, "invalid public key"),
            Self::InvalidSignature => write!(f, "invalid signature format"),
            Self::VerificationFailed => write!(f, "signature verification failed"),
        }
    }
}

/// Verify a signed challenge. Returns the verified public key bytes on success.
pub fn verify_auth(
    challenge: &AuthChallenge,
    pubkey_bytes: &[u8],
    signature_bytes: &[u8],
) -> Result<[u8; 32], AuthError> {
    let key_arr: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| AuthError::InvalidKey)?;

    let pubkey = VerifyingKey::from_bytes(&key_arr)
        .map_err(|_| AuthError::InvalidKey)?;

    let sig_arr: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| AuthError::InvalidSignature)?;

    let sig = Signature::from_bytes(&sig_arr);

    pubkey.verify(&challenge.nonce, &sig)
        .map_err(|_| AuthError::VerificationFailed)?;

    Ok(key_arr)
}
