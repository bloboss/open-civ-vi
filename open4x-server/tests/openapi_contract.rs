//! Contract-drift tests between the live Axum router and the OpenAPI doc.
//!
//! Both sides describe the same REST surface; if they ever disagree, one of
//! them is wrong. These tests fail loudly so the discrepancy is caught at
//! `cargo test` time instead of slipping into the generated `openapi.json`
//! that ships in `book/src/multiplayer/openapi.json`.
//!
//! Compiled only with the `openapi` cargo feature on `open4x-server`. The
//! file is harmless on default builds because the whole module is `cfg`-gated.

#![cfg(feature = "openapi")]

use std::collections::BTreeSet;

use open4x_server::server::openapi;

/// Axum uses `{name}` for path params. The OpenAPI document uses the same
/// syntax, so we can compare them directly without translation.
fn router_path_strings() -> BTreeSet<(String, String)> {
    // We don't have direct access to Axum's route table at runtime (Router
    // doesn't expose its `path_router`), so we mirror the table from
    // `rest::mod::v1_router` here. The single-source-of-truth file is
    // `open4x-server/src/server/rest/mod.rs`; updates must land in both.
    //
    // This list intentionally lives next to the doc-contract test instead of
    // being introspected from the live router — Axum's typed `MethodRouter`
    // makes introspection awkward, and a static list is enough to catch
    // accidental drift.
    let pairs = [
        ("get",    "/api/v1/health"),
        ("post",   "/api/v1/games/new"),
        ("get",    "/api/v1/player-state"),
        ("get",    "/api/v1/world/snapshot"),
        ("get",    "/api/v1/world/tile/{q}/{r}"),
        ("get",    "/api/v1/map/overlays"),
        ("get",    "/api/v1/cities"),
        ("get",    "/api/v1/cities/{id}"),
        ("get",    "/api/v1/cities/{id}/tiles"),
        ("post",   "/api/v1/cities/{id}/production"),
        ("delete", "/api/v1/cities/{id}/production/{pos}"),
        ("post",   "/api/v1/cities/{id}/focus"),
        ("post",   "/api/v1/cities/{id}/rename"),
        ("get",    "/api/v1/units"),
        ("get",    "/api/v1/units/{id}"),
        ("post",   "/api/v1/units/{id}/action"),
        ("get",    "/api/v1/armies"),
        ("get",    "/api/v1/combat/preview"),
        ("get",    "/api/v1/tech"),
        ("post",   "/api/v1/tech/research"),
        ("delete", "/api/v1/tech/research"),
        ("get",    "/api/v1/civics"),
        ("post",   "/api/v1/civics/research"),
        ("delete", "/api/v1/civics/research"),
        ("get",    "/api/v1/government"),
        ("post",   "/api/v1/government/change"),
        ("get",    "/api/v1/diplomacy"),
        ("get",    "/api/v1/diplomacy/civs/{id}"),
        ("get",    "/api/v1/empire/overview"),
        ("get",    "/api/v1/victory"),
        ("get",    "/api/v1/notifications"),
        ("delete", "/api/v1/notifications"),
        ("delete", "/api/v1/notifications/{id}"),
        ("get",    "/api/v1/turn-queue"),
        ("post",   "/api/v1/turn/end"),
        ("get",    "/api/v1/registry"),
    ];
    pairs.into_iter().map(|(m, p)| (m.to_string(), p.to_string())).collect()
}

fn openapi_operation_set() -> BTreeSet<(String, String)> {
    let doc = openapi::document();
    let mut out = BTreeSet::new();
    for (path, item) in doc.paths.paths.iter() {
        // PathItem has one Option<Operation> per HTTP method; enumerate them.
        for (method, op) in [
            ("get",     item.get.as_ref()),
            ("put",     item.put.as_ref()),
            ("post",    item.post.as_ref()),
            ("delete",  item.delete.as_ref()),
            ("patch",   item.patch.as_ref()),
            ("head",    item.head.as_ref()),
            ("options", item.options.as_ref()),
            ("trace",   item.trace.as_ref()),
        ] {
            if op.is_some() {
                out.insert((method.to_string(), path.clone()));
            }
        }
    }
    out
}

#[test]
fn openapi_paths_match_router() {
    let router = router_path_strings();
    let docs = openapi_operation_set();

    let missing_from_docs: Vec<_> = router.difference(&docs).collect();
    let missing_from_router: Vec<_> = docs.difference(&router).collect();

    assert!(
        missing_from_docs.is_empty() && missing_from_router.is_empty(),
        "OpenAPI / router drift.\n\
         Routes in v1_router but missing from openapi.rs: {missing_from_docs:#?}\n\
         Operations in openapi.rs but missing from v1_router: {missing_from_router:#?}\n\
         Source of truth: open4x-server/src/server/rest/mod.rs::v1_router\n\
         OpenAPI:       open4x-server/src/server/openapi.rs",
    );
}

#[test]
fn declared_paths_match_router() {
    // `openapi::declared_paths` is a hand-maintained mirror used by external
    // tooling (e.g. SDK generators). It must match the router too.
    let declared: BTreeSet<(String, String)> = openapi::declared_paths()
        .into_iter()
        .map(|(m, p)| (m.to_string(), p.to_string()))
        .collect();
    let router = router_path_strings();

    assert_eq!(
        declared,
        router,
        "declared_paths() out of sync with v1_router. Update \
         open4x-server/src/server/openapi.rs::declared_paths."
    );
}

#[test]
fn openapi_document_is_serializable() {
    // Smoke test: gen-openapi relies on this. If `to_pretty_json` ever fails
    // (e.g. a generic `MutationResponse<T>` slipped in without an alias) we
    // want the failure here rather than at release-build time.
    let doc = openapi::document();
    let json = doc.to_pretty_json().expect("OpenAPI must serialize");
    assert!(json.contains("\"openapi\""));
    assert!(json.contains("\"/api/v1/health\""));
}
