//! Single fetch helper used by every binding under
//! `crate::components::api`.
//!
//! Wraps `web_sys::Request` + `Window::fetch_with_request`, encodes
//! the body as JSON, decodes the response with `serde-wasm-bindgen`.
//! Browser auth lives in the `lobby_session` httpOnly cookie which
//! the browser auto-attaches; there's no manual bearer-token hook.

use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, Response};

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: u16,
    pub code: String,
    pub message: Option<String>,
}

impl ApiError {
    pub fn transport(message: impl Into<String>) -> Self {
        Self {
            status: 0,
            code: "transport".to_string(),
            message: Some(message.into()),
        }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.message {
            Some(m) => write!(f, "{} ({}): {m}", self.code, self.status),
            None => write!(f, "{} ({})", self.code, self.status),
        }
    }
}

#[derive(serde::Deserialize, Default)]
struct ApiErrorBody {
    #[serde(default)]
    error: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    message: Option<String>,
}

/// Issue a single REST request and decode the JSON response.
pub async fn fetch_json<T: DeserializeOwned, B: Serialize>(
    method: &str,
    url: &str,
    body: Option<&B>,
) -> Result<T, ApiError> {
    let init = RequestInit::new();
    init.set_method(method);
    init.set_credentials(web_sys::RequestCredentials::SameOrigin);

    if let Some(b) = body {
        let json = serde_json::to_string(b).map_err(|e| ApiError::transport(e.to_string()))?;
        init.set_body(&JsValue::from_str(&json));
    }

    let req = Request::new_with_str_and_init(url, &init)
        .map_err(|e| ApiError::transport(format!("{e:?}")))?;

    let headers = req.headers();
    headers
        .set("accept", "application/json")
        .map_err(|e| ApiError::transport(format!("{e:?}")))?;
    if body.is_some() {
        headers
            .set("content-type", "application/json")
            .map_err(|e| ApiError::transport(format!("{e:?}")))?;
    }

    let window = web_sys::window().ok_or_else(|| ApiError::transport("no window"))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&req))
        .await
        .map_err(|e| ApiError::transport(format!("{e:?}")))?;
    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| ApiError::transport("non-Response result"))?;

    let status = resp.status();
    let text = JsFuture::from(
        resp.text()
            .map_err(|e| ApiError::transport(format!("{e:?}")))?,
    )
    .await
    .map_err(|e| ApiError::transport(format!("{e:?}")))?
    .as_string()
    .unwrap_or_default();

    if !(200..300).contains(&status) {
        let body: ApiErrorBody = serde_json::from_str(&text).unwrap_or_default();
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

    let payload = if text.trim().is_empty() {
        "null"
    } else {
        &text
    };
    serde_json::from_str::<T>(payload).map_err(|e| ApiError::transport(e.to_string()))
}
