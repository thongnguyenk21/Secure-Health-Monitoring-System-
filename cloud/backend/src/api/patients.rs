//! Clinician view of patients, ordered by priority, and clinical notes.

use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extract::{ApiJson, Caller};
use super::measurements::MeasurementDto;
use super::no_store;
use crate::alerts::engine::OFFLINE_AFTER_MS;
use crate::alerts::model::{ALERT_COLUMNS, AlertDto, AlertRow};
use crate::domain::{AlertStatus, AlertType, PatientStatus, Role, Severity};
use crate::error::{ApiError, ApiResult};
use crate::infra::audit::Outcome;
use crate::state::AppState;
use crate::time::{now_ms, to_ms};

/// One patient with the latest vitals (OpenAPI `PatientSummary`).
#[derive(Debug, Clone, Serialize)]
pub struct PatientSummary {
    /// User id.
    pub user_id: Uuid,
    /// Pseudonymous code, e.g. `BN-017`.
    pub subject_code: String,
    /// Display name.
    pub display_name: Option<String>,
    /// Assigned wearable.
    pub device_id: Option<String>,
    /// Priority level.
    pub status: PatientStatus,
    /// Number of open (unacknowledged) alerts; acknowledged ones still set the level.
    pub open_alert_count: usize,
    /// Most severe open alert, newest first among equals.
    pub top_alert: Option<AlertDto>,
    /// Latest summary vitals.
    pub latest: Option<MeasurementDto>,
    /// RMSSD of the latest window, ms.
    pub hrv_rmssd_ms: Option<f32>,
    /// Newest record time of the device, epoch ms.
    pub last_seen_unix_ms: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct PatientRow {
    id: Uuid,
    subject_code: Option<String>,
    display_name: Option<String>,
    device_id: Option<String>,
    last_seen: Option<DateTime<Utc>>,
    ts: Option<DateTime<Utc>>,
    hr_bpm: Option<i16>,
    spo2_pct: Option<i16>,
    activity: Option<String>,
    signal_quality: Option<i16>,
    hrv_rmssd_ms: Option<f32>,
}

const PATIENT_QUERY: &str = "
SELECT u.id, u.subject_code, u.display_name, d.device_id, d.last_seen,
       v.ts, v.hr_bpm, v.spo2_pct, v.activity, v.signal_quality, w.hrv_rmssd_ms
FROM users u
LEFT JOIN LATERAL (
    SELECT device_id, last_seen FROM devices
    WHERE assigned_user_id = u.id AND kind = 'wearable' AND status <> 'revoked'
    ORDER BY last_seen DESC NULLS LAST LIMIT 1) d ON TRUE
LEFT JOIN LATERAL (
    SELECT ts, hr_bpm, spo2_pct, activity, signal_quality FROM vitals
    WHERE device_id = d.device_id ORDER BY ts DESC LIMIT 1) v ON TRUE
LEFT JOIN LATERAL (
    SELECT hrv_rmssd_ms FROM windows
    WHERE device_id = d.device_id AND hrv_rmssd_ms IS NOT NULL ORDER BY window_start DESC LIMIT 1) w ON TRUE
WHERE u.role = 'patient' AND NOT u.disabled AND ($1::uuid IS NULL OR u.id = $1)";

/// Decides the priority level of a patient.
///
/// # Arguments
/// * `has_device` - a wearable is assigned.
/// * `open` - unresolved alerts of the device; an acknowledged alert still means
///   the condition lasts, so it keeps the level (it only stops escalation).
/// * `last_seen_ms` - newest record time.
/// * `now_ms` - current time.
#[must_use]
pub fn classify(
    has_device: bool,
    open: &[AlertDto],
    last_seen_ms: Option<i64>,
    now_ms: i64,
) -> PatientStatus {
    if !has_device {
        return PatientStatus::NoDevice;
    }
    let clinical = open
        .iter()
        .filter(|a| a.alert_type != AlertType::DeviceOffline);
    let worst = clinical.map(|a| a.severity.rank()).max();
    if worst == Some(Severity::Critical.rank()) {
        return PatientStatus::Critical;
    }
    if worst.is_some() {
        return PatientStatus::Warning;
    }
    let silent = last_seen_ms.is_none_or(|t| now_ms - t > OFFLINE_AFTER_MS);
    if silent
        || open
            .iter()
            .any(|a| a.alert_type == AlertType::DeviceOffline)
    {
        return PatientStatus::Offline;
    }
    PatientStatus::Normal
}

/// Picks the alert to show: unacknowledged first, then the most severe, then the newest.
#[must_use]
pub fn top_alert(open: &[AlertDto]) -> Option<AlertDto> {
    open.iter()
        .max_by_key(|a| {
            (
                a.status == AlertStatus::Open,
                a.severity.rank(),
                a.raised_at_unix_ms,
            )
        })
        .cloned()
}

async fn load(state: &AppState, only: Option<Uuid>) -> ApiResult<Vec<PatientSummary>> {
    let rows: Vec<PatientRow> = sqlx::query_as(PATIENT_QUERY)
        .bind(only)
        .fetch_all(&state.db)
        .await?;
    let devices: Vec<String> = rows.iter().filter_map(|r| r.device_id.clone()).collect();
    let alerts: Vec<AlertRow> = sqlx::query_as(&format!(
        "SELECT {ALERT_COLUMNS} FROM alerts WHERE status <> 'resolved' AND device_id = ANY($1)"
    ))
    .bind(&devices)
    .fetch_all(&state.db)
    .await?;
    let mut by_device: HashMap<String, Vec<AlertDto>> = HashMap::new();
    for a in alerts.iter().filter_map(AlertRow::to_dto) {
        by_device.entry(a.device_id.clone()).or_default().push(a);
    }
    let now = now_ms();
    let mut out: Vec<PatientSummary> = rows
        .into_iter()
        .map(|r| {
            let open = r
                .device_id
                .as_ref()
                .and_then(|d| by_device.get(d))
                .cloned()
                .unwrap_or_default();
            let last_seen = r.last_seen.map(to_ms);
            let latest = match (&r.device_id, r.ts, r.activity, r.signal_quality) {
                (Some(d), Some(ts), Some(activity), Some(q)) => Some(MeasurementDto {
                    device_id: d.clone(),
                    ts_unix_ms: to_ms(ts),
                    hr_bpm: r.hr_bpm,
                    spo2_pct: r.spo2_pct,
                    activity,
                    signal_quality: q,
                }),
                _ => None,
            };
            PatientSummary {
                user_id: r.id,
                subject_code: r.subject_code.unwrap_or_else(|| "BN-???".into()),
                display_name: r.display_name,
                status: classify(r.device_id.is_some(), &open, last_seen, now),
                open_alert_count: open
                    .iter()
                    .filter(|a| a.status == AlertStatus::Open)
                    .count(),
                top_alert: top_alert(&open),
                device_id: r.device_id,
                latest,
                hrv_rmssd_ms: r.hrv_rmssd_ms,
                last_seen_unix_ms: last_seen,
            }
        })
        .collect();
    out.sort_by_key(|p| {
        let recent = p
            .top_alert
            .as_ref()
            .map(|a| a.raised_at_unix_ms)
            .or(p.last_seen_unix_ms)
            .unwrap_or(i64::MIN);
        (p.status.order(), std::cmp::Reverse(recent))
    });
    Ok(out)
}

/// `GET /patients` (clinician).
///
/// # Errors
/// `403` for other roles.
pub async fn list(State(state): State<AppState>, caller: Caller) -> ApiResult<Response> {
    caller.require(&[Role::Clinician])?;
    let patients = load(&state, None).await?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "patients.list",
            "",
            Outcome::Allowed,
        )
        .await;
    Ok(no_store(Json(patients).into_response()))
}

/// `GET /patients/{user_id}` (clinician).
///
/// # Errors
/// `404` for other roles or unknown patients.
pub async fn get_one(
    State(state): State<AppState>,
    caller: Caller,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Response> {
    if caller.role != Role::Clinician {
        return Err(ApiError::not_found());
    }
    let one = load(&state, Some(user_id))
        .await?
        .into_iter()
        .next()
        .ok_or_else(ApiError::not_found)?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "patients.read",
            &user_id.to_string(),
            Outcome::Allowed,
        )
        .await;
    Ok(no_store(Json(one).into_response()))
}

/// A clinical note (OpenAPI `ClinicalNote`).
#[derive(Debug, Clone, Serialize)]
pub struct NoteDto {
    /// Note id.
    pub id: Uuid,
    /// Patient.
    pub patient_id: Uuid,
    /// Clinician who wrote it.
    pub author_id: Uuid,
    /// Author's display name.
    pub author_name: Option<String>,
    /// Text, 1..=1000 characters.
    pub text: String,
    /// When it was written, epoch ms.
    pub created_at_unix_ms: i64,
}

/// Body of `POST /patients/{user_id}/notes`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteCreate {
    /// Text, 1..=1000 characters.
    pub text: String,
}

type NoteRow = (Uuid, Uuid, Uuid, Option<String>, String, DateTime<Utc>);

fn note_dto((id, patient_id, author_id, author_name, text, created_at): NoteRow) -> NoteDto {
    NoteDto {
        id,
        patient_id,
        author_id,
        author_name,
        text,
        created_at_unix_ms: to_ms(created_at),
    }
}

async fn ensure_patient(state: &AppState, caller: Caller, user_id: Uuid) -> ApiResult<()> {
    if caller.role != Role::Clinician {
        return Err(ApiError::not_found());
    }
    let is_patient: Option<bool> =
        sqlx::query_scalar("SELECT role = 'patient' FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?;
    if is_patient == Some(true) {
        Ok(())
    } else {
        Err(ApiError::not_found())
    }
}

/// `GET /patients/{user_id}/notes` (clinician), newest first.
///
/// # Errors
/// `404` for other roles or unknown patients.
pub async fn list_notes(
    State(state): State<AppState>,
    caller: Caller,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Response> {
    ensure_patient(&state, caller, user_id).await?;
    let rows: Vec<NoteRow> = sqlx::query_as(
        "SELECT n.id, n.patient_id, n.author_id, a.display_name, n.text, n.created_at
         FROM clinical_notes n JOIN users a ON a.id = n.author_id
         WHERE n.patient_id = $1 ORDER BY n.created_at DESC LIMIT 1000",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "notes.read",
            &user_id.to_string(),
            Outcome::Allowed,
        )
        .await;
    let notes: Vec<NoteDto> = rows.into_iter().map(note_dto).collect();
    Ok(no_store(Json(notes).into_response()))
}

/// `POST /patients/{user_id}/notes` (clinician).
///
/// # Errors
/// `400` for an empty or long text, `404` for other roles or unknown patients.
pub async fn create_note(
    State(state): State<AppState>,
    caller: Caller,
    Path(user_id): Path<Uuid>,
    ApiJson(body): ApiJson<NoteCreate>,
) -> ApiResult<impl IntoResponse> {
    ensure_patient(&state, caller, user_id).await?;
    let text = body.text.trim().to_owned();
    if text.is_empty() || text.chars().count() > 1000 {
        return Err(ApiError::invalid("text", "must be 1 to 1000 characters"));
    }
    let row: NoteRow = sqlx::query_as(
        "WITH n AS (INSERT INTO clinical_notes (id, patient_id, author_id, text) VALUES ($1, $2, $3, $4)
                    RETURNING id, patient_id, author_id, text, created_at)
         SELECT n.id, n.patient_id, n.author_id, a.display_name, n.text, n.created_at
         FROM n JOIN users a ON a.id = n.author_id",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(caller.id)
    .bind(text)
    .fetch_one(&state.db)
    .await?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "notes.create",
            &user_id.to_string(),
            Outcome::Allowed,
        )
        .await;
    Ok((StatusCode::CREATED, Json(note_dto(row))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alert(t: AlertType, s: Severity, at: i64) -> AlertDto {
        AlertDto {
            id: Uuid::new_v4(),
            device_id: "0".repeat(32),
            alert_type: t,
            severity: s,
            status: AlertStatus::Open,
            raised_at_unix_ms: at,
            value: None,
            acknowledged_at_unix_ms: None,
            acknowledged_by: None,
        }
    }

    #[test]
    fn levels_follow_the_priority_rules() {
        let now = 10_000_000;
        assert_eq!(classify(false, &[], None, now), PatientStatus::NoDevice);
        assert_eq!(
            classify(true, &[], Some(now - 1000), now),
            PatientStatus::Normal
        );
        assert_eq!(
            classify(true, &[], Some(now - OFFLINE_AFTER_MS - 1), now),
            PatientStatus::Offline
        );
        assert_eq!(classify(true, &[], None, now), PatientStatus::Offline);
        let warn = [alert(AlertType::Spo2Low, Severity::Warning, 1)];
        assert_eq!(
            classify(true, &warn, Some(now), now),
            PatientStatus::Warning
        );
        let crit = [
            alert(AlertType::Spo2Low, Severity::Warning, 1),
            alert(AlertType::HrHigh, Severity::Critical, 2),
        ];
        assert_eq!(
            classify(true, &crit, Some(now), now),
            PatientStatus::Critical
        );
        let offline = [alert(AlertType::DeviceOffline, Severity::Warning, 1)];
        assert_eq!(
            classify(true, &offline, Some(now), now),
            PatientStatus::Offline
        );
    }

    #[test]
    fn the_top_alert_is_the_most_severe_then_newest() {
        let a = alert(AlertType::Spo2Low, Severity::Warning, 5);
        let b = alert(AlertType::HrHigh, Severity::Critical, 1);
        let c = alert(AlertType::HrLow, Severity::Critical, 3);
        assert_eq!(top_alert(&[a, b, c.clone()]).map(|x| x.id), Some(c.id));
        assert_eq!(top_alert(&[]), None);
    }
}
