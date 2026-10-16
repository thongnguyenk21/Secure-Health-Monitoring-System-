//! Alerts: list and acknowledge.

use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extract::{ApiQuery, Caller, PageParams, encode_cursor, split_cursor};
use super::no_store;
use crate::alerts::model::{ALERT_COLUMNS, AlertDto, AlertRow, LiveEventType};
use crate::domain::{AlertStatus, Role};
use crate::error::{ApiError, ApiResult};
use crate::infra::audit::Outcome;
use crate::state::AppState;
use crate::time::from_ms;

/// Query of `GET /alerts`.
#[derive(Debug, Deserialize)]
pub struct AlertQuery {
    /// Filter by status.
    pub status: Option<AlertStatus>,
    /// Filter by device.
    pub device_id: Option<String>,
    /// Paging.
    #[serde(flatten)]
    pub page: PageParams,
}

/// One page of alerts.
#[derive(Debug, Serialize)]
pub struct AlertPage {
    /// Alerts, newest first.
    pub items: Vec<AlertDto>,
    /// Cursor of the next page.
    pub next_cursor: Option<String>,
}

/// `GET /alerts`: newest first; patients see their own devices only.
///
/// # Errors
/// `403` for admins, `400` for bad paging.
pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
    ApiQuery(q): ApiQuery<AlertQuery>,
) -> ApiResult<Response> {
    caller.require(&[Role::Patient, Role::Clinician])?;
    let limit = q.page.limit()?;
    let (after_ts, after_id) = match q.page.cursor()?.map(|c| split_cursor(&c)).transpose()? {
        Some((ms, id)) => (from_ms(ms), Some(id)),
        None => (None, None),
    };
    let only_user = (caller.role == Role::Patient).then_some(caller.id);
    let rows: Vec<AlertRow> = sqlx::query_as(&format!(
        "SELECT {ALERT_COLUMNS} FROM alerts a
         WHERE ($1::text IS NULL OR a.status = $1)
           AND ($2::text IS NULL OR a.device_id = $2)
           AND ($3::uuid IS NULL OR a.device_id IN (SELECT device_id FROM devices WHERE assigned_user_id = $3))
           AND ($4::timestamptz IS NULL OR (a.raised_at, a.id) < ($4, $5))
         ORDER BY a.raised_at DESC, a.id DESC LIMIT $6"
    ))
    .bind(q.status.map(AlertStatus::as_str))
    .bind(&q.device_id)
    .bind(only_user)
    .bind(after_ts)
    .bind(after_id)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let more = rows.len() as i64 > limit;
    let items: Vec<AlertDto> = rows
        .iter()
        .take(usize::try_from(limit).unwrap_or(100))
        .filter_map(AlertRow::to_dto)
        .collect();
    let next_cursor = more
        .then(|| {
            items
                .last()
                .map(|a| encode_cursor(&format!("{}|{}", a.raised_at_unix_ms, a.id)))
        })
        .flatten();
    Ok(no_store(
        Json(AlertPage { items, next_cursor }).into_response(),
    ))
}

/// `POST /alerts/{alert_id}/ack`: stops escalation; idempotent.
///
/// # Errors
/// `404` when the alert does not exist or is not visible to the caller.
pub async fn acknowledge(
    State(state): State<AppState>,
    caller: Caller,
    Path(alert_id): Path<Uuid>,
) -> ApiResult<Json<AlertDto>> {
    if caller.role == Role::Admin {
        return Err(ApiError::not_found());
    }
    let only_user = (caller.role == Role::Patient).then_some(caller.id);
    let changed: Option<AlertRow> = sqlx::query_as(&format!(
        "UPDATE alerts SET status = 'acknowledged', acknowledged_at = now(), acknowledged_by = $2, updated_at = now()
         WHERE id = $1 AND status = 'open'
           AND ($3::uuid IS NULL OR device_id IN (SELECT device_id FROM devices WHERE assigned_user_id = $3))
         RETURNING {ALERT_COLUMNS}"
    ))
    .bind(alert_id)
    .bind(caller.id)
    .bind(only_user)
    .fetch_optional(&state.db)
    .await?;
    let row = match changed {
        Some(row) => {
            state.engine.publish(LiveEventType::AlertUpdated, &row);
            row
        }
        None => sqlx::query_as(&format!(
            "SELECT {ALERT_COLUMNS} FROM alerts
             WHERE id = $1 AND ($2::uuid IS NULL OR device_id IN (SELECT device_id FROM devices WHERE assigned_user_id = $2))"
        ))
        .bind(alert_id)
        .bind(only_user)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(ApiError::not_found)?,
    };
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "alert.ack",
            &alert_id.to_string(),
            Outcome::Allowed,
        )
        .await;
    Ok(Json(row.to_dto().ok_or_else(ApiError::internal)?))
}
