//! `GET /api/v1/empire/overview` — top-level empire dashboard.

use open4x_protocol::v1::web::empire_overview::EmpireOverview;

use crate::error::ApiError;
use crate::transport::{Method, Transport};

pub async fn overview<T: Transport>(t: &T) -> Result<EmpireOverview, ApiError> {
    let body = t
        .request(Method::Get, "/api/v1/empire/overview", None)
        .await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}
