//! Account & identity model for the Open4X platform.
//!
//! This crate is the type-level skeleton for what will become the auth /
//! lobby substrate. Today it ships **shapes only** — no DB, no OAuth, no
//! token signing. The downstream `open4x-lobby` crate uses these types as
//! placeholders so the screen ports compile against a real `Account`/
//! `Identity` shape, and so we have a clear seam to fill in later.
//!
//! Out of scope, pending implementation:
//! - Persistence (sqlite/postgres backend)
//! - Email magic-link minting + verification
//! - OIDC client (Google/GitHub/GitLab/Microsoft + custom issuer)
//! - atproto OAuth + DID resolution
//! - Identity linking (multiple `Identity`s → one `PlayerId`)
//! - Session tokens + refresh
//!
//! See `docs/open4x-landing/project/hifi/login.jsx` and `profile` for the
//! UX surfaces these flows back.

#![allow(dead_code)]

#[cfg(feature = "persistence")]
pub mod audit;

#[cfg(feature = "persistence")]
pub mod friends;

#[cfg(feature = "persistence")]
pub mod games;

#[cfg(feature = "persistence")]
pub mod magic_link;

#[cfg(feature = "persistence")]
pub mod mailer;

#[cfg(feature = "persistence")]
pub mod oidc;

#[cfg(feature = "persistence")]
pub mod presets;

#[cfg(feature = "persistence")]
pub mod session;

#[cfg(feature = "persistence")]
pub mod store;

use serde::{Deserialize, Serialize};

// ─────────────────────────── Player identity ──────────────────────────────────

/// Stable, opaque identifier for a single platform user.
///
/// Wire format: `0xA9C3·7F12·EE04` — 16 hex digits (64 bits) split into
/// dot-grouped quads, prefixed `0x`. Internally a u64. Generated on first
/// successful sign-in and immutable thereafter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct PlayerId(pub u64);

impl PlayerId {
    pub fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Pretty-print as `0xAAAA·BBBB·CCCC·DDDD`.
    pub fn display(&self) -> String {
        let raw = self.0;
        format!(
            "0x{:04X}·{:04X}·{:04X}·{:04X}",
            (raw >> 48) & 0xFFFF,
            (raw >> 32) & 0xFFFF,
            (raw >> 16) & 0xFFFF,
            raw & 0xFFFF,
        )
    }
}

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.display())
    }
}

// ────────────────────────────── Identities ────────────────────────────────────

/// One way the user can prove they are the owner of a [`PlayerId`].
///
/// A single `Account` may have many `Identity` rows attached; any of them
/// resolves the same `PlayerId` on sign-in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Identity {
    /// Email + magic-link.
    Email {
        address: String,
        verified: bool,
        primary: bool,
    },
    /// OpenID Connect — `issuer` is the discovery URL, `subject` is the
    /// `sub` claim from the ID token.
    OpenId {
        issuer: String,
        subject: String,
        /// Display label used in UIs (e.g. `"google.com / 110293·a73f"`).
        label: String,
    },
    /// atproto. Stored as `did:plc:…` plus the user-friendly handle.
    Atproto { did: String, handle: String },
    /// Ed25519 public key. The lowercase-hex 32-byte key is the stable
    /// identifier; authentication is by challenge-response signature
    /// (the holder proves possession of the private key), so a linked
    /// `PublicKey` identity is inherently verified. The same keypair can
    /// authenticate against the in-game `open4x-server` bearer surface.
    PublicKey { ed25519_hex: String, label: String },
}

impl Identity {
    pub fn label(&self) -> String {
        match self {
            Identity::Email { address, .. } => address.clone(),
            Identity::OpenId { label, .. } => label.clone(),
            Identity::Atproto { handle, did } => format!("{handle} ({did})"),
            Identity::PublicKey { ed25519_hex, label } => {
                if !label.is_empty() {
                    label.clone()
                } else {
                    // Short fingerprint: first 8 + last 4 hex chars.
                    pubkey_fingerprint(ed25519_hex)
                }
            }
        }
    }

    pub fn type_label(&self) -> &'static str {
        match self {
            Identity::Email { .. } => "EMAIL",
            Identity::OpenId { .. } => "OPENID",
            Identity::Atproto { .. } => "ATPROTO",
            Identity::PublicKey { .. } => "PUBKEY",
        }
    }
}

/// Short human-readable fingerprint for an Ed25519 public key given as
/// lowercase hex: `first8…last4`. Falls back to the whole string when
/// it's too short to abbreviate.
pub fn pubkey_fingerprint(ed25519_hex: &str) -> String {
    if ed25519_hex.len() <= 12 {
        ed25519_hex.to_string()
    } else {
        format!(
            "{}…{}",
            &ed25519_hex[..8],
            &ed25519_hex[ed25519_hex.len() - 4..]
        )
    }
}

// ────────────────────────────── Account row ──────────────────────────────────

/// A user account: `PlayerId` plus the identities linked to it plus profile
/// preferences. The "row" stored by the (future) accounts service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Account {
    pub player_id: PlayerId,
    pub preferred_name: String,
    pub pronouns: String,
    pub bio: String,
    pub identities: Vec<Identity>,
    pub prefs: Preferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub density: Density,
    pub color_scheme: ColorScheme,
    pub keyboard_nav: bool,
    pub turn_notifications: bool,
    pub discoverable_by_id: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            density: Density::Comfortable,
            color_scheme: ColorScheme::Paper,
            keyboard_nav: true,
            turn_notifications: true,
            discoverable_by_id: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    Compact,
    Comfortable,
    Spacious,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    Paper,
    Ink,
    Auto,
}

// ─────────────────────────── Magic-link token ─────────────────────────────────

/// Opaque single-use token mailed to the user during the email login flow.
///
/// TODO: implement minting, signing, and 15-minute expiration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MagicLinkToken(pub String);

// ─────────────────────────────── Tests ───────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_id_format() {
        let id = PlayerId::new(0xA9C37F12EE04ABCDu64);
        assert_eq!(id.display(), "0xA9C3·7F12·EE04·ABCD");
    }

    #[test]
    fn identity_labels() {
        let e = Identity::Email {
            address: "alice@example.com".into(),
            verified: true,
            primary: true,
        };
        assert_eq!(e.type_label(), "EMAIL");
        assert!(e.label().contains("alice"));
    }
}
