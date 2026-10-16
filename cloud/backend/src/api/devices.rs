//! Devices: list, register, label, assign and revoke.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use iomt_protocol::DeviceId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::extract::{ApiJson, ApiQuery, Caller, PageParams, encode_cursor};
use crate::domain::{DeviceKind, DeviceStatus, Role};
use crate::error::{ApiError, ApiResult};
use crate::infra::audit::Outcome;
use crate::state::AppState;
use crate::time::to_ms;

/// Columns selected for a device.
pub const DEVICE_COLUMNS: &str =
    "device_id, kind, status, label, assigned_user_id, fw_version, last_seen";

/// One row of `devices`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    /// Pseudonymous id.
    pub device_id: String,
    /// Stored kind spelling.
    pub kind: String,
    /// Stored status spelling.
    pub status: String,
    /// Admin label.
    pub label: Option<String>,
    /// Assigned patient.
    pub assigned_user_id: Option<Uuid>,
    /// Firmware version.
    pub fw_version: Option<String>,
    /// Newest record time.
    pub last_seen: Option<DateTime<Utc>>,
}

/// A device as the API returns it (OpenAPI `Device`).
#[derive(Debug, Clone, Serialize)]
pub struct DeviceDto {
    /// Pseudonymous id.
    pub device_id: String,
    /// Kind.
    pub kind: DeviceKind,
    /// Status.
    pub status: DeviceStatus,
    /// Admin label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Assigned patient.
    pub assigned_user_id: Option<Uuid>,
    /// Firmware version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fw_version: Option<String>,
    /// Newest record time, epoch ms.
    pub last_seen_unix_ms: Option<i64>,
}

impl TryFrom<DeviceRow> for DeviceDto {
    type Error = ApiError;
    fn try_from(r: DeviceRow) -> Result<Self, Self::Error> {
        Ok(Self {
            device_id: r.device_id,
            kind: DeviceKind::parse(&r.kind).ok_or_else(ApiError::internal)?,
            status: DeviceStatus::parse(&r.status).ok_or_else(ApiError::internal)?,
            label: r.label,
            assigned_user_id: r.assigned_user_id,
            fw_version: r.fw_version,
            last_seen_unix_ms: r.last_seen.map(to_ms),
        })
    }
}

/// One page of devices.
#[derive(Debug, Serialize)]
pub struct DevicePage {
    /// Devices ordered by id.
    pub items: Vec<DeviceDto>,
    /// Cursor of the next page.
    pub next_cursor: Option<String>,
}

/// Body of `POST /devices`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceCreate {
    /// Pseudonymous id, 32 lowercase hex characters.
    pub device_id: String,
    /// Kind.
    pub kind: DeviceKind,
    /// Admin label.
    pub label: Option<String>,
}

/// Body of `PATCH /devices/{id}`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevicePatch {
    /// New label.
    pub label: Option<String>,
}

/// Body of `POST /devices/{id}/assign`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceAssign {
    /// Patient to assign to; `null` unassigns.
    pub user_id: Option<Uuid>,
}

fn check_id(raw: &str) -> ApiResult<String> {
    DeviceId::new(raw)
        .map(String::from)
        .map_err(|_| ApiError::invalid("device_id", "must be 32 lowercase hex characters"))
}

fn check_label(label: Option<String>) -> ApiResult<Option<String>> {
    let label = label.map(|l| l.trim().to_owned()).filter(|l| !l.is_empty());
    if label.as_ref().is_some_and(|l| l.chars().count() > 64) {
        return Err(ApiError::invalid("label", "at most 64 characters"));
    }
    Ok(label)
}

/// Loads a device the caller may see.
///
/// Patients see only devices assigned to them; clinicians and admins see all.
///
/// # Errors
/// `404` when the device does not exist or is not visible.
pub async fn visible(state: &AppState, caller: Caller, device_id: &str) -> ApiResult<DeviceRow> {
    let row: Option<DeviceRow> = sqlx::query_as(&format!(
        "SELECT {DEVICE_COLUMNS} FROM devices WHERE device_id = $1"
    ))
    .bind(device_id)
    .fetch_optional(&state.db)
    .await?;
    row.filter(|d| caller.role != Role::Patient || d.assigned_user_id == Some(caller.id))
        .ok_or_else(ApiError::not_found)
}

/// `GET /devices`.
///
/// # Errors
/// `400` for bad paging.
pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
    ApiQuery(page): ApiQuery<PageParams>,
) -> ApiResult<Json<DevicePage>> {
    let limit = page.limit()?;
    let after = page.cursor()?;
    let only_user = (caller.role == Role::Patient).then_some(caller.id);
    let rows: Vec<DeviceRow> = sqlx::query_as(&format!(
        "SELECT {DEVICE_COLUMNS} FROM devices
         WHERE ($1::uuid IS NULL OR assigned_user_id = $1) AND ($2::text IS NULL OR device_id > $2)
         ORDER BY device_id LIMIT $3"
    ))
    .bind(only_user)
    .bind(after)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;
    let more = rows.len() as i64 > limit;
    let items: Vec<DeviceDto> = rows
        .into_iter()
        .take(usize::try_from(limit).unwrap_or(100))
        .map(DeviceDto::try_from)
        .collect::<Result<_, _>>()?;
    let next_cursor = more
        .then(|| items.last().map(|d| encode_cursor(&d.device_id)))
        .flatten();
    Ok(Json(DevicePage { items, next_cursor }))
}

/// `GET /devices/{device_id}`.
///
/// # Errors
/// `404` when not visible.
pub async fn get_one(
    State(state): State<AppState>,
    caller: Caller,
    Path(device_id): Path<String>,
) -> ApiResult<Json<DeviceDto>> {
    Ok(Json(visible(&state, caller, &device_id).await?.try_into()?))
}

/// `POST /devices`: registers a device as pending (admin).
///
/// # Errors
/// `400` for a bad id or label, `403` for non-admins, `409` for a known id.
pub async fn create(
    State(state): State<AppState>,
    caller: Caller,
    ApiJson(body): ApiJson<DeviceCreate>,
) -> ApiResult<impl IntoResponse> {
    caller.require(&[Role::Admin])?;
    let id = check_id(&body.device_id)?;
    let label = check_label(body.label)?;
    let row: Result<DeviceRow, sqlx::Error> = sqlx::query_as(&format!(
        "INSERT INTO devices (device_id, kind, label) VALUES ($1, $2, $3) RETURNING {DEVICE_COLUMNS}"
    ))
    .bind(&id)
    .bind(body.kind.as_str())
    .bind(label)
    .fetch_one(&state.db)
    .await;
    let row = match row {
        Ok(r) => r,
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            return Err(ApiError::conflict("a device with this id"));
        }
        Err(e) => return Err(e.into()),
    };
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "device.create",
            &id,
            Outcome::Allowed,
        )
        .await;
    Ok((StatusCode::CREATED, Json(DeviceDto::try_from(row)?)))
}

/// `PATCH /devices/{device_id}`: changes the label (admin).
///
/// # Errors
/// `400` for a long label, `404` for non-admins or unknown ids.
pub async fn update(
    State(state): State<AppState>,
    caller: Caller,
    Path(device_id): Path<String>,
    ApiJson(body): ApiJson<DevicePatch>,
) -> ApiResult<Json<DeviceDto>> {
    if caller.role != Role::Admin {
        return Err(ApiError::not_found());
    }
    let label = check_label(body.label)?;
    let row: Option<DeviceRow> = sqlx::query_as(&format!(
        "UPDATE devices SET label = $2 WHERE device_id = $1 RETURNING {DEVICE_COLUMNS}"
    ))
    .bind(&device_id)
    .bind(label)
    .fetch_optional(&state.db)
    .await?;
    let row = row.ok_or_else(ApiError::not_found)?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "device.label",
            &device_id,
            Outcome::Allowed,
        )
        .await;
    Ok(Json(row.try_into()?))
}

/// `POST /devices/{device_id}/assign` (clinician, admin).
///
/// # Errors
/// `400` when the user is not a patient, `404` for patients or unknown ids.
pub async fn assign(
    State(state): State<AppState>,
    caller: Caller,
    Path(device_id): Path<String>,
    ApiJson(body): ApiJson<DeviceAssign>,
) -> ApiResult<Json<DeviceDto>> {
    if caller.role == Role::Patient {
        return Err(ApiError::not_found());
    }
    if let Some(user) = body.user_id {
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
            .bind(user)
            .fetch_optional(&state.db)
            .await?;
        if role.as_deref() != Some("patient") {
            return Err(ApiError::invalid("user_id", "must be a patient"));
        }
    }
    let row: Option<DeviceRow> = sqlx::query_as(&format!(
        "UPDATE devices SET assigned_user_id = $2 WHERE device_id = $1 AND kind = 'wearable'
         RETURNING {DEVICE_COLUMNS}"
    ))
    .bind(&device_id)
    .bind(body.user_id)
    .fetch_optional(&state.db)
    .await?;
    let row = row.ok_or_else(ApiError::not_found)?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "device.assign",
            &device_id,
            Outcome::Allowed,
        )
        .await;
    Ok(Json(row.try_into()?))
}

/// `POST /devices/{device_id}/revoke` (admin): refused from the next upload on.
///
/// # Errors
/// `404` for non-admins or unknown ids.
pub async fn revoke(
    State(state): State<AppState>,
    caller: Caller,
    Path(device_id): Path<String>,
) -> ApiResult<Json<DeviceDto>> {
    if caller.role != Role::Admin {
        return Err(ApiError::not_found());
    }
    let row: Option<DeviceRow> = sqlx::query_as(&format!(
        "UPDATE devices SET status = 'revoked' WHERE device_id = $1 RETURNING {DEVICE_COLUMNS}"
    ))
    .bind(&device_id)
    .fetch_optional(&state.db)
    .await?;
    let row = row.ok_or_else(ApiError::not_found)?;
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "device.revoke",
            &device_id,
            Outcome::Allowed,
        )
        .await;
    Ok(Json(row.try_into()?))
}
