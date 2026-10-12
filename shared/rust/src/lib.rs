//! Shared domain types and gRPC contracts for the IoMT system.
//!
//! Every service (edge, cloud, tools) depends on this crate so that a
//! "heart rate" or a "device id" means exactly the same thing everywhere.
//!
//! - [`domain`]: hand-written, validated types used by application code.
//! - [`generated`]: wire types and gRPC stubs produced from `shared/protocols`.
//! - [`convert`]: the only path from wire types to domain types.

pub mod convert;
pub mod domain;

/// Code generated from `shared/protocols/proto`. Do not edit; run
/// `tools/scripts/gen-contracts.sh` after changing a contract.
#[allow(missing_docs, clippy::all, clippy::pedantic)]
pub mod generated;

pub use convert::{ConversionError, vitals_from_pb, vitals_to_pb};
pub use domain::{
    Activity, DeviceId, DomainError, IdempotencyKey, SignalQuality, SummaryVitals, UnixMs,
};
/// Telemetry contract `iomt.telemetry.v1`: messages and the ingest service.
pub use generated::iomt::telemetry::v1 as telemetry_v1;
