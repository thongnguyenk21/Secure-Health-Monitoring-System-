//! Applies rule transitions to the database and publishes live events.

use std::collections::HashMap;
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use super::model::{ALERT_COLUMNS, AlertRow, LiveEvent, LiveEventType};
use super::rules::{DeviceRules, RuleState, Sample, Thresholds, Transition};
use crate::domain::{AlertType, Severity};
use crate::time::from_ms;

/// A wearable is offline after this long without data, ms.
pub const OFFLINE_AFTER_MS: i64 = 5 * 60 * 1000;
/// An unacknowledged warning becomes critical after this long, ms (REQ-REL-036).
pub const ESCALATE_WARNING_AFTER_MS: i64 = 10 * 60 * 1000;
/// An unacknowledged critical alert is escalated after this long, ms.
pub const ESCALATE_CRITICAL_AFTER_MS: i64 = 2 * 60 * 1000;

/// Evaluates rules per device and keeps their state between batches.
#[derive(Debug)]
pub struct AlertEngine {
    thresholds: Thresholds,
    devices: Mutex<HashMap<String, DeviceRules>>,
    live: broadcast::Sender<LiveEvent>,
}

impl AlertEngine {
    /// Creates an engine that publishes on `live`.
    #[must_use]
    pub fn new(thresholds: Thresholds, live: broadcast::Sender<LiveEvent>) -> Self {
        Self {
            thresholds,
            devices: Mutex::new(HashMap::new()),
            live,
        }
    }

    /// Feeds new samples of one device, oldest first, and applies the transitions.
    ///
    /// The first time a device is seen, rule state is restored from its
    /// unresolved alerts, so a restart does not open duplicates or forget to
    /// resolve.
    ///
    /// # Errors
    /// Returns the database error.
    pub async fn on_vitals(
        &self,
        db: &PgPool,
        device_id: &str,
        samples: &[Sample],
    ) -> Result<(), sqlx::Error> {
        let mut devices = self.devices.lock().await;
        if !devices.contains_key(device_id) {
            let restored = restore(db, device_id).await?;
            devices.insert(device_id.to_owned(), restored);
        }
        let Some(rules) = devices.get_mut(device_id) else {
            return Ok(());
        };
        let transitions: Vec<Transition> = samples
            .iter()
            .flat_map(|s| rules.evaluate(*s, &self.thresholds))
            .collect();
        drop(devices);
        for t in transitions {
            self.apply(db, device_id, t).await?;
        }
        Ok(())
    }

    /// Opens or resolves the `anomaly` alert from an edge model result.
    ///
    /// # Errors
    /// Returns the database error.
    pub async fn on_anomaly(
        &self,
        db: &PgPool,
        device_id: &str,
        flagged: bool,
        score: f64,
        at_ms: i64,
    ) -> Result<(), sqlx::Error> {
        let t = if flagged {
            Transition::Open {
                alert_type: AlertType::Anomaly,
                severity: Severity::Warning,
                value: score,
                at_ms,
            }
        } else {
            Transition::Resolve {
                alert_type: AlertType::Anomaly,
                at_ms,
            }
        };
        self.apply(db, device_id, t).await
    }

    /// Resolves an open `device_offline` alert because data arrived.
    ///
    /// # Errors
    /// Returns the database error.
    pub async fn on_data(
        &self,
        db: &PgPool,
        device_id: &str,
        at_ms: i64,
    ) -> Result<(), sqlx::Error> {
        self.apply(
            db,
            device_id,
            Transition::Resolve {
                alert_type: AlertType::DeviceOffline,
                at_ms,
            },
        )
        .await
    }

    async fn apply(&self, db: &PgPool, device_id: &str, t: Transition) -> Result<(), sqlx::Error> {
        let row: Option<AlertRow> = match t {
            Transition::Open {
                alert_type,
                severity,
                value,
                at_ms,
            } => {
                sqlx::query_as(&format!(
                    "INSERT INTO alerts (id, device_id, patient_id, alert_type, severity, raised_at, value)
                     SELECT $1, d.device_id, d.assigned_user_id, $3, $4, $5, $6 FROM devices d WHERE d.device_id = $2
                     ON CONFLICT (device_id, alert_type) WHERE status <> 'resolved' DO NOTHING
                     RETURNING {ALERT_COLUMNS}"
                ))
                .bind(Uuid::new_v4())
                .bind(device_id)
                .bind(alert_type.as_str())
                .bind(severity.as_str())
                .bind(from_ms(at_ms).unwrap_or_else(chrono::Utc::now))
                .bind(value)
                .fetch_optional(db)
                .await?
            }
            Transition::Raise { alert_type, value } => {
                sqlx::query_as(&format!(
                    "UPDATE alerts SET severity = 'critical', value = $3, updated_at = now()
                     WHERE device_id = $1 AND alert_type = $2 AND status <> 'resolved' AND severity <> 'critical'
                     RETURNING {ALERT_COLUMNS}"
                ))
                .bind(device_id)
                .bind(alert_type.as_str())
                .bind(value)
                .fetch_optional(db)
                .await?
            }
            Transition::Resolve { alert_type, at_ms } => {
                sqlx::query_as(&format!(
                    "UPDATE alerts SET status = 'resolved', resolved_at = $3, updated_at = now()
                     WHERE device_id = $1 AND alert_type = $2 AND status <> 'resolved'
                     RETURNING {ALERT_COLUMNS}"
                ))
                .bind(device_id)
                .bind(alert_type.as_str())
                .bind(from_ms(at_ms).unwrap_or_else(chrono::Utc::now))
                .fetch_optional(db)
                .await?
            }
        };
        if let Some(row) = row {
            let kind = if matches!(t, Transition::Open { .. }) {
                LiveEventType::AlertOpened
            } else {
                LiveEventType::AlertUpdated
            };
            tracing::info!(device = &device_id[..8], alert_type = %row.alert_type, ?kind, "alert changed");
            self.publish(kind, &row);
        }
        Ok(())
    }

    /// Publishes a change on the live stream; nobody listening is fine.
    pub fn publish(&self, kind: LiveEventType, row: &AlertRow) {
        if let Some(alert) = row.to_dto() {
            let _ = self.live.send(LiveEvent { kind, alert });
        }
    }

    /// Opens `device_offline` for silent wearables and escalates old open alerts.
    ///
    /// # Errors
    /// Returns the database error.
    pub async fn periodic(&self, db: &PgPool) -> Result<(), sqlx::Error> {
        let opened: Vec<AlertRow> = sqlx::query_as(&format!(
            "INSERT INTO alerts (id, device_id, patient_id, alert_type, severity, raised_at)
             SELECT gen_random_uuid(), d.device_id, d.assigned_user_id, 'device_offline', 'warning', now()
             FROM devices d
             WHERE d.kind = 'wearable' AND d.status = 'active' AND d.assigned_user_id IS NOT NULL
               AND d.last_seen < now() - make_interval(secs => $1::double precision / 1000)
             ON CONFLICT (device_id, alert_type) WHERE status <> 'resolved' DO NOTHING
             RETURNING {ALERT_COLUMNS}"
        ))
        .bind(OFFLINE_AFTER_MS)
        .fetch_all(db)
        .await?;
        for row in &opened {
            self.publish(LiveEventType::AlertOpened, row);
        }

        let escalated: Vec<AlertRow> = sqlx::query_as(&format!(
            "UPDATE alerts SET escalated_at = now(), updated_at = now(),
                    severity = CASE WHEN severity = 'warning' THEN 'critical' ELSE severity END
             WHERE status = 'open' AND escalated_at IS NULL AND (
                   (severity = 'warning' AND raised_at < now() - make_interval(secs => $1::double precision / 1000))
                OR (severity = 'critical' AND raised_at < now() - make_interval(secs => $2::double precision / 1000)))
             RETURNING {ALERT_COLUMNS}"
        ))
        .bind(ESCALATE_WARNING_AFTER_MS)
        .bind(ESCALATE_CRITICAL_AFTER_MS)
        .fetch_all(db)
        .await?;
        if !escalated.is_empty() {
            // Escalated critical alerts would page the on-call clinician through
            // FCM here; push delivery lands in S5.
            tracing::warn!(count = escalated.len(), "alerts escalated");
            let mut devices = self.devices.lock().await;
            for row in &escalated {
                if let (Some(rules), Some(t)) = (
                    devices.get_mut(&row.device_id),
                    AlertType::parse(&row.alert_type),
                ) && let Some(slot) = rules.slot(t)
                {
                    *slot = RuleState::open(Severity::Critical);
                }
            }
        }
        for row in &escalated {
            self.publish(LiveEventType::AlertUpdated, row);
        }
        Ok(())
    }

    /// Runs [`periodic`](Self::periodic) every `every` until the process stops.
    pub async fn run_periodic(&self, db: PgPool, every: Duration) {
        let mut tick = tokio::time::interval(every);
        loop {
            tick.tick().await;
            if let Err(e) = self.periodic(&db).await {
                tracing::error!(error = %e, "periodic alert check failed");
            }
        }
    }
}

async fn restore(db: &PgPool, device_id: &str) -> Result<DeviceRules, sqlx::Error> {
    let open: Vec<(String, String)> = sqlx::query_as(
        "SELECT alert_type, severity FROM alerts WHERE device_id = $1 AND status <> 'resolved'",
    )
    .bind(device_id)
    .fetch_all(db)
    .await?;
    let mut rules = DeviceRules::default();
    for (t, s) in open {
        if let (Some(t), Some(s)) = (AlertType::parse(&t), Severity::parse(&s))
            && let Some(slot) = rules.slot(t)
        {
            *slot = RuleState::open(s);
        }
    }
    Ok(rules)
}
