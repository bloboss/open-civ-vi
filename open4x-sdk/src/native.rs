//! Native HTTP transport (reqwest-backed).
//!
//! Two clients live behind separate feature gates so a CLI consumer that
//! never touches async runtimes can pull in `native-blocking` alone, and
//! a server-side test harness can pull in `native-async` without
//! enabling the blocking client's transitive scheduler.
//!
//! - [`NativeBlockingClient`] (`feature = "native-blocking"`) — wraps
//!   `reqwest::blocking::Client`. Each [`Transport::request`] call does the
//!   HTTP work eagerly (synchronously) and returns a future that's already
//!   ready. Safe to use from either sync or async contexts; the future
//!   itself never `.await`s anything that could block.
//! - [`NativeAsyncClient`] (`feature = "native-async"`) — wraps
//!   `reqwest::Client`. The `Transport::request` future actually awaits the
//!   network round-trip. Used by the in-process Axum integration tests.
//!
//! Both clients trim trailing slashes from the configured `base` so the
//! `path` argument (e.g. `/api/v1/health`) always concatenates cleanly.

#[cfg(any(feature = "native-blocking", feature = "native-async"))]
use crate::error::ApiError;
#[cfg(any(feature = "native-blocking", feature = "native-async"))]
use crate::transport::{Method, Transport};

#[cfg(any(feature = "native-blocking", feature = "native-async"))]
fn trim_base(mut base: String) -> String {
    while base.ends_with('/') {
        base.pop();
    }
    base
}

#[cfg(any(feature = "native-blocking", feature = "native-async"))]
fn join_url(base: &str, path: &str) -> String {
    if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

// ── Blocking client ──────────────────────────────────────────────────────────

#[cfg(feature = "native-blocking")]
pub struct NativeBlockingClient {
    base: String,
    token: Option<String>,
    inner: reqwest::blocking::Client,
}

#[cfg(feature = "native-blocking")]
impl NativeBlockingClient {
    /// Construct a client targeting `base` (e.g. `"http://localhost:8080"`).
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: trim_base(base.into()),
            token: None,
            inner: reqwest::blocking::Client::builder()
                .build()
                .expect("failed to build reqwest::blocking::Client"),
        }
    }

    /// Attach a bearer token sent on every subsequent request.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    fn do_request(
        &self,
        method: Method,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<Vec<u8>, ApiError> {
        let url = join_url(&self.base, path);
        let mut req = match method {
            Method::Get => self.inner.get(&url),
            Method::Post => self.inner.post(&url),
            Method::Put => self.inner.put(&url),
            Method::Patch => self.inner.patch(&url),
            Method::Delete => self.inner.delete(&url),
        };
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        if let Some(b) = body {
            req = req
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(b.to_vec());
        }

        let resp = req.send().map_err(|e| {
            ApiError::transport(format!("{method} {path}: {e}", method = method.as_str()))
        })?;
        let status = resp.status().as_u16();
        let bytes = resp.bytes().map_err(|e| {
            ApiError::transport(format!(
                "{method} {path}: read body: {e}",
                method = method.as_str()
            ))
        })?;

        if (200..300).contains(&status) {
            Ok(bytes.to_vec())
        } else {
            let body = std::str::from_utf8(&bytes).unwrap_or("");
            Err(ApiError::from_response(status, body))
        }
    }
}

#[cfg(feature = "native-blocking")]
impl Transport for NativeBlockingClient {
    fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&[u8]>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, ApiError>> + Send {
        // Run the blocking HTTP call eagerly; wrap the result in a ready
        // future so the trait's async signature is satisfied without
        // pulling in an async runtime. This is safe from sync contexts
        // (no nested executor) and from async contexts (the future is
        // immediately ready, never yields).
        let result = self.do_request(method, path, body);
        async move { result }
    }
}

// ── Async client ─────────────────────────────────────────────────────────────

#[cfg(feature = "native-async")]
pub struct NativeAsyncClient {
    base: String,
    token: Option<String>,
    inner: reqwest::Client,
}

#[cfg(feature = "native-async")]
impl NativeAsyncClient {
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: trim_base(base.into()),
            token: None,
            inner: reqwest::Client::builder()
                .build()
                .expect("failed to build reqwest::Client"),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn base(&self) -> &str {
        &self.base
    }
}

#[cfg(feature = "native-async")]
impl Transport for NativeAsyncClient {
    fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&[u8]>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, ApiError>> + Send {
        let url = join_url(&self.base, path);
        let mut req = match method {
            Method::Get => self.inner.get(&url),
            Method::Post => self.inner.post(&url),
            Method::Put => self.inner.put(&url),
            Method::Patch => self.inner.patch(&url),
            Method::Delete => self.inner.delete(&url),
        };
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        if let Some(b) = body {
            req = req
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(b.to_vec());
        }
        // Pre-format the descriptor used in error messages so the future
        // can be `'static` w.r.t. method/path borrowing rules.
        let descriptor = format!("{} {}", method.as_str(), path);

        async move {
            let resp = req
                .send()
                .await
                .map_err(|e| ApiError::transport(format!("{descriptor}: {e}")))?;
            let status = resp.status().as_u16();
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| ApiError::transport(format!("{descriptor}: read body: {e}")))?;
            if (200..300).contains(&status) {
                Ok(bytes.to_vec())
            } else {
                let body = std::str::from_utf8(&bytes).unwrap_or("");
                Err(ApiError::from_response(status, body))
            }
        }
    }
}
