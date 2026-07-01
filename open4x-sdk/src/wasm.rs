//! WASM HTTP transport (`web_sys::fetch`-backed).
//!
//! Phase 2b: lifts `open4x-server/src/components/api/http.rs` into the SDK
//! and adapts it to implement the [`Transport`] trait so the per-resource
//! endpoint functions in [`crate::endpoints`] share a single body across
//! the wasm and native backends.
//!
//! ## `Send` bound on the wasm future
//!
//! The [`Transport::request`] trait method returns an
//! `impl Future<Output = ...> + Send`. The future returned by
//! [`wasm_bindgen_futures::JsFuture`] is **not** `Send` (JsValue/Promise are
//! single-threaded). Because the trait is shared with the native backend,
//! we can't drop the bound here. Instead we wrap the future in
//! [`send_wrapper::SendWrapper`], which asserts a single-threaded contract
//! at runtime and unconditionally implements [`Send`]. In a browser there
//! is only one JS thread, so the assertion is always satisfied; on any
//! attempt to `poll` the future from another thread the wrapper panics
//! loudly rather than mis-compiling.

use send_wrapper::SendWrapper;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

use crate::error::ApiError;
use crate::transport::{Method, Transport};

/// SDK client backed by the browser's `fetch` API.
///
/// Construct with [`WasmClient::new`] and optionally attach a bearer token
/// via [`WasmClient::with_token`]. The struct is cheaply cloneable.
#[derive(Debug, Clone)]
pub struct WasmClient {
    base: String,
    token: Option<String>,
}

impl WasmClient {
    /// Create a new client rooted at `base` (e.g. `"http://localhost:3000"`).
    /// Paths passed to [`Transport::request`] are appended verbatim; they
    /// should start with a leading `/` (e.g. `/api/v1/cities`).
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            token: None,
        }
    }

    /// Attach a bearer token to be sent with every request. Returns the
    /// client by value so callers can chain construction:
    /// `WasmClient::new(base).with_token(jwt)`.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Base URL the client was constructed with.
    pub fn base(&self) -> &str {
        &self.base
    }

    /// Bearer token attached to outgoing requests, if any.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

/// Build the full URL by appending `path` to the configured base. We do a
/// minimal join: if `base` ends with `/` and `path` starts with `/`, drop
/// one slash. Otherwise concatenate as-is.
fn join_url(base: &str, path: &str) -> String {
    if base.ends_with('/') && path.starts_with('/') {
        format!("{}{}", base.trim_end_matches('/'), path)
    } else if !base.ends_with('/') && !path.starts_with('/') && !base.is_empty() {
        format!("{base}/{path}")
    } else {
        format!("{base}{path}")
    }
}

/// Translate a JsValue error into an [`ApiError`] tagged as a transport
/// failure. The `{e:?}` formatting matches the legacy helper.
fn transport_err(e: impl std::fmt::Debug) -> ApiError {
    ApiError {
        status: 0,
        code: "transport".to_string(),
        message: Some(format!("{e:?}")),
    }
}

/// Translate a string into an [`ApiError`] tagged as a transport failure.
fn transport_msg(msg: impl Into<String>) -> ApiError {
    ApiError {
        status: 0,
        code: "transport".to_string(),
        message: Some(msg.into()),
    }
}

/// Minimal server error body shape: `{ "error": "...", "message": "..." }`.
/// Both fields are optional; we fall back to `"http_error"` if `error` is
/// missing or empty.
#[derive(Default, serde::Deserialize)]
struct ServerErrorBody {
    #[serde(default)]
    error: String,
    #[serde(default)]
    message: Option<String>,
}

impl Transport for WasmClient {
    fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&[u8]>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, ApiError>> + Send {
        // Snapshot everything the future needs as owned values so it can
        // outlive `&self`. SendWrapper enforces single-threaded polling.
        let url = join_url(&self.base, path);
        let token = self.token.clone();
        // `body` is &[u8] of JSON bytes; convert to owned String for the
        // JS-side Request body. We assume valid UTF-8 (callers serialise via
        // `serde_json`, which is always UTF-8).
        let body_string = body.map(|b| String::from_utf8_lossy(b).into_owned());

        SendWrapper::new(async move {
            let init = RequestInit::new();
            init.set_method(method.as_str());

            if let Some(json) = &body_string {
                init.set_body(&JsValue::from_str(json));
            }

            let req = Request::new_with_str_and_init(&url, &init).map_err(transport_err)?;

            let headers = req.headers();
            headers
                .set("accept", "application/json")
                .map_err(transport_err)?;
            if body_string.is_some() {
                headers
                    .set("content-type", "application/json")
                    .map_err(transport_err)?;
            }
            if let Some(t) = &token {
                headers
                    .set("authorization", &format!("Bearer {t}"))
                    .map_err(transport_err)?;
            }

            let window = web_sys::window().ok_or_else(|| transport_msg("no window"))?;
            let resp_value = JsFuture::from(window.fetch_with_request(&req))
                .await
                .map_err(transport_err)?;
            let resp: Response = resp_value
                .dyn_into()
                .map_err(|_| transport_msg("non-Response result"))?;

            let status = resp.status();
            let text = JsFuture::from(resp.text().map_err(transport_err)?)
                .await
                .map_err(transport_err)?
                .as_string()
                .unwrap_or_default();

            if !(200..300).contains(&status) {
                let body: ServerErrorBody = serde_json::from_str(&text).unwrap_or_default();
                return Err(ApiError {
                    status,
                    code: if body.error.is_empty() {
                        "http_error".into()
                    } else {
                        body.error
                    },
                    message: body.message,
                });
            }

            // 204 / empty bodies return an empty Vec — callers that decode
            // JSON will treat this as missing and can substitute `null`.
            Ok(text.into_bytes())
        })
    }
}
