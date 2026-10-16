//! Cloud backend of the IoMT system (MVP-05).
//!
//! - [`api`]: REST API for the web and mobile apps (contract
//!   `shared/protocols/openapi/cloud-api.openapi.yaml`, version 0.2.0) and the
//!   live alert WebSocket.
//! - [`ingest`]: gRPC `TelemetryIngestService` for gateways (`iomt.telemetry.v1`).
//! - [`alerts`]: threshold rules with hysteresis, offline detection and escalation.
//! - [`infra`]: tokens, password hashing, the audit chain and rate limiting.
//! - [`domain`]: small enums shared by the modules above.

pub mod alerts;
pub mod api;
pub mod config;
pub mod domain;
pub mod error;
pub mod infra;
pub mod ingest;
pub mod seed;
pub mod state;
pub mod time;

/// Embedded SQL migrations from `cloud/backend/migrations`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
