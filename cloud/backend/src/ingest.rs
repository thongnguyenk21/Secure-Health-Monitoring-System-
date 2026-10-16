//! gRPC `TelemetryIngestService` (`iomt.telemetry.v1`) for gateways.
//!
//! Delivery is at-least-once, so every insert is `ON CONFLICT DO NOTHING` on the
//! `RecordKey`: a resend is reported as accepted and stored once. Only rows that
//! were actually inserted reach the alert engine, so a replayed batch cannot
//! open an alert twice.
//!
//! Gateway identity comes from the mTLS client certificate in production (S4).
//! Until then the service trusts its network and checks only that each device
//! is registered and not revoked.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use iomt_protocol::telemetry_v1 as pb;
use iomt_protocol::telemetry_v1::telemetry_ingest_service_server::TelemetryIngestService;
use iomt_protocol::{Activity, ConversionError, IdempotencyKey, vitals_from_pb};
use tonic::{Request, Response, Status};

use crate::alerts::rules::Sample;
use crate::state::AppState;
use crate::time::{from_ms, now_ms};

/// Most records accepted in one batch (REQ-REL-011).
pub const MAX_BATCH_RECORDS: usize = 400;
/// Largest encoded batch, bytes (REQ-REL-011).
pub const MAX_BATCH_BYTES: usize = 256 * 1024;
/// Records further in the future than this are quarantined, ms.
pub const MAX_FUTURE_MS: i64 = 5 * 60 * 1000;
/// Records older than this are quarantined, ms (a phone queue holds about a day).
pub const MAX_PAST_MS: i64 = 30 * 24 * 3600 * 1000;

/// The ingest service.
#[derive(Debug, Clone)]
pub struct Ingest {
    state: AppState,
}

impl Ingest {
    /// Creates the service over the shared state.
    #[must_use]
    pub const fn new(state: AppState) -> Self {
        Self { state }
    }
}

fn activity_text(a: Activity) -> &'static str {
    match a {
        Activity::Rest => "rest",
        Activity::Walk => "walk",
        Activity::Run => "run",
        Activity::Unknown => "unknown",
    }
}

fn reject(
    key: pb::RecordKey,
    reason: pb::RejectReason,
    detail: impl Into<String>,
) -> pb::Rejection {
    pb::Rejection {
        key: Some(key),
        reason: reason as i32,
        detail: detail.into(),
    }
}

/// Whether a timestamp is plausible relative to `now_ms`.
#[must_use]
pub fn timestamp_ok(ts_ms: i64, now_ms: i64) -> bool {
    ts_ms <= now_ms + MAX_FUTURE_MS && ts_ms >= now_ms - MAX_PAST_MS
}

/// Status of the devices named in a batch: `true` = may upload.
async fn device_gate(
    state: &AppState,
    ids: Vec<String>,
) -> Result<HashMap<String, bool>, sqlx::Error> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT device_id, status FROM devices WHERE device_id = ANY($1) AND kind = 'wearable'",
    )
    .bind(&ids)
    .fetch_all(&state.db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, status)| (id, status != "revoked"))
        .collect())
}

fn internal(e: &sqlx::Error) -> Status {
    tracing::error!(error = %e, "ingest database error");
    Status::unavailable("storage unavailable; retry the batch")
}

#[tonic::async_trait]
impl TelemetryIngestService for Ingest {
    async fn upload_batch(
        &self,
        request: Request<pb::UploadBatchRequest>,
    ) -> Result<Response<pb::UploadBatchResponse>, Status> {
        let req = request.into_inner();
        if req.vitals.len() + req.windows.len() > MAX_BATCH_RECORDS {
            return Err(Status::invalid_argument(
                "more than 400 records in one batch",
            ));
        }
        if req.schema_version != "1" {
            return Err(Status::invalid_argument("unsupported schema_version"));
        }
        let state = &self.state;
        let now = now_ms();

        let mut ids: Vec<String> = req
            .vitals
            .iter()
            .filter_map(|v| v.key.as_ref().map(|k| k.device_id.clone()))
            .chain(
                req.windows
                    .iter()
                    .filter_map(|w| w.key.as_ref().map(|k| k.device_id.clone())),
            )
            .collect();
        ids.sort();
        ids.dedup();
        let gate = device_gate(state, ids).await.map_err(|e| internal(&e))?;

        let mut accepted = Vec::new();
        let mut rejected = Vec::new();
        let mut new_samples: BTreeMap<String, Vec<Sample>> = BTreeMap::new();
        let mut newest: HashMap<String, i64> = HashMap::new();

        for msg in req.vitals {
            let raw_key = msg.key.clone().unwrap_or_default();
            let (key, vitals) = match vitals_from_pb(msg) {
                Ok(ok) => ok,
                Err(e) => {
                    rejected.push(reject(raw_key, e.reject_reason(), e.to_string()));
                    continue;
                }
            };
            let device = key.device_id.as_str().to_owned();
            if gate.get(&device) != Some(&true) {
                rejected.push(reject(
                    raw_key,
                    pb::RejectReason::UnknownDevice,
                    "device not registered or revoked",
                ));
                continue;
            }
            if !timestamp_ok(vitals.ts.0, now) {
                quarantine(state, &device, "vitals", vitals.ts.0)
                    .await
                    .map_err(|e| internal(&e))?;
                rejected.push(reject(
                    raw_key,
                    pb::RejectReason::TimestampQuarantined,
                    "timestamp too far from server time",
                ));
                continue;
            }
            let (Some(ts), Some(window_start)) =
                (from_ms(vitals.ts.0), from_ms(key.window_start.0))
            else {
                rejected.push(reject(
                    raw_key,
                    pb::RejectReason::InvalidValue,
                    "timestamp out of range",
                ));
                continue;
            };
            let inserted = sqlx::query(
                "INSERT INTO vitals (device_id, window_start, seq, boot_id, ts, hr_bpm, spo2_pct, activity, signal_quality)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (device_id, window_start, seq, boot_id) DO NOTHING",
            )
            .bind(&device)
            .bind(window_start)
            .bind(i64::from(key.seq))
            .bind(i64::from(key.boot_id))
            .bind(ts)
            .bind(vitals.hr_bpm.map(|v| i16::try_from(v).unwrap_or(i16::MAX)))
            .bind(vitals.spo2_pct.map(i16::from))
            .bind(activity_text(vitals.activity))
            .bind(i16::from(vitals.quality.value()))
            .execute(&state.db)
            .await
            .map_err(|e| internal(&e))?
            .rows_affected()
                == 1;
            if inserted {
                new_samples.entry(device.clone()).or_default().push(Sample {
                    ts_ms: vitals.ts.0,
                    hr_bpm: vitals.hr_bpm,
                    spo2_pct: vitals.spo2_pct.map(u16::from),
                    quality: vitals.quality.value(),
                });
                let n = newest.entry(device).or_insert(i64::MIN);
                *n = (*n).max(vitals.ts.0);
            }
            accepted.push(raw_key);
        }

        let mut anomalies: Vec<(String, bool, f64, i64)> = Vec::new();
        for msg in req.windows {
            let raw_key = msg.key.clone().unwrap_or_default();
            match store_window(state, &gate, msg, now).await {
                Ok(WindowOutcome::Stored {
                    device,
                    end_ms,
                    anomaly,
                }) => {
                    if let Some((flag, score)) = anomaly {
                        anomalies.push((device.clone(), flag, score, end_ms));
                    }
                    let n = newest.entry(device).or_insert(i64::MIN);
                    *n = (*n).max(end_ms);
                    accepted.push(raw_key);
                }
                Ok(WindowOutcome::Duplicate) => accepted.push(raw_key),
                Ok(WindowOutcome::Rejected(reason, detail)) => {
                    rejected.push(reject(raw_key, reason, detail))
                }
                Err(e) => return Err(internal(&e)),
            }
        }

        for (device, ts) in &newest {
            sqlx::query(
                "UPDATE devices SET last_seen = GREATEST(last_seen, $2),
                        status = CASE WHEN status = 'pending' THEN 'active' ELSE status END
                 WHERE device_id = $1",
            )
            .bind(device)
            .bind(from_ms(*ts))
            .execute(&state.db)
            .await
            .map_err(|e| internal(&e))?;
            state
                .engine
                .on_data(&state.db, device, *ts)
                .await
                .map_err(|e| internal(&e))?;
        }
        for (device, mut samples) in new_samples {
            samples.sort_by_key(|s| s.ts_ms);
            state
                .engine
                .on_vitals(&state.db, &device, &samples)
                .await
                .map_err(|e| internal(&e))?;
        }
        for (device, flag, score, at) in anomalies {
            state
                .engine
                .on_anomaly(&state.db, &device, flag, score, at)
                .await
                .map_err(|e| internal(&e))?;
        }

        tracing::debug!(
            accepted = accepted.len(),
            rejected = rejected.len(),
            "batch stored"
        );
        Ok(Response::new(pb::UploadBatchResponse {
            accepted,
            rejected,
        }))
    }
}

async fn quarantine(
    state: &AppState,
    device: &str,
    kind: &str,
    ts_ms: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO quarantine (device_id, kind, ts, reason) VALUES ($1, $2, $3, 'timestamp')",
    )
    .bind(device)
    .bind(kind)
    .bind(from_ms(ts_ms))
    .execute(&state.db)
    .await?;
    Ok(())
}

enum WindowOutcome {
    Stored {
        device: String,
        end_ms: i64,
        anomaly: Option<(bool, f64)>,
    },
    Duplicate,
    Rejected(pb::RejectReason, String),
}

fn opt_u16(
    v: Option<u32>,
    field: &'static str,
    range: std::ops::RangeInclusive<u32>,
) -> Result<Option<i16>, String> {
    match v {
        None => Ok(None),
        Some(x) if range.contains(&x) => Ok(i16::try_from(x).ok()),
        Some(_) => Err(format!("{field} out of range")),
    }
}

fn window_activity(raw: i32) -> Result<&'static str, ConversionError> {
    let a = pb::Activity::try_from(raw).map_err(|_| ConversionError::InvalidActivity(raw))?;
    Activity::try_from(a).map(activity_text)
}

async fn store_window(
    state: &AppState,
    gate: &HashMap<String, bool>,
    msg: pb::WindowFeatures,
    now: i64,
) -> Result<WindowOutcome, sqlx::Error> {
    let invalid = |d: String| Ok(WindowOutcome::Rejected(pb::RejectReason::InvalidValue, d));
    let Some(raw_key) = msg.key.clone() else {
        return invalid("required field `key` is missing".into());
    };
    let key = match IdempotencyKey::try_from(raw_key) {
        Ok(k) => k,
        Err(e) => return invalid(e.to_string()),
    };
    let device = key.device_id.as_str().to_owned();
    if gate.get(&device) != Some(&true) {
        return Ok(WindowOutcome::Rejected(
            pb::RejectReason::UnknownDevice,
            "device not registered or revoked".into(),
        ));
    }
    if msg.window_end_unix_ms <= key.window_start.0 {
        return invalid("window_end must be after window_start".into());
    }
    if !timestamp_ok(msg.window_end_unix_ms, now) {
        quarantine(state, &device, "window", msg.window_end_unix_ms).await?;
        return Ok(WindowOutcome::Rejected(
            pb::RejectReason::TimestampQuarantined,
            "timestamp too far from server time".into(),
        ));
    }
    let activity = match window_activity(msg.dominant_activity) {
        Ok(a) => a,
        Err(e) => return invalid(e.to_string()),
    };
    let (hr_min, hr_max, spo2_min) = match (
        opt_u16(msg.hr_min_bpm, "hr_min_bpm", 20..=250),
        opt_u16(msg.hr_max_bpm, "hr_max_bpm", 20..=250),
        opt_u16(msg.spo2_min_pct, "spo2_min_pct", 50..=100),
    ) {
        (Ok(a), Ok(b), Ok(c)) => (a, b, c),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => return invalid(e),
    };
    if msg.mean_signal_quality > 100 {
        return invalid("mean_signal_quality out of range".into());
    }
    let inference = msg.inference.as_ref();
    if inference.is_some_and(|i| !(0.0..=1.0).contains(&i.anomaly_score)) {
        return invalid("anomaly_score out of range".into());
    }
    let start: Option<DateTime<Utc>> = from_ms(key.window_start.0);
    let end: Option<DateTime<Utc>> = from_ms(msg.window_end_unix_ms);
    let inserted = sqlx::query(
        "INSERT INTO windows (device_id, window_start, seq, boot_id, window_end, sample_count, hr_mean_bpm,
                              hr_min_bpm, hr_max_bpm, spo2_min_pct, hrv_rmssd_ms, step_count, dominant_activity,
                              mean_signal_quality, model_version, anomaly_score, anomaly_flag)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
         ON CONFLICT (device_id, window_start, seq, boot_id) DO NOTHING",
    )
    .bind(&device)
    .bind(start)
    .bind(i64::from(key.seq))
    .bind(i64::from(key.boot_id))
    .bind(end)
    .bind(i32::try_from(msg.sample_count).unwrap_or(i32::MAX))
    .bind(msg.hr_mean_bpm)
    .bind(hr_min)
    .bind(hr_max)
    .bind(spo2_min)
    .bind(msg.hrv_rmssd_ms)
    .bind(i32::try_from(msg.step_count).unwrap_or(i32::MAX))
    .bind(activity)
    .bind(i16::try_from(msg.mean_signal_quality).unwrap_or(100))
    .bind(inference.map(|i| i.model_version.clone()))
    .bind(inference.map(|i| i.anomaly_score))
    .bind(inference.map(|i| i.anomaly_flag))
    .execute(&state.db)
    .await?
    .rows_affected()
        == 1;
    if !inserted {
        return Ok(WindowOutcome::Duplicate);
    }
    Ok(WindowOutcome::Stored {
        device,
        end_ms: msg.window_end_unix_ms,
        anomaly: inference.map(|i| (i.anomaly_flag, f64::from(i.anomaly_score))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_are_checked_against_both_bounds() {
        let now = 1_700_000_000_000;
        assert!(timestamp_ok(now, now));
        assert!(
            timestamp_ok(now - 15 * 60 * 1000, now),
            "an offline backlog is fine"
        );
        assert!(!timestamp_ok(now + MAX_FUTURE_MS + 1, now));
        assert!(!timestamp_ok(now - MAX_PAST_MS - 1, now));
    }

    #[test]
    fn optional_ranges_reject_out_of_range_values() {
        assert_eq!(opt_u16(None, "x", 20..=250), Ok(None));
        assert_eq!(opt_u16(Some(72), "x", 20..=250), Ok(Some(72)));
        assert!(opt_u16(Some(300), "x", 20..=250).is_err());
    }
}
