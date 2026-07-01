//! `/api/v1/cities/*` — list/detail reads plus the production-queue, focus,
//! and rename mutations.

use serde::Serialize;

use open4x_protocol::v1::web::MutationResponse;
use open4x_protocol::v1::web::city_data::{CityData, CityRow};
use open4x_protocol::v1::web::city_tiles::CityTiles;

use crate::error::ApiError;
use crate::transport::{Method, Transport};

pub async fn list<T: Transport>(t: &T) -> Result<CityData, ApiError> {
    let body = t.request(Method::Get, "/api/v1/cities", None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

pub async fn detail<T: Transport>(t: &T, id: &str) -> Result<CityRow, ApiError> {
    let url = format!("/api/v1/cities/{id}");
    let body = t.request(Method::Get, &url, None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

pub async fn tiles<T: Transport>(t: &T, id: &str) -> Result<CityTiles, ApiError> {
    let url = format!("/api/v1/cities/{id}/tiles");
    let body = t.request(Method::Get, &url, None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

// ── mutations ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct QueueProductionBody {
    pub item_id: String,
    /// `"unit" | "building" | "wonder" | "project"`. (`"district"` is not
    /// supported over REST yet — see server handler.)
    pub item_type: String,
}

/// `POST /api/v1/cities/{id}/production` — append to the city's queue.
pub async fn queue_production<T: Transport>(
    t: &T,
    id: &str,
    body: &QueueProductionBody,
) -> Result<MutationResponse<CityRow>, ApiError> {
    let url = format!("/api/v1/cities/{id}/production");
    let bytes = serde_json::to_vec(body).map_err(|e| ApiError::transport(e.to_string()))?;
    let resp = t.request(Method::Post, &url, Some(&bytes)).await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}

/// `DELETE /api/v1/cities/{id}/production/{pos}` — remove queue entry.
pub async fn cancel_production<T: Transport>(
    t: &T,
    id: &str,
    pos: usize,
) -> Result<MutationResponse<CityRow>, ApiError> {
    let url = format!("/api/v1/cities/{id}/production/{pos}");
    let body = t.request(Method::Delete, &url, None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

#[derive(Debug, Clone, Serialize)]
pub struct AssignCityFocusBody {
    /// Lowercase: `"default" | "food" | "production" | "gold" | "science"
    /// | "culture" | "faith"`.
    pub focus: String,
}

/// `POST /api/v1/cities/{id}/focus` — set the city's production focus.
pub async fn assign_focus<T: Transport>(
    t: &T,
    id: &str,
    body: &AssignCityFocusBody,
) -> Result<MutationResponse<CityRow>, ApiError> {
    let url = format!("/api/v1/cities/{id}/focus");
    let bytes = serde_json::to_vec(body).map_err(|e| ApiError::transport(e.to_string()))?;
    let resp = t.request(Method::Post, &url, Some(&bytes)).await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}

#[derive(Debug, Clone, Serialize)]
pub struct RenameCityBody {
    pub name: String,
}

/// `POST /api/v1/cities/{id}/rename` — replace `City.name` (1..=64 chars).
pub async fn rename<T: Transport>(
    t: &T,
    id: &str,
    body: &RenameCityBody,
) -> Result<MutationResponse<CityRow>, ApiError> {
    let url = format!("/api/v1/cities/{id}/rename");
    let bytes = serde_json::to_vec(body).map_err(|e| ApiError::transport(e.to_string()))?;
    let resp = t.request(Method::Post, &url, Some(&bytes)).await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}
