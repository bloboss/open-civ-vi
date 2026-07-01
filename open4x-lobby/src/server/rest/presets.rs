//! `/api/v1/presets` — list / create / delete user-saved wizard
//! presets.

#![cfg(feature = "ssr")]

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use open4x_accounts::presets::{PresetRow, PresetsError, PresetsStore};
use serde::{Deserialize, Serialize};

use crate::server::AppState;
use crate::server::auth::RequireSession;

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PresetView {
    pub id: String,
    pub name: String,
    pub body_json: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PresetRow> for PresetView {
    fn from(r: PresetRow) -> Self {
        Self {
            id: r.id,
            name: r.name,
            body_json: r.body_json,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListResp {
    pub presets: Vec<PresetView>,
}

pub async fn list(State(state): State<AppState>, RequireSession(me): RequireSession) -> Response {
    match state.presets.list_for(me).await {
        Ok(rows) => Json(ListResp {
            presets: rows.into_iter().map(PresetView::from).collect(),
        })
        .into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateBody {
    pub name: String,
    /// Free-form JSON serialized to a string. The lobby validates
    /// it parses; everything else is opaque.
    pub body_json: String,
}

pub async fn create(
    State(state): State<AppState>,
    RequireSession(me): RequireSession,
    Json(body): Json<CreateBody>,
) -> Response {
    match state.presets.create(me, &body.name, &body.body_json).await {
        Ok(row) => (StatusCode::CREATED, Json(PresetView::from(row))).into_response(),
        Err(e) => err_response(e),
    }
}

pub async fn delete_one(
    State(state): State<AppState>,
    RequireSession(me): RequireSession,
    Path(id): Path<String>,
) -> Response {
    match state.presets.delete(me, &id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => err_response(e),
    }
}

fn err_response(e: PresetsError) -> Response {
    let (status, code) = match &e {
        PresetsError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
        PresetsError::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid_input"),
        PresetsError::Sqlx(_) => (StatusCode::INTERNAL_SERVER_ERROR, "store_error"),
    };
    (
        status,
        Json(ErrorBody {
            error: code,
            message: Some(e.to_string()),
        }),
    )
        .into_response()
}
