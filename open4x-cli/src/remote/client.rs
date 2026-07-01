//! Thin HTTP wrapper used by every remote handler.
//!
//! Since Phase 5 of the crate-split migration, this module is a shim
//! over [`open4x_sdk::native::NativeBlockingClient`]. The SDK owns the
//! actual reqwest plumbing, base-URL handling, bearer-auth header, and
//! `{error, message}` envelope decoding; this wrapper only adapts the
//! SDK's async `Transport::request` (which the blocking client returns
//! as an immediately-ready future) into the sync
//! `Result<serde_json::Value, String>` surface the rest of
//! `open4x-cli::remote` expects.
//!
//! Keeping the shim in place means the action/list/status/view/etc.
//! handlers stay untouched: they continue to call `client.get_json`,
//! `client.post_json`, `client.delete_json` exactly as before.

use open4x_sdk::native::NativeBlockingClient;
use open4x_sdk::{Method, Transport, error::ApiError};
use serde_json::Value;

pub struct ApiClient {
    inner: NativeBlockingClient,
}

impl ApiClient {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            inner: NativeBlockingClient::new(base),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.inner = self.inner.with_token(token);
        self
    }

    pub fn get_json(&self, path: &str) -> Result<Value, String> {
        self.request(Method::Get, path, None)
    }

    pub fn post_json(&self, path: &str, body: &Value) -> Result<Value, String> {
        let bytes = serde_json::to_vec(body)
            .map_err(|e| format!("POST {path}: failed to encode body: {e}"))?;
        self.request(Method::Post, path, Some(&bytes))
    }

    pub fn delete_json(&self, path: &str) -> Result<Value, String> {
        self.request(Method::Delete, path, None)
    }

    fn request(&self, method: Method, path: &str, body: Option<&[u8]>) -> Result<Value, String> {
        // `NativeBlockingClient::request` does the HTTP work eagerly and
        // hands back an immediately-ready future, so `block_on` here
        // does not spin a real executor — it just unwraps the result.
        let ctx = format!("{} {path}", method.as_str());
        let fut = self.inner.request(method, path, body);
        let bytes = pollster::block_on(fut).map_err(|e| format_err(&ctx, &e))?;

        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes).map_err(|e| {
            format!(
                "{ctx}: invalid JSON response ({e}): {}",
                String::from_utf8_lossy(&bytes)
            )
        })
    }
}

/// Reformat an [`ApiError`] to match the legacy bespoke-client error
/// string. The legacy format was:
///
/// ```text
/// {ctx} -> {status} {error_code}: {message} ({body_json})
/// ```
///
/// We approximate it as `"{ctx}: <ApiError Display>"` for transport
/// failures and `"{ctx} -> {status} {code}: {message}"` for HTTP
/// failures. The CLI parity harness only asserts that the relevant
/// error code (e.g. `pending_required_action`) appears in the string;
/// it doesn't grep for the exact framing.
fn format_err(ctx: &str, e: &ApiError) -> String {
    if e.status == 0 {
        // Transport-level failure. The SDK already includes the
        // method/path in its message, so just surface it.
        match &e.message {
            Some(m) => m.clone(),
            None => format!("{ctx}: transport error"),
        }
    } else {
        let msg = e.message.as_deref().unwrap_or("");
        format!("{ctx} -> {} {}: {msg}", e.status, e.code)
    }
}
