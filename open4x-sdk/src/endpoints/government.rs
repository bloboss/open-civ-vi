//! `/api/v1/government*` — current government + active policies, plus the
//! switch mutation.

use serde::Serialize;

use open4x_protocol::v1::web::MutationResponse;
use open4x_protocol::v1::web::government::GovernmentPolicies;

use crate::error::ApiError;
use crate::transport::{Method, Transport};

pub async fn get<T: Transport>(t: &T) -> Result<GovernmentPolicies, ApiError> {
    let body = t.request(Method::Get, "/api/v1/government", None).await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangeGovernmentBody {
    /// Government name as it appears in the registry (case-sensitive),
    /// e.g. `"Chiefdom"`, `"Monarchy"`.
    pub government: String,
}

/// `POST /api/v1/government/change` — switch to a (previously unlocked)
/// government. Rejects unknown or locked names with a structured 400.
pub async fn change<T: Transport>(
    t: &T,
    body: &ChangeGovernmentBody,
) -> Result<MutationResponse<GovernmentPolicies>, ApiError> {
    let bytes = serde_json::to_vec(body).map_err(|e| ApiError::transport(e.to_string()))?;
    let resp = t
        .request(Method::Post, "/api/v1/government/change", Some(&bytes))
        .await?;
    serde_json::from_slice(&resp).map_err(|e| ApiError::transport(e.to_string()))
}
