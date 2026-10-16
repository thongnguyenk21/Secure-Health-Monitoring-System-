//! Liveness and readiness probes (D25: no Prometheus, only `/healthz` and `/readyz`).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use crate::state::AppState;

/// `GET /healthz`: answers without touching any dependency.
pub async fn healthz() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

/// `GET /readyz`: checks the database within 2 seconds.
pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let check = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db),
    )
    .await;
    match check {
        Ok(Ok(_)) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready", "reason": "database" })),
        ),
    }
}
