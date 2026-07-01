//! `/api/v1/notifications` — list + dismiss-one + dismiss-all.

use open4x_protocol::v1::web::notifications::Notifications;

use crate::error::ApiError;
use crate::transport::{Method, Transport};

pub async fn list<T: Transport>(t: &T) -> Result<Notifications, ApiError> {
    let body = t
        .request(Method::Get, "/api/v1/notifications", None)
        .await?;
    serde_json::from_slice(&body).map_err(|e| ApiError::transport(e.to_string()))
}

/// `DELETE /api/v1/notifications/{id}` — server returns 204 with empty
/// body; the SDK normalises that to `Ok(())`.
pub async fn dismiss<T: Transport>(t: &T, id: &str) -> Result<(), ApiError> {
    let url = format!("/api/v1/notifications/{id}");
    t.request(Method::Delete, &url, None).await.map(|_| ())
}

/// `DELETE /api/v1/notifications` — dismiss every notification.
pub async fn dismiss_all<T: Transport>(t: &T) -> Result<(), ApiError> {
    t.request(Method::Delete, "/api/v1/notifications", None)
        .await
        .map(|_| ())
}
