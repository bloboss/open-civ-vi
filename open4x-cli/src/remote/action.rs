//! `action <kind>` over REST. Only the `ActionKind` variants that have
//! a current `/api/v1/*` mutation are implemented; everything else
//! returns a clean "not supported in server mode" error so parity
//! scripts surface gaps loudly.
//!
//! As new `GameAction` variants land in the server (tracked in
//! `book/src/roadmap/ongoing.md`), promote the corresponding arm out
//! of the `unsupported` group.

use std::path::Path;

use serde_json::{Value, json};

use crate::cli::ActionKind;
use crate::handlers::parse_ulid;

use super::client::ApiClient;
use super::session::Session;

pub fn action(server: &str, token_file: &Path, action: &ActionKind) -> Result<(), String> {
    let session = Session::load(token_file)?;
    let client = ApiClient::new(server).with_token(session.token);

    let value = match action {
        // ── Movement & combat ───────────────────────────────────────
        ActionKind::Move { unit, to_q, to_r } => {
            let uid = parse_ulid(unit)?.to_string();
            client.post_json(
                &format!("/api/v1/units/{uid}/action"),
                &json!({ "action_id": "move", "target_q": to_q, "target_r": to_r }),
            )?
        }
        ActionKind::Attack { unit, target } => {
            // The REST endpoint resolves the defender by coord, not
            // by ID — look up the target unit's tile first.
            let uid = parse_ulid(unit)?.to_string();
            let tid = parse_ulid(target)?.to_string();
            let target_unit = client.get_json(&format!("/api/v1/units/{tid}"))?;
            let (q, r) = extract_coord(&target_unit).ok_or_else(|| {
                format!("could not resolve coord for target unit {tid} from /units/{tid}")
            })?;
            client.post_json(
                &format!("/api/v1/units/{uid}/action"),
                &json!({ "action_id": "attack", "target_q": q, "target_r": r }),
            )?
        }
        ActionKind::FoundCity { unit, name } => {
            let uid = parse_ulid(unit)?.to_string();
            client.post_json(
                &format!("/api/v1/units/{uid}/action"),
                &json!({ "action_id": "found_city", "name": name }),
            )?
        }

        // ── Production ──────────────────────────────────────────────
        ActionKind::Build { city, item } => {
            let cid = parse_ulid(city)?.to_string();
            let (item_id, item_type) = resolve_buildable(&client, item)?;
            client.post_json(
                &format!("/api/v1/cities/{cid}/production"),
                &json!({ "item_id": item_id, "item_type": item_type }),
            )?
        }
        ActionKind::CancelProduction { city } => {
            let cid = parse_ulid(city)?.to_string();
            // Local handler cancels the *front* item; the REST shape
            // takes an explicit position, so we always send 0.
            client.delete_json(&format!("/api/v1/cities/{cid}/production/0"))?
        }

        // ── Tech & civics ───────────────────────────────────────────
        ActionKind::Research { tech } => {
            let tech_id = resolve_tree_id(&client, "/api/v1/tech", "techs", tech)?;
            client.post_json("/api/v1/tech/research", &json!({ "tech_id": tech_id }))?
        }
        ActionKind::CancelResearch => client.delete_json("/api/v1/tech/research")?,
        ActionKind::StudyCivic { civic } => {
            let civic_id = resolve_tree_id(&client, "/api/v1/civics", "civics", civic)?;
            client.post_json("/api/v1/civics/research", &json!({ "civic_id": civic_id }))?
        }
        ActionKind::CancelCivic => client.delete_json("/api/v1/civics/research")?,
        ActionKind::AdoptGovernment { name } => {
            client.post_json("/api/v1/government/change", &json!({ "government": name }))?
        }

        // ── City state ──────────────────────────────────────────────
        ActionKind::AssignCityFocus { city, focus } => {
            let cid = parse_ulid(city)?.to_string();
            client.post_json(
                &format!("/api/v1/cities/{cid}/focus"),
                &json!({ "focus": focus }),
            )?
        }
        ActionKind::RenameCity { city, name } => {
            let cid = parse_ulid(city)?.to_string();
            client.post_json(
                &format!("/api/v1/cities/{cid}/rename"),
                &json!({ "name": name }),
            )?
        }

        other => {
            return Err(format!(
                "server mode does not yet support action {other:?} (no REST mutation). \
                 Track the queue at book/src/roadmap/ongoing.md."
            ));
        }
    };

    println!("{}", serde_json::to_string_pretty(&value).unwrap());
    Ok(())
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn extract_coord(unit: &Value) -> Option<(i32, i32)> {
    // Server unit row exposes coord as either {q,r} or "q,r" depending
    // on the projection — try the documented `coord` object first,
    // then fall back to flat top-level fields.
    if let Some(c) = unit.get("coord")
        && let (Some(q), Some(r)) = (
            c.get("q").and_then(Value::as_i64),
            c.get("r").and_then(Value::as_i64),
        )
    {
        return Some((q as i32, r as i32));
    }
    if let (Some(q), Some(r)) = (
        unit.get("q").and_then(Value::as_i64),
        unit.get("r").and_then(Value::as_i64),
    ) {
        return Some((q as i32, r as i32));
    }
    None
}

/// Walk `/api/v1/registry` to map a buildable's friendly name to a
/// `(item_id, item_type)` pair the REST endpoint understands.
fn resolve_buildable(client: &ApiClient, name: &str) -> Result<(String, String), String> {
    let registry = client.get_json("/api/v1/registry")?;
    let needle = name.to_ascii_lowercase();

    for (key, item_type) in [
        ("unit_types", "unit"),
        ("buildings", "building"),
        ("wonders", "wonder"),
        ("projects", "project"),
    ] {
        let Some(arr) = registry.get(key).and_then(|v| v.as_array()) else {
            continue;
        };
        for entry in arr {
            let entry_name = entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if entry_name.to_ascii_lowercase() == needle {
                let id = entry
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("registry entry for '{name}' missing id"))?;
                return Ok((id.to_string(), item_type.to_string()));
            }
        }
    }
    Err(format!(
        "no buildable '{name}' in /api/v1/registry (unit_types/buildings/wonders/projects)"
    ))
}

/// Walk a tech / civic tree response to map a node's name to its raw
/// ULID. Used by `Research` and `StudyCivic`.
fn resolve_tree_id(
    client: &ApiClient,
    path: &str,
    array_key: &str,
    name: &str,
) -> Result<String, String> {
    let body = client.get_json(path)?;
    let needle = name.to_ascii_lowercase();
    let arr = body
        .get(array_key)
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("{path} response missing '{array_key}' array"))?;
    for node in arr {
        let n = node.get("name").and_then(Value::as_str).unwrap_or_default();
        if n.to_ascii_lowercase() == needle {
            return node
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("{path} entry for '{name}' missing id"));
        }
    }
    Err(format!("no '{array_key}' entry named '{name}' in {path}"))
}
