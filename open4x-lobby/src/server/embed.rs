//! Compile-time-embedded SPA + book assets for single-binary
//! deploy.
//!
//! When the binary is built, `rust-embed` snapshots the files in
//! `open4x-lobby/dist/` and `book/book/` into the binary itself so a
//! deploy is just `open4x-lobby` + an `OPEN4X_LOBBY_DATA_DIR`. If
//! `OPEN4X_LOBBY_STATIC_DIR` is set at runtime (the dev path —
//! `trunk serve` overlays a watched directory) the binary continues
//! to use ServeDir against that path; the embedded fallback only
//! kicks in when the env var is unset.
//!
//! The rust-embed `folder` paths are relative to `CARGO_MANIFEST_DIR`,
//! so they resolve to `<repo>/open4x-lobby/dist/` and `<repo>/book/book/`
//! at compile time. Both directories must *exist* at compile time —
//! rust-embed's derive macro fails on a missing folder — so the source
//! tree ships a `.gitkeep` in each. An otherwise-empty dir produces a
//! bundle with only that marker, which [`spa_assets_present`] /
//! [`book_assets_present`] correctly treat as "no real assets".

#![cfg(feature = "ssr")]

use axum::body::Body;
use axum::extract::Path;
use axum::http::{StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

// rust-embed resolves these paths relative to CARGO_MANIFEST_DIR
// (= open4x-lobby/) at build time.
#[derive(Embed)]
#[folder = "dist"]
struct SpaAssets;

#[derive(Embed)]
#[folder = "../book/book"]
struct BookAssets;

/// SPA fallback handler: serves `index.html` for any missing route
/// (so client-side routing works) and the matching file otherwise.
///
/// `axum::Router::fallback` doesn't pre-extract the URL path, so we
/// take a `Uri` and pull the path off it ourselves.
pub async fn spa_fallback(uri: Uri) -> Response {
    let trimmed = uri.path().trim_start_matches('/');
    let candidate = if trimmed.is_empty() { "index.html" } else { trimmed };
    if let Some(file) = SpaAssets::get(candidate) {
        return embedded_response(candidate, file);
    }
    // Path missed — fall through to the SPA shell so client-side
    // routes (e.g. /menu, /profile) hit the WASM-driven router.
    if let Some(file) = SpaAssets::get("index.html") {
        return embedded_response("index.html", file);
    }
    (StatusCode::NOT_FOUND, "Lobby SPA not embedded — build with `trunk build` first.")
        .into_response()
}

/// `/book/{*path}` handler. 404s when the embedded book is empty
/// (operator hasn't run `mdbook build book/`).
pub async fn book_handler(Path(path): Path<String>) -> Response {
    let candidate = path.trim_start_matches('/');
    let lookup = if candidate.is_empty() || candidate.ends_with('/') {
        format!("{candidate}index.html")
    } else {
        candidate.to_string()
    };
    if let Some(file) = BookAssets::get(&lookup) {
        return embedded_response(&lookup, file);
    }
    if let Some(file) = BookAssets::get(&format!("{candidate}/index.html")) {
        return embedded_response(&format!("{candidate}/index.html"), file);
    }
    (
        StatusCode::NOT_FOUND,
        "mdBook not embedded — run `mdbook build book/` and rebuild.",
    )
        .into_response()
}

/// Whether the SPA bundle has real content. Lets `main.rs` skip the
/// embedded fallback wiring when the binary was built without running
/// `trunk build` first (dev path). We key on `index.html` rather than
/// "any file" because the source tree ships a `.gitkeep` marker so the
/// `dist/` directory exists at compile time (rust-embed's derive fails
/// on a *missing* folder); that marker must not be mistaken for a real
/// SPA build.
pub fn spa_assets_present() -> bool {
    SpaAssets::get("index.html").is_some()
}

/// Whether the mdBook bundle has real content. Same `.gitkeep` caveat
/// as [`spa_assets_present`]; we key on the book's root `index.html`.
pub fn book_assets_present() -> bool {
    BookAssets::get("index.html").is_some()
}

fn embedded_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    Response::builder()
        .header(header::CONTENT_TYPE, mime.as_ref())
        .body(Body::from(file.data.into_owned()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
