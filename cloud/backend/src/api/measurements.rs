//! Summary vitals: a time range at raw or rollup resolution, and the latest row.
//!
//! Rollups are computed with `time_bucket` on the raw hypertable. Continuous
//! aggregates (D18) replace these queries when the data volume needs it; the
//! API does not change.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::devices::visible;
use super::extract::{ApiQuery, Caller, PageParams, encode_cursor};
use super::no_store;
use crate::domain::Role;
use crate::error::{ApiError, ApiResult};
use crate::infra::audit::Outcome;
use crate::state::AppState;
use crate::time::{from_ms, to_ms};

/// Resolution of a measurement query.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub enum Resolution {
    /// One row per second.
    #[default]
    #[serde(rename = "raw")]
    Raw,
    /// One-minute buckets.
    #[serde(rename = "1m")]
    Minute,
    /// One-hour buckets.
    #[serde(rename = "1h")]
    Hour,
    /// One-day buckets.
    #[serde(rename = "1d")]
    Day,
}

impl Resolution {
    const fn interval(self) -> &'static str {
        match self {
            Self::Raw | Self::Minute => "1 minute",
            Self::Hour => "1 hour",
            Self::Day => "1 day",
        }
    }
}

/// Query of `GET /measurements`.
#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    /// Device to read.
    pub device_id: String,
    /// Start, inclusive, epoch ms.
    pub from_unix_ms: i64,
    /// End, exclusive, epoch ms.
    pub to_unix_ms: i64,
    /// Raw rows or a rollup.
    #[serde(default)]
    pub resolution: Resolution,
    /// Paging.
    #[serde(flatten)]
    pub page: PageParams,
}

/// Query of `GET /measurements/latest`.
#[derive(Debug, Deserialize)]
pub struct LatestQuery {
    /// Device to read.
    pub device_id: String,
}

/// One raw summary row (OpenAPI `Measurement`).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MeasurementDto {
    /// Device id.
    pub device_id: String,
    /// Measurement time, epoch ms.
    pub ts_unix_ms: i64,
    /// Heart rate, bpm.
    pub hr_bpm: Option<i16>,
    /// Oxygen saturation, percent.
    pub spo2_pct: Option<i16>,
    /// Activity spelling.
    pub activity: String,
    /// Signal quality, 0..=100.
    pub signal_quality: i16,
}

/// One rollup bucket (OpenAPI `MeasurementBucket`).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BucketDto {
    /// Bucket start, epoch ms.
    pub bucket_start_unix_ms: i64,
    /// Raw rows in the bucket.
    pub sample_count: i64,
    /// Lowest heart rate, bpm.
    pub hr_min_bpm: Option<i16>,
    /// Mean heart rate, bpm.
    pub hr_avg_bpm: Option<f64>,
    /// Highest heart rate, bpm.
    pub hr_max_bpm: Option<i16>,
    /// Lowest SpO₂, percent.
    pub spo2_min_pct: Option<i16>,
    /// Steps counted by the edge windows starting in the bucket.
    pub step_count: i64,
}

/// bucket start, rows, hr min, hr avg, hr max, spo2 min, steps.
type BucketRow = (
    DateTime<Utc>,
    i64,
    Option<i16>,
    Option<f64>,
    Option<i16>,
    Option<i16>,
    i64,
);

#[derive(Debug, Serialize)]
struct MeasurementPage {
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Vec<MeasurementDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    buckets: Option<Vec<BucketDto>>,
    next_cursor: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct VitalsRow {
    device_id: String,
    ts: DateTime<Utc>,
    hr_bpm: Option<i16>,
    spo2_pct: Option<i16>,
    activity: String,
    signal_quality: i16,
}

impl From<VitalsRow> for MeasurementDto {
    fn from(r: VitalsRow) -> Self {
        Self {
            device_id: r.device_id,
            ts_unix_ms: to_ms(r.ts),
            hr_bpm: r.hr_bpm,
            spo2_pct: r.spo2_pct,
            activity: r.activity,
            signal_quality: r.signal_quality,
        }
    }
}

/// Checks that the caller may read health data of `device_id`.
///
/// Admins are refused (privacy by design: operations see metadata only).
///
/// # Errors
/// `403` for admins, `404` when the device is not visible.
pub async fn authorize_health_read(
    state: &AppState,
    caller: Caller,
    device_id: &str,
) -> ApiResult<()> {
    if caller.role == Role::Admin {
        state
            .audit
            .record(
                &state.db,
                Some(caller.id),
                "measurements.read",
                device_id,
                Outcome::Denied,
            )
            .await;
        return Err(ApiError::forbidden());
    }
    visible(state, caller, device_id).await.map(|_| ())
}

/// `GET /measurements`.
///
/// # Errors
/// `400` for a bad range or paging, `403` for admins, `404` for invisible devices.
pub async fn list(
    State(state): State<AppState>,
    caller: Caller,
    ApiQuery(q): ApiQuery<RangeQuery>,
) -> ApiResult<Response> {
    authorize_health_read(&state, caller, &q.device_id).await?;
    let limit = q.page.limit()?;
    let (Some(from), Some(to)) = (from_ms(q.from_unix_ms), from_ms(q.to_unix_ms)) else {
        return Err(ApiError::invalid("from_unix_ms", "out of range"));
    };
    if to <= from {
        return Err(ApiError::invalid(
            "to_unix_ms",
            "must be after from_unix_ms",
        ));
    }
    let after = q
        .page
        .cursor()?
        .map(|c| {
            c.parse::<i64>()
                .map_err(|_| ApiError::invalid("cursor", "not a valid cursor"))
        })
        .transpose()?
        .and_then(from_ms);
    state
        .audit
        .record(
            &state.db,
            Some(caller.id),
            "measurements.read",
            &q.device_id,
            Outcome::Allowed,
        )
        .await;

    let page = if q.resolution == Resolution::Raw {
        let rows: Vec<VitalsRow> = sqlx::query_as(
            "SELECT device_id, ts, hr_bpm, spo2_pct, activity, signal_quality FROM vitals
             WHERE device_id = $1 AND ts >= $2 AND ts < $3 AND ($4::timestamptz IS NULL OR ts > $4)
             ORDER BY ts LIMIT $5",
        )
        .bind(&q.device_id)
        .bind(from)
        .bind(to)
        .bind(after)
        .bind(limit + 1)
        .fetch_all(&state.db)
        .await?;
        let more = rows.len() as i64 > limit;
        let items: Vec<MeasurementDto> = rows
            .into_iter()
            .take(usize::try_from(limit).unwrap_or(100))
            .map(MeasurementDto::from)
            .collect();
        let next_cursor = more
            .then(|| {
                items
                    .last()
                    .map(|m| encode_cursor(&m.ts_unix_ms.to_string()))
            })
            .flatten();
        MeasurementPage {
            items: Some(items),
            buckets: None,
            next_cursor,
        }
    } else {
        let interval = q.resolution.interval();
        let rows: Vec<BucketRow> =
            sqlx::query_as(&format!(
                "WITH v AS (
                    SELECT time_bucket(INTERVAL '{interval}', ts) AS b, count(*) AS n,
                           min(hr_bpm) AS hr_min, avg(hr_bpm)::float8 AS hr_avg, max(hr_bpm) AS hr_max,
                           min(spo2_pct) AS spo2_min
                    FROM vitals WHERE device_id = $1 AND ts >= $2 AND ts < $3 GROUP BY b),
                 w AS (
                    SELECT time_bucket(INTERVAL '{interval}', window_start) AS b, sum(step_count)::int8 AS steps
                    FROM windows WHERE device_id = $1 AND window_start >= $2 AND window_start < $3 GROUP BY b)
                 SELECT v.b, v.n, v.hr_min, v.hr_avg, v.hr_max, v.spo2_min, coalesce(w.steps, 0)
                 FROM v LEFT JOIN w USING (b)
                 WHERE ($4::timestamptz IS NULL OR v.b > $4)
                 ORDER BY v.b LIMIT $5"
            ))
            .bind(&q.device_id)
            .bind(from)
            .bind(to)
            .bind(after)
            .bind(limit + 1)
            .fetch_all(&state.db)
            .await?;
        let more = rows.len() as i64 > limit;
        let buckets: Vec<BucketDto> = rows
            .into_iter()
            .take(usize::try_from(limit).unwrap_or(100))
            .map(
                |(b, n, hr_min, hr_avg, hr_max, spo2_min, steps)| BucketDto {
                    bucket_start_unix_ms: to_ms(b),
                    sample_count: n,
                    hr_min_bpm: hr_min,
                    hr_avg_bpm: hr_avg,
                    hr_max_bpm: hr_max,
                    spo2_min_pct: spo2_min,
                    step_count: steps,
                },
            )
            .collect();
        let next_cursor = more
            .then(|| {
                buckets
                    .last()
                    .map(|b| encode_cursor(&b.bucket_start_unix_ms.to_string()))
            })
            .flatten();
        MeasurementPage {
            items: None,
            buckets: Some(buckets),
            next_cursor,
        }
    };
    Ok(no_store(Json(page).into_response()))
}

/// Latest summary row of a device, if any.
///
/// # Errors
/// Returns the database error.
pub async fn latest_row(
    state: &AppState,
    device_id: &str,
) -> Result<Option<MeasurementDto>, sqlx::Error> {
    let row: Option<VitalsRow> = sqlx::query_as(
        "SELECT device_id, ts, hr_bpm, spo2_pct, activity, signal_quality FROM vitals
         WHERE device_id = $1 ORDER BY ts DESC LIMIT 1",
    )
    .bind(device_id)
    .fetch_optional(&state.db)
    .await?;
    Ok(row.map(MeasurementDto::from))
}

/// `GET /measurements/latest`.
///
/// # Errors
/// `403` for admins, `404` for invisible devices or no data yet.
pub async fn latest(
    State(state): State<AppState>,
    caller: Caller,
    ApiQuery(q): ApiQuery<LatestQuery>,
) -> ApiResult<Response> {
    authorize_health_read(&state, caller, &q.device_id).await?;
    let row = latest_row(&state, &q.device_id)
        .await?
        .ok_or_else(ApiError::not_found)?;
    Ok(no_store(Json(row).into_response()))
}
