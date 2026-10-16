//! Consent records: list and append.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extract::{ApiJson, Caller};
use crate::domain::{ConsentScope, Role};
use crate::error::{ApiError, ApiResult};
use crate::infra::audit::Outcome;
use crate::state::AppState;
use crate::time::to_ms;

/// A consent record (OpenAPI `Consent`).
#[derive(Debug, Serialize)]
pub struct ConsentDto {
    /// Record id.
    pub id: Uuid,
    /// Scope.
    pub scope: ConsentScope,
    /// Granted or withdrawn.
    pub granted: bool,
    /// Version of the form shown.
    pub form_version: String,
    /// When it was recorded, epoch ms.
    pub recorded_at_unix_ms: i64,
}

/// Body of `POST /consents`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentCreate {
    /// Scope.
    pub scope: ConsentScope,
    /// Granted or withdrawn.
    pub granted: bool,
    /// Version of the form shown, at most 32 characters.
    pub form_version: String,
}

type ConsentRow = (Uuid, String, bool, String, DateTime<Utc>);

fn dto((id, scope, granted, form_version, at): ConsentRow) -> Option<ConsentDto> {
    Some(ConsentDto {
        id,
        scope: ConsentScope::parse(&scope)?,
        granted,
        form_version,
        recorded_at_unix_ms: to_ms(at),
    })
}

/// `GET /consents`: the caller's records, or every record for clinicians.
///
/// # Errors
/// `403` for admins.
pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
) -> ApiResult<Json<Vec<ConsentDto>>> {
    caller.require(&[Role::Patient, Role::Clinician])?;
    let only = (caller.role == Role::Patient).then_some(caller.id);
    let rows: Vec<ConsentRow> = sqlx::query_as(
        "SELECT id, scope, granted, form_version, recorded_at FROM consents
         WHERE ($1::uuid IS NULL OR subject_user_id = $1) ORDER BY recorded_at DESC LIMIT 4096",
    )
    .bind(only)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows.into_iter().filter_map(dto).collect()))
}

/// `POST /consents`: appends a decision of the caller.
///
/// # Errors
/// `400` for a long form version.
pub async fn record(
    State(state): State<AppState>,
    caller: Caller,
    ApiJson(body): ApiJson<ConsentCreate>,
) -> ApiResult<impl IntoResponse> {
    if body.form_version.is_empty() || body.form_version.len() > 32 {
        return Err(ApiError::invalid(
            "form_version",
            "must be 1 to 32 characters",
        ));
    }
    let row: ConsentRow = sqlx::query_as(
        "INSERT INTO consents (id, subject_user_id, scope, granted, form_version, recorded_by)
         VALUES ($1, $2, $3, $4, $5, $2) RETURNING id, scope, granted, form_version, recorded_at",
    )
    .bind(Uuid::new_v4())
    .bind(caller.id)
    .bind(body.scope.as_str())
    .bind(body.granted)
    .bind(&body.form_version)
    .fetch_one(&state.db)
    .await?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "consent.record",
            body.scope.as_str(),
            Outcome::Allowed,
        )
        .await;
    let dto = dto(row).ok_or_else(ApiError::internal)?;
    Ok((StatusCode::CREATED, Json(dto)))
}
