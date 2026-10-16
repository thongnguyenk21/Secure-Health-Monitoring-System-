//! Alert engine: rules on incoming vitals, edge anomalies, offline devices and escalation.
//!
//! - [`rules`]: pure threshold rules with hysteresis.
//! - [`engine`]: applies rule transitions to the `alerts` table and publishes
//!   them on the live stream; runs the periodic offline and escalation checks.
//! - [`model`]: the alert row and its API shape.

pub mod engine;
pub mod model;
pub mod rules;

pub use engine::AlertEngine;
pub use model::{AlertDto, LiveEvent};
