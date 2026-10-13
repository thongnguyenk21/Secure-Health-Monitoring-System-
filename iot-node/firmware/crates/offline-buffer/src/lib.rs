//! Keeps samples on the wearable until the gateway confirms it has them.
//!
//! The wearable records one sample per second whether or not the BLE link is
//! up. Samples wait in a fixed-size ring buffer ([`RingBuffer`]). The
//! [`Courier`] numbers them, hands the oldest ones to the link layer, and
//! deletes them only when the gateway acknowledges their sequence number. When
//! the wearer stays out of range for too long, the oldest sample is overwritten
//! and counted as lost.
//!
//! The crate is `no_std` and never allocates, so it runs on the ESP32-C3 and,
//! unchanged, in host tests.
#![cfg_attr(not(test), no_std)]

mod courier;
mod ring;

pub use courier::{Courier, Sample};
pub use ring::RingBuffer;
