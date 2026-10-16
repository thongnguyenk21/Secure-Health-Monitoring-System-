//! `GET /system/status` (admin): component states for the admin dashboard.

use axum::Json;
use axum::extract::State;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::extract::Caller;
use crate::domain::Role;
use crate::error::ApiResult;
use crate::state::AppState;
use crate::time::{now_ms, to_ms};

/// State of one component (OpenAPI `ComponentStatus`).
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ComponentStatus {
    /// Component name.
    pub name: String,
    /// `ok`, `degraded`, `down` or `unknown`.
    pub state: &'static str,
    /// Reason when not ok.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Grades a component by the age of its last activity.
///
/// Within 2 minutes is ok, within 10 minutes degraded, older is down, never
/// seen is unknown.
#[must_use]
pub fn grade(last_ms: Option<i64>, now_ms: i64) -> &'static str {
    match last_ms.map(|t| now_ms - t) {
        None => "unknown",
        Some(age) if age <= 2 * 60_000 => "ok",
        Some(age) if age <= 10 * 60_000 => "degraded",
        Some(_) => "down",
    }
}

/// `GET /system/status`.
///
/// # Errors
/// `403` for non-admins.
pub async fn status(
    State(state): State<AppState>,
    caller: Caller,
) -> ApiResult<Json<Vec<ComponentStatus>>> {
    caller.require(&[Role::Admin])?;
    let mut out = vec![ComponentStatus {
        name: "Backend API".into(),
        state: "ok",
        detail: None,
    }];
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();
    out.push(ComponentStatus {
        name: "TimescaleDB".into(),
        state: if db_ok { "ok" } else { "down" },
        detail: (!db_ok).then(|| "no answer".into()),
    });
    if db_ok {
        let newest: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT max(last_seen) FROM devices WHERE kind = 'wearable'")
                .fetch_one(&state.db)
                .await?;
        let s = grade(newest.map(to_ms), now_ms());
        out.push(ComponentStatus {
            name: "Telemetry ingest".into(),
            state: s,
            detail: (s != "ok").then(|| "no recent upload".into()),
        });
        let gateways: Vec<(String, Option<String>, Option<DateTime<Utc>>)> = sqlx::query_as(
            "SELECT device_id, label, last_seen FROM devices WHERE kind = 'gateway' AND status <> 'revoked' ORDER BY label",
        )
        .fetch_all(&state.db)
        .await?;
        for (id, label, last) in gateways {
            let s = grade(last.map(to_ms), now_ms());
            out.push(ComponentStatus {
                name: label.unwrap_or_else(|| format!("Gateway {}", &id[..8])),
                state: s,
                detail: (s != "ok").then(|| "slow or no sync".into()),
            });
        }
    }
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades_follow_the_age() {
        let now = 1_000_000_000;
        assert_eq!(grade(None, now), "unknown");
        assert_eq!(grade(Some(now - 1000), now), "ok");
        assert_eq!(grade(Some(now - 5 * 60_000), now), "degraded");
        assert_eq!(grade(Some(now - 60 * 60_000), now), "down");
    }
}
