//! OIDC client config and authorization-URL builder.
//!
//! Phase 2.3 of `book/src/roadmap/accounts-and-login.md`. This module
//! covers the **deterministic** half of the OIDC code-flow:
//! provider configuration, scope/PKCE/nonce/state generation, and
//! the authorization-URL string the lobby returns to the browser as
//! a 302.
//!
//! The async exchange + ID-token verification half lands in a
//! follow-up commit (will use the `openidconnect` crate behind a
//! feature gate). The split keeps this commit small and testable
//! without a network mock.
//!
//! GitHub is intentionally absent — it uses OAuth2 + REST userinfo,
//! not OIDC, and gets its own module under Phase 2.3 follow-up.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OidcError {
    #[error("malformed redirect_uri or issuer URL")]
    BadUrl,
    #[error("custom issuer URL is required when provider is Custom")]
    MissingIssuer,
    #[error("scope must be non-empty")]
    NoScopes,
}

/// Pre-configured OIDC providers. Issuer URLs match each provider's
/// `.well-known/openid-configuration` endpoint when suffixed with
/// the discovery path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OidcProvider {
    Google,
    GitLab,
    Microsoft, // Azure AD `common` tenant
    Custom { issuer: String },
}

impl OidcProvider {
    pub fn issuer(&self) -> Result<String, OidcError> {
        Ok(match self {
            OidcProvider::Google => "https://accounts.google.com".into(),
            OidcProvider::GitLab => "https://gitlab.com".into(),
            OidcProvider::Microsoft => "https://login.microsoftonline.com/common/v2.0".into(),
            OidcProvider::Custom { issuer } => {
                if issuer.is_empty() {
                    return Err(OidcError::MissingIssuer);
                }
                issuer.clone()
            }
        })
    }

    pub fn label(&self) -> &str {
        match self {
            OidcProvider::Google => "Google",
            OidcProvider::GitLab => "GitLab",
            OidcProvider::Microsoft => "Microsoft",
            OidcProvider::Custom { .. } => "OpenID",
        }
    }
}

/// Static config for one OIDC client registration. The lobby holds
/// one of these per enabled provider.
#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub provider: OidcProvider,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl OidcConfig {
    pub fn google(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            provider: OidcProvider::Google,
            client_id,
            client_secret: Some(client_secret),
            redirect_uri,
            scopes: vec!["openid".into(), "email".into(), "profile".into()],
        }
    }

    pub fn gitlab(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            provider: OidcProvider::GitLab,
            client_id,
            client_secret: Some(client_secret),
            redirect_uri,
            scopes: vec!["openid".into(), "email".into(), "profile".into()],
        }
    }

    pub fn microsoft(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            provider: OidcProvider::Microsoft,
            client_id,
            client_secret: Some(client_secret),
            redirect_uri,
            scopes: vec!["openid".into(), "email".into(), "profile".into()],
        }
    }

    pub fn custom(
        issuer: String,
        client_id: String,
        client_secret: Option<String>,
        redirect_uri: String,
    ) -> Self {
        Self {
            provider: OidcProvider::Custom { issuer },
            client_id,
            client_secret,
            redirect_uri,
            scopes: vec!["openid".into(), "email".into(), "profile".into()],
        }
    }

    fn discovery_url(&self) -> Result<String, OidcError> {
        Ok(format!(
            "{}/.well-known/openid-configuration",
            self.provider.issuer()?.trim_end_matches('/'),
        ))
    }
}

/// PKCE pair: stash `verifier` server-side; the URL embeds
/// `code_challenge = SHA-256(verifier)` (base64url, no pad).
#[derive(Debug, Clone)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Default for Pkce {
    fn default() -> Self {
        Self::new()
    }
}

impl Pkce {
    /// 64-byte verifier per RFC 7636 §4.1 ("between 43 and 128
    /// characters"); URL-safe alphabet only.
    pub fn new() -> Self {
        let bytes: [u8; 32] = rand::random();
        let verifier = B64.encode(bytes);
        let mut h = Sha256::new();
        h.update(verifier.as_bytes());
        let challenge = B64.encode(h.finalize());
        Self {
            verifier,
            challenge,
        }
    }
}

/// Per-flow ephemeral state the lobby stashes in a signed cookie
/// (or row in `oidc_pending` once that exists). Returned alongside
/// the redirect URL so the callback handler can pull it back.
#[derive(Debug, Clone)]
pub struct AuthorizationRequest {
    /// The redirect URL to 302 the user-agent to.
    pub url: String,
    /// CSRF guard. Must match the `state` query param on callback.
    pub state: String,
    /// Replay guard for the ID-token. Must match the `nonce` claim.
    pub nonce: String,
    /// PKCE pair. Verifier travels in the eventual code-exchange POST.
    pub pkce: Pkce,
}

/// Build the authorization-URL the lobby 302s to. Construction is
/// purely client-side — no discovery happens until exchange time.
///
/// Today this hard-codes the standard `/oauth2/v2.0/authorize` /
/// `/oauth/authorize` endpoints per provider. When the
/// `discover()` half of the client lands, this becomes a thin
/// wrapper that picks the URL from the discovered metadata.
pub fn build_authorization_request(config: &OidcConfig) -> Result<AuthorizationRequest, OidcError> {
    if config.scopes.is_empty() {
        return Err(OidcError::NoScopes);
    }

    let pkce = Pkce::new();
    let state = random_id();
    let nonce = random_id();
    let scope = config.scopes.join(" ");

    let endpoint = match &config.provider {
        OidcProvider::Google => "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
        OidcProvider::GitLab => "https://gitlab.com/oauth/authorize".to_string(),
        OidcProvider::Microsoft => {
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string()
        }
        OidcProvider::Custom { issuer } => {
            format!("{}/oauth2/authorize", issuer.trim_end_matches('/'))
        }
    };

    let url = format!(
        "{endpoint}?response_type=code&client_id={client_id}&redirect_uri={redirect}\
         &scope={scope}&state={state}&nonce={nonce}\
         &code_challenge={challenge}&code_challenge_method=S256",
        client_id = url_encode(&config.client_id),
        redirect = url_encode(&config.redirect_uri),
        scope = url_encode(&scope),
        state = url_encode(&state),
        nonce = url_encode(&nonce),
        challenge = url_encode(&pkce.challenge),
    );

    Ok(AuthorizationRequest {
        url,
        state,
        nonce,
        pkce,
    })
}

fn random_id() -> String {
    let bytes: [u8; 16] = rand::random();
    B64.encode(bytes)
}

/// Minimal, allocation-free percent-encoder for the unreserved set
/// per RFC 3986 §2.3 plus a small extra (slashes pass through).
/// Sufficient for client_id / redirect_uri / scope / state / nonce
/// which never contain control bytes in practice.
fn url_encode(s: &str) -> String {
    const SAFE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~";
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if SAFE.contains(&b) {
            out.push(b as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

// ───────────────────────────── Tests ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_issuers() {
        assert_eq!(
            OidcProvider::Google.issuer().unwrap(),
            "https://accounts.google.com"
        );
        assert_eq!(OidcProvider::GitLab.issuer().unwrap(), "https://gitlab.com");
        assert_eq!(
            OidcProvider::Microsoft.issuer().unwrap(),
            "https://login.microsoftonline.com/common/v2.0"
        );
        assert_eq!(
            OidcProvider::Custom {
                issuer: "https://auth.example.com".into()
            }
            .issuer()
            .unwrap(),
            "https://auth.example.com"
        );
    }

    #[test]
    fn empty_custom_issuer_errors() {
        let err = OidcProvider::Custom {
            issuer: String::new(),
        }
        .issuer()
        .unwrap_err();
        assert!(matches!(err, OidcError::MissingIssuer));
    }

    #[test]
    fn discovery_url_strips_trailing_slash() {
        let cfg = OidcConfig::custom(
            "https://auth.example.com/".into(),
            "id".into(),
            None,
            "https://lobby.example/cb".into(),
        );
        assert_eq!(
            cfg.discovery_url().unwrap(),
            "https://auth.example.com/.well-known/openid-configuration"
        );
    }

    #[test]
    fn pkce_challenge_is_b64url_sha256_of_verifier() {
        let p = Pkce::new();
        assert!(!p.verifier.is_empty());
        let mut h = Sha256::new();
        h.update(p.verifier.as_bytes());
        let expected = B64.encode(h.finalize());
        assert_eq!(p.challenge, expected);
    }

    #[test]
    fn authorization_url_carries_required_params() {
        let cfg = OidcConfig::google(
            "client-123".into(),
            "secret".into(),
            "https://lobby.example/auth/oidc/google/callback".into(),
        );
        let req = build_authorization_request(&cfg).unwrap();
        assert!(
            req.url
                .starts_with("https://accounts.google.com/o/oauth2/v2/auth?")
        );
        assert!(req.url.contains("client_id=client-123"));
        assert!(req.url.contains("response_type=code"));
        assert!(req.url.contains("code_challenge_method=S256"));
        assert!(
            req.url
                .contains(&format!("state={}", url_encode(&req.state)))
        );
        assert!(
            req.url
                .contains(&format!("nonce={}", url_encode(&req.nonce)))
        );
        assert!(req.url.contains("scope=openid%20email%20profile"));
    }

    #[test]
    fn empty_scopes_errors() {
        let mut cfg = OidcConfig::google(
            "id".into(),
            "secret".into(),
            "https://lobby.example/cb".into(),
        );
        cfg.scopes.clear();
        let err = build_authorization_request(&cfg).unwrap_err();
        assert!(matches!(err, OidcError::NoScopes));
    }

    #[test]
    fn url_encode_handles_special_chars() {
        assert_eq!(url_encode("https://x.com/cb"), "https%3A%2F%2Fx.com%2Fcb");
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a-b_c.d~e"), "a-b_c.d~e");
    }
}
