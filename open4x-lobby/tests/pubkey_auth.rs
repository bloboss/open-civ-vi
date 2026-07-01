//! In-process integration test for the Ed25519 pubkey challenge-response
//! auth flow. Drives the real `/api/v1` router (with the session-cookie
//! middleware) via `tower::ServiceExt::oneshot`, signing challenges with a
//! real Ed25519 key. Replaces the throwaway curl/python smoke.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use open4x_lobby::server::{api_app, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

/// POST a JSON body, optionally with a session cookie. Returns
/// `(status, set_cookie, json_body)`.
async fn post(
    app: &axum::Router,
    path: &str,
    body: Value,
    cookie: Option<&str>,
) -> (StatusCode, Option<String>, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");
    if let Some(c) = cookie {
        req = req.header("cookie", c);
    }
    let req = req.body(Body::from(body.to_string())).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let set_cookie = resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, set_cookie, value)
}

async fn get(app: &axum::Router, path: &str, cookie: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .header("cookie", cookie)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// Extract the `lobby_session=...` pair from a Set-Cookie header for replay
/// as a `Cookie:` request header.
fn session_cookie(set_cookie: &str) -> String {
    set_cookie.split(';').next().unwrap_or("").to_string()
}

/// Request a fresh challenge for `pubkey_hex` and return the signed-nonce
/// signature hex, using the given key.
async fn challenge_and_sign(app: &axum::Router, sk: &SigningKey, pubkey_hex: &str) -> String {
    let (st, _, body) = post(app, "/api/v1/auth/pubkey/challenge", json!({"pubkey": pubkey_hex}), None).await;
    assert_eq!(st, StatusCode::OK, "challenge failed: {body}");
    let nonce_hex = body["nonce"].as_str().expect("nonce in challenge response");
    let nonce = hex::decode(nonce_hex).expect("hex nonce");
    hex::encode(sk.sign(&nonce).to_bytes())
}

#[tokio::test]
async fn pubkey_auth_full_flow() {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::boot(dir.path()).await.expect("boot");
    let app = api_app(state);

    // Deterministic key (no RNG version-mismatch headaches).
    let sk = SigningKey::from_bytes(&[7u8; 32]);
    let pubkey_hex = hex::encode(sk.verifying_key().to_bytes());

    // 1. challenge + sign + verify (with an email to link as secondary).
    let sig = challenge_and_sign(&app, &sk, &pubkey_hex).await;
    let (st, set_cookie, body) = post(
        &app,
        "/api/v1/auth/pubkey/verify",
        json!({"pubkey": pubkey_hex, "signature": sig, "email": "keyholder@example.com"}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "verify failed: {body}");
    let player_id = body["player_id"].as_str().expect("player_id").to_string();
    let cookie = session_cookie(&set_cookie.expect("session cookie set"));
    assert!(cookie.starts_with("lobby_session="));

    // 2. /me with the session shows pubkey (verified) + linked email.
    let (st, me) = get(&app, "/api/v1/me", &cookie).await;
    assert_eq!(st, StatusCode::OK, "/me failed: {me}");
    let kinds: Vec<&str> = me["identities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"pubkey"), "pubkey identity missing: {me}");
    assert!(kinds.contains(&"email"), "linked email missing: {me}");

    // 3. Replaying a consumed challenge's signature → 401 (single use).
    let (st, _, body) = post(
        &app,
        "/api/v1/auth/pubkey/verify",
        json!({"pubkey": pubkey_hex, "signature": sig, "email": Value::Null}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "replay should be rejected: {body}");
    assert_eq!(body["error"], "no_pending_challenge");

    // 4. Fresh challenge, BAD signature → 401.
    let (st, _, _) = post(&app, "/api/v1/auth/pubkey/challenge", json!({"pubkey": pubkey_hex}), None).await;
    assert_eq!(st, StatusCode::OK);
    let bad_sig = hex::encode(sk.sign(b"not the nonce").to_bytes());
    let (st, _, body) = post(
        &app,
        "/api/v1/auth/pubkey/verify",
        json!({"pubkey": pubkey_hex, "signature": bad_sig, "email": Value::Null}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "bad sig should be rejected: {body}");
    assert_eq!(body["error"], "bad_signature");

    // 5. Re-login with the same key → same account (idempotent identity).
    let sig2 = challenge_and_sign(&app, &sk, &pubkey_hex).await;
    let (st, _, body) = post(
        &app,
        "/api/v1/auth/pubkey/verify",
        json!({"pubkey": pubkey_hex, "signature": sig2, "email": Value::Null}),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["player_id"].as_str().unwrap(), player_id, "same key must map to same account");
}

#[tokio::test]
async fn pubkey_challenge_rejects_malformed_key() {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::boot(dir.path()).await.expect("boot");
    let app = api_app(state);

    let (st, _, body) = post(&app, "/api/v1/auth/pubkey/challenge", json!({"pubkey": "abcd"}), None).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_pubkey");
}
