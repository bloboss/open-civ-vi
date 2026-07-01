//! `/api/v1/units/*` — unit list, detail, and the action dispatcher.

use serde::Serialize;

use open4x_protocol::v1::web::MutationResponse;
use open4x_protocol::v1::web::unit_data::{Unit, UnitData};

use crate::error::ApiError;
use crate::transport::{Method, Transport};

pub async fn list<T: Transport>(t: &T) -> Result<UnitData, ApiError> {
    let body = t.request(Method::Get, "/api/v1/units", None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

pub async fn detail<T: Transport>(t: &T, id: &str) -> Result<Unit, ApiError> {
    let url = format!("/api/v1/units/{id}");
    let body = t.request(Method::Get, &url, None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UnitActionBody {
    /// `"move" | "attack" | "fortify" | "sleep" | "found_city"`.
    pub action_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_q: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_r: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `POST /api/v1/units/{id}/action` — dispatch a unit action. The `view`
/// field of the response is the updated `Unit` for actions that touch the
/// unit, or `null` for the fortify/sleep stub.
pub async fn action<T: Transport>(
    t: &T,
    id: &str,
    body: &UnitActionBody,
) -> Result<MutationResponse<serde_json::Value>, ApiError> {
    let url = format!("/api/v1/units/{id}/action");
    let bytes = serde_json::to_vec(body).map_err(|e| ApiError::transport(e.to_string()))?;
    let resp = t.request(Method::Post, &url, Some(&bytes)).await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}
