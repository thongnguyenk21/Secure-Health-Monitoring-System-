//! Shared domain types for the IoMT system.
//!
//! Every service (edge, cloud, tools) depends on this crate so that a
//! "heart rate" or a "device id" means exactly the same thing everywhere.

pub mod domain;

pub use domain::{
    Activity, DeviceId, DomainError, IdempotencyKey, SignalQuality, SummaryVitals, UnixMs,
};
