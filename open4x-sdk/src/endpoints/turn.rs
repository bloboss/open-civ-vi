//! `/api/v1/turn*` — turn-queue read and end-of-turn mutation.

use serde::{Deserialize, Serialize};

use open4x_protocol::v1::web::MutationResponse;
use open4x_protocol::v1::web::turn_queue::TurnQueue;

use crate::error::ApiError;
use crate::transport::{Method, Transport};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EndTurnView {
    pub turn: u32,
}

#[derive(Serialize)]
struct Empty {}

/// `GET /api/v1/turn-queue` — pending-action list shown in the turn HUD.
pub async fn queue<T: Transport>(t: &T) -> Result<TurnQueue, ApiError> {
    let body = t.request(Method::Get, "/api/v1/turn-queue", None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

/// `POST /api/v1/turn/end` — single-player advance. Returns the resolved
/// turn number plus the standard mutation envelope.
///
/// Returns the structured error envelope (status 400, code
/// `"unresolved_required_actions"`) if the turn queue has any `required`
/// items.
pub async fn end<T: Transport>(t: &T) -> Result<MutationResponse<EndTurnView>, ApiError> {
    let bytes = serde_json::to_vec(&Empty {}).map_err(|e| ApiError::transport(e.to_string()))?;
    let body = t
        .request(Method::Post, "/api/v1/turn/end", Some(&bytes))
        .await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}
