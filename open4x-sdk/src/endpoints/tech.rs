//! `/api/v1/tech*` — research tree read + queue mutate.

use serde::Serialize;

use open4x_protocol::v1::web::MutationResponse;
use open4x_protocol::v1::web::tech_tree::TechTreeView;

use crate::error::ApiError;
use crate::transport::{Method, Transport};

pub async fn get<T: Transport>(t: &T) -> Result<TechTreeView, ApiError> {
    let body = t.request(Method::Get, "/api/v1/tech", None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

#[derive(Debug, Clone, Serialize)]
pub struct TechResearchBody {
    pub tech_id: String,
}

/// `POST /api/v1/tech/research` — push a tech onto the research queue.
pub async fn research<T: Transport>(
    t: &T,
    body: &TechResearchBody,
) -> Result<MutationResponse<TechTreeView>, ApiError> {
    let bytes = serde_json::to_vec(body).map_err(|e| ApiError::transport(e.to_string()))?;
    let resp = t
        .request(Method::Post, "/api/v1/tech/research", Some(&bytes))
        .await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}

/// `DELETE /api/v1/tech/research` — drop the active (front) tech.
/// Idempotent: a no-op on an empty queue still returns 200.
pub async fn cancel<T: Transport>(t: &T) -> Result<MutationResponse<TechTreeView>, ApiError> {
    let resp = t
        .request(Method::Delete, "/api/v1/tech/research", None)
        .await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}
