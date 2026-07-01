//! `status <kind>` over REST. Each arm forwards the relevant
//! `/api/v1/*` endpoint's JSON verbatim. Where the local handler
//! emits a richer-but-different shape, the divergence is documented
//! in the parity matrix in `book/src/roadmap/cli-server-mode.md`.

use std::path::Path;

use crate::cli::StatusKind;
use crate::handlers::parse_ulid;

use super::client::ApiClient;
use super::session::Session;

pub fn status(server: &str, token_file: &Path, kind: &StatusKind) -> Result<(), String> {
    let session = Session::load(token_file)?;
    let client = ApiClient::new(server).with_token(session.token);

    let value = match kind {
        StatusKind::Yields => client.get_json("/api/v1/player-state")?,
        StatusKind::Pending => client.get_json("/api/v1/turn-queue")?,
        StatusKind::Victory => client.get_json("/api/v1/victory")?,
        StatusKind::Policies => client.get_json("/api/v1/government")?,
        StatusKind::Techs => client.get_json("/api/v1/tech")?,
        StatusKind::Civics => client.get_json("/api/v1/civics")?,
        StatusKind::Diplomacy => client.get_json("/api/v1/diplomacy")?,
        StatusKind::City { id } => {
            let raw = parse_ulid(id)?.to_string();
            client.get_json(&format!("/api/v1/cities/{raw}"))?
        }
        StatusKind::Unit { id } => {
            let raw = parse_ulid(id)?.to_string();
            client.get_json(&format!("/api/v1/units/{raw}"))?
        }
        StatusKind::Tile { q, r } => client.get_json(&format!("/api/v1/world/tile/{q}/{r}"))?,
        StatusKind::UnitActions { id } => {
            // The server folds available actions into the unit detail
            // payload, so just print the `actions` slice if present.
            let raw = parse_ulid(id)?.to_string();
            let body = client.get_json(&format!("/api/v1/units/{raw}"))?;
            body.get("actions").cloned().unwrap_or(body)
        }
        StatusKind::CombatPreview { attacker, q, r } => {
            let raw = parse_ulid(attacker)?.to_string();
            client.get_json(&format!(
                "/api/v1/combat/preview?attacker_id={raw}&defender_q={q}&defender_r={r}"
            ))?
        }

        // Not exposed over REST yet — fail loudly so parity tests
        // don't quietly skip them.
        StatusKind::Scores => {
            return Err(
                "server mode does not expose 'status scores' yet (no REST endpoint)".into(),
            );
        }
        StatusKind::Congress => {
            return Err(
                "server mode does not expose 'status congress' yet (no REST endpoint)".into(),
            );
        }
    };

    println!("{}", serde_json::to_string_pretty(&value).unwrap());
    Ok(())
}
