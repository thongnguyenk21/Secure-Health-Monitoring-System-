//! The alert row and its API representation (OpenAPI `Alert`, `LiveEvent`).

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::{AlertStatus, AlertType, Severity};
use crate::time::to_ms;

/// Columns selected for every alert query.
pub const ALERT_COLUMNS: &str = "id, device_id, patient_id, alert_type, severity, status, raised_at, value, acknowledged_at, acknowledged_by";

/// One row of the `alerts` table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AlertRow {
    /// Alert id.
    pub id: Uuid,
    /// Device that produced it.
    pub device_id: String,
    /// Patient the device was assigned to when the alert opened.
    pub patient_id: Option<Uuid>,
    /// Stored alert type spelling.
    pub alert_type: String,
    /// Stored severity spelling.
    pub severity: String,
    /// Stored status spelling.
    pub status: String,
    /// When it opened.
    pub raised_at: DateTime<Utc>,
    /// Value that crossed the threshold.
    pub value: Option<f64>,
    /// When it was acknowledged.
    pub acknowledged_at: Option<DateTime<Utc>>,
    /// Who acknowledged it.
    pub acknowledged_by: Option<Uuid>,
}

/// An alert as the API returns it.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AlertDto {
    /// Alert id.
    pub id: Uuid,
    /// Pseudonymous device id.
    pub device_id: String,
    /// Kind of alert.
    pub alert_type: AlertType,
    /// Severity.
    pub severity: Severity,
    /// Life-cycle status.
    pub status: AlertStatus,
    /// When it opened, epoch ms.
    pub raised_at_unix_ms: i64,
    /// Value that crossed the threshold (bpm or percent).
    pub value: Option<f64>,
    /// When it was acknowledged, epoch ms.
    pub acknowledged_at_unix_ms: Option<i64>,
    /// Who acknowledged it.
    pub acknowledged_by: Option<Uuid>,
}

impl AlertRow {
    /// Converts the row to the API shape.
    ///
    /// # Returns
    /// `None` if a stored enum spelling is unknown, which the `CHECK`
    /// constraints prevent.
    #[must_use]
    pub fn to_dto(&self) -> Option<AlertDto> {
        Some(AlertDto {
            id: self.id,
            device_id: self.device_id.clone(),
            alert_type: AlertType::parse(&self.alert_type)?,
            severity: Severity::parse(&self.severity)?,
            status: AlertStatus::parse(&self.status)?,
            raised_at_unix_ms: to_ms(self.raised_at),
            value: self.value,
            acknowledged_at_unix_ms: self.acknowledged_at.map(to_ms),
            acknowledged_by: self.acknowledged_by,
        })
    }
}

/// Kind of live event.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveEventType {
    /// A new alert.
    AlertOpened,
    /// An alert changed status or severity.
    AlertUpdated,
}

/// One frame of the live WebSocket.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LiveEvent {
    /// What happened.
    #[serde(rename = "type")]
    pub kind: LiveEventType,
    /// The alert after the change.
    pub alert: AlertDto,
}
