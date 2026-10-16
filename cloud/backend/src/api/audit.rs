//! `GET /audit` (admin); the query itself is audited.

use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extract::{ApiQuery, Caller, PageParams, encode_cursor};
use crate::domain::Role;
use crate::error::{ApiError, ApiResult};
use crate::infra::audit::Outcome;
use crate::state::AppState;
use crate::time::{from_ms, to_ms};

/// Query of `GET /audit`.
#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    /// Start of the range, inclusive, epoch ms.
    pub from_unix_ms: Option<i64>,
    /// Filter by action.
    pub action: Option<String>,
    /// Paging.
    #[serde(flatten)]
    pub page: PageParams,
}

type AuditRow = (i64, DateTime<Utc>, Option<Uuid>, String, String, String);

/// One audit entry (OpenAPI `AuditEntry`).
#[derive(Debug, Serialize)]
pub struct AuditEntry {
    /// Sequence number.
    pub id: i64,
    /// When, epoch ms.
    pub at_unix_ms: i64,
    /// Who.
    pub actor_id: Option<Uuid>,
    /// What.
    pub action: String,
    /// On what.
    pub target: String,
    /// Allowed or denied.
    pub outcome: String,
}

/// One page of entries, newest first.
#[derive(Debug, Serialize)]
pub struct AuditPage {
    /// Entries.
    pub items: Vec<AuditEntry>,
    /// Cursor of the next page.
    pub next_cursor: Option<String>,
}

/// `GET /audit`.
///
/// # Errors
/// `403` for non-admins, `400` for bad paging.
pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
    ApiQuery(q): ApiQuery<AuditQuery>,
) -> ApiResult<Json<AuditPage>> {
    if caller.require(&[Role::Admin]).is_err() {
        state
            .audit
            .record(
                &state.db,
                Some(caller.id),
                "audit.read",
                "",
                Outcome::Denied,
            )
            .await;
        return Err(ApiError::forbidden());
    }
    let limit = q.page.limit()?;
    let before = q
        .page
        .cursor()?
        .map(|c| {
            c.parse::<i64>()
                .map_err(|_| ApiError::invalid("cursor", "not a valid cursor"))
        })
        .transpose()?;
    if q.action.as_ref().is_some_and(|a| a.len() > 64) {
        return Err(ApiError::invalid("action", "at most 64 characters"));
    }
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "audit.read",
            "",
            Outcome::Allowed,
        )
        .await;
    let rows: Vec<AuditRow> = sqlx::query_as(
        "SELECT id, at, actor_id, action, target, outcome FROM audit_log
         WHERE ($1::timestamptz IS NULL OR at >= $1) AND ($2::text IS NULL OR action = $2)
           AND ($3::int8 IS NULL OR id < $3)
         ORDER BY id DESC LIMIT $4",
    )
    .bind(q.from_unix_ms.and_then(from_ms))
    .bind(&q.action)
    .bind(before)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let more = rows.len() as i64 > limit;
    let items: Vec<AuditEntry> = rows
        .into_iter()
        .take(usize::try_from(limit).unwrap_or(100))
        .map(|(id, at, actor_id, action, target, outcome)| AuditEntry {
            id,
            at_unix_ms: to_ms(at),
            actor_id,
            action,
            target,
            outcome,
        })
        .collect();
    let next_cursor = more
        .then(|| items.last().map(|e| encode_cursor(&e.id.to_string())))
        .flatten();
    Ok(Json(AuditPage { items, next_cursor }))
}
