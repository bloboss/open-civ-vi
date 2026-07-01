//! Bindings for `/api/v1/presets`.

use serde::{Deserialize, Serialize};

use super::http::{ApiError, fetch_json};

#[derive(Debug, Clone, Deserialize)]
pub struct PresetView {
    pub id: String,
    pub name: String,
    pub body_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ListResp {
    presets: Vec<PresetView>,
}

pub async fn list() -> Result<Vec<PresetView>, ApiError> {
    let r: ListResp = fetch_json::<ListResp, ()>("GET", "/api/v1/presets", None).await?;
    Ok(r.presets)
}

#[derive(Debug, Serialize)]
struct CreateBody {
    name: String,
    body_json: String,
}

pub async fn create(name: String, body_json: String) -> Result<PresetView, ApiError> {
    let body = CreateBody { name, body_json };
    fetch_json::<PresetView, CreateBody>("POST", "/api/v1/presets", Some(&body)).await
}

pub async fn delete_preset(id: &str) -> Result<(), ApiError> {
    let url = format!("/api/v1/presets/{id}");
    fetch_json::<serde_json::Value, ()>("DELETE", &url, None)
        .await
        .map(|_| ())
}
