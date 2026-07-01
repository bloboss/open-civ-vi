//! In-memory single-use challenge store for pubkey auth.
//!
//! A login challenge is a random 32-byte nonce the server hands out
//! for a given public key. The client signs it with the matching
//! Ed25519 private key and posts the signature back; the server
//! verifies, then *consumes* the nonce so it can never be replayed.
//!
//! Challenges are deliberately ephemeral and process-local: they live
//! ~2 minutes, are dropped on restart, and never touch the database.
//! That keeps the flow lightweight — losing a pending challenge just
//! means the client re-requests one. The map is keyed by lowercase-hex
//! pubkey, so requesting a fresh challenge overwrites any prior one.

#![cfg(feature = "ssr")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use rand::RngCore;

/// How long an issued challenge stays valid.
pub const CHALLENGE_TTL_SECS: i64 = 120;

struct Entry {
    nonce: [u8; 32],
    expires: DateTime<Utc>,
}

/// Cheap-`Clone` handle over a shared challenge map. One instance lives
/// on `AppState`; handlers clone it freely.
#[derive(Clone, Default)]
pub struct PubkeyChallengeStore {
    inner: Arc<Mutex<HashMap<String, Entry>>>,
}

impl PubkeyChallengeStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Issue a fresh random nonce for `pubkey_hex`, replacing any prior
    /// pending challenge for the same key. Expired entries are swept
    /// opportunistically on each issue. Returns the nonce so the
    /// handler can hand it to the client.
    pub fn issue(&self, pubkey_hex: &str, now: DateTime<Utc>) -> [u8; 32] {
        let mut nonce = [0u8; 32];
        rand::rng().fill_bytes(&mut nonce);
        let mut guard = self.inner.lock().expect("challenge map poisoned");
        guard.retain(|_, e| e.expires > now);
        guard.insert(
            pubkey_hex.to_ascii_lowercase(),
            Entry {
                nonce,
                expires: now + Duration::seconds(CHALLENGE_TTL_SECS),
            },
        );
        nonce
    }

    /// Consume the pending nonce for `pubkey_hex` (single use). Returns
    /// `None` if there is no pending challenge or it has expired; in
    /// both cases any stored entry is removed.
    pub fn consume(&self, pubkey_hex: &str, now: DateTime<Utc>) -> Option<[u8; 32]> {
        let mut guard = self.inner.lock().expect("challenge map poisoned");
        match guard.remove(&pubkey_hex.to_ascii_lowercase()) {
            Some(e) if e.expires > now => Some(e.nonce),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_then_consume_roundtrips_once() {
        let store = PubkeyChallengeStore::new();
        let now = Utc::now();
        let nonce = store.issue("ABCD", now);
        // Key match is case-insensitive.
        assert_eq!(store.consume("abcd", now), Some(nonce));
        // Single use: a second consume finds nothing.
        assert_eq!(store.consume("abcd", now), None);
    }

    #[test]
    fn expired_challenge_is_rejected() {
        let store = PubkeyChallengeStore::new();
        let now = Utc::now();
        store.issue("abcd", now);
        let later = now + Duration::seconds(CHALLENGE_TTL_SECS + 1);
        assert_eq!(store.consume("abcd", later), None);
    }

    #[test]
    fn reissue_overwrites_prior_nonce() {
        let store = PubkeyChallengeStore::new();
        let now = Utc::now();
        let first = store.issue("abcd", now);
        let second = store.issue("abcd", now);
        assert_ne!(first, second);
        // Only the latest nonce is valid.
        assert_eq!(store.consume("abcd", now), Some(second));
    }
}
