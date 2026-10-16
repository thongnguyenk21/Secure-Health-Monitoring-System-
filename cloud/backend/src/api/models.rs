//! Signed model versions: list, install target, roll back.
//!
//! Upload and signing happen in CI (`docs/runbooks`); this API only chooses
//! which registered version gateways should run.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::extract::Caller;
use crate::domain::Role;
use crate::error::{ApiError, ApiResult};
use crate::infra::audit::Outcome;
use crate::state::AppState;
use crate::time::to_ms;

/// A model version (OpenAPI `Model`).
#[derive(Debug, Serialize)]
pub struct ModelDto {
    /// Semantic version.
    pub version: String,
    /// SHA-256 of the artifact, hex.
    pub sha256: String,
    /// Lowest firmware it supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_fw: Option<String>,
    /// Registration time, epoch ms.
    pub created_at_unix_ms: i64,
    /// Whether gateways should run it.
    pub is_target: bool,
    /// Release notes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

type ModelRow = (
    String,
    String,
    Option<String>,
    DateTime<Utc>,
    bool,
    Option<String>,
);

const MODEL_COLUMNS: &str = "version, sha256, min_fw, created_at, is_target, notes";

fn dto((version, sha256, min_fw, created_at, is_target, notes): ModelRow) -> ModelDto {
    ModelDto {
        version,
        sha256,
        min_fw,
        created_at_unix_ms: to_ms(created_at),
        is_target,
        notes,
    }
}

/// `GET /models` (clinician, admin), newest first.
///
/// # Errors
/// `403` for patients.
pub async fn list(State(state): State<AppState>, caller: Caller) -> ApiResult<Json<Vec<ModelDto>>> {
    caller.require(&[Role::Clinician, Role::Admin])?;
    let rows: Vec<ModelRow> = sqlx::query_as(&format!(
        "SELECT {MODEL_COLUMNS} FROM models ORDER BY created_at DESC LIMIT 4096"
    ))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().map(dto).collect()))
}

async fn set_target(
    state: &AppState,
    caller: Caller,
    version: &str,
    action: &str,
) -> ApiResult<ModelDto> {
    if caller.role != Role::Admin {
        return Err(ApiError::not_found());
    }
    let mut tx = state.db.begin().await?;
    let exists: Option<String> =
        sqlx::query_scalar("SELECT version FROM models WHERE version = $1")
            .bind(version)
            .fetch_optional(&mut *tx)
            .await?;
    if exists.is_none() {
        return Err(ApiError::not_found());
    }
    sqlx::query("UPDATE models SET is_target = FALSE WHERE is_target")
        .execute(&mut *tx)
        .await?;
    let row: ModelRow = sqlx::query_as(&format!(
        "UPDATE models SET is_target = TRUE WHERE version = $1 RETURNING {MODEL_COLUMNS}"
    ))
    .bind(version)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            action,
            version,
            Outcome::Allowed,
        )
        .await;
    Ok(dto(row))
}

/// `POST /models/{version}/install` (admin): makes it the target.
///
/// # Errors
/// `404` for non-admins or unknown versions.
pub async fn install(
    State(state): State<AppState>,
    caller: Caller,
    Path(version): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let m = set_target(&state, caller, &version, "model.install").await?;
    Ok((StatusCode::ACCEPTED, Json(m)))
}

/// `POST /models/{version}/rollback` (admin): makes the previous version the target.
///
/// # Errors
/// `404` for non-admins, unknown versions or when there is no older version.
pub async fn rollback(
    State(state): State<AppState>,
    caller: Caller,
    Path(version): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let previous: Option<String> = sqlx::query_scalar(
        "SELECT version FROM models WHERE created_at < (SELECT created_at FROM models WHERE version = $1)
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&version)
    .fetch_optional(&state.db)
    .await?;
    let previous = previous.ok_or_else(ApiError::not_found)?;
    let m = set_target(&state, caller, &previous, "model.rollback").await?;
    Ok((StatusCode::ACCEPTED, Json(m)))
}
