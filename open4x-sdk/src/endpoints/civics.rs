//! `/api/v1/civics*` — culture tree read + queue mutate.

use serde::Serialize;

use open4x_protocol::v1::web::MutationResponse;
use open4x_protocol::v1::web::civics_tree::CivicsTreeView;

use crate::error::ApiError;
use crate::transport::{Method, Transport};

pub async fn get<T: Transport>(t: &T) -> Result<CivicsTreeView, ApiError> {
    let body = t.request(Method::Get, "/api/v1/civics", None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

#[derive(Debug, Clone, Serialize)]
pub struct CivicResearchBody {
    pub civic_id: String,
}

/// `POST /api/v1/civics/research` — queue a civic.
pub async fn research<T: Transport>(
    t: &T,
    body: &CivicResearchBody,
) -> Result<MutationResponse<CivicsTreeView>, ApiError> {
    let bytes = serde_json::to_vec(body).map_err(|e| ApiError::transport(e.to_string()))?;
    let resp = t
        .request(Method::Post, "/api/v1/civics/research", Some(&bytes))
        .await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}

/// `DELETE /api/v1/civics/research` — clear the active civic slot.
/// Idempotent.
pub async fn cancel<T: Transport>(t: &T) -> Result<MutationResponse<CivicsTreeView>, ApiError> {
    let resp = t
        .request(Method::Delete, "/api/v1/civics/research", None)
        .await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}
