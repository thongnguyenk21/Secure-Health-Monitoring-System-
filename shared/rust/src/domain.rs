//! Hand-written domain types: identifiers, timestamps and vital signs.
//!
//! Wire formats (Protobuf) are generated later from `shared/protocols`; these
//! types are the validated, in-memory model that application code uses.

use serde::{Deserialize, Serialize};

/// Summary vitals sent about once per second per node (decision D18).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SummaryVitals {
    /// Which device produced the measurement.
    pub device_id: DeviceId,
    /// When the measurement was taken (UTC).
    pub ts: UnixMs,
    /// Heart rate in beats per minute, if it could be estimated.
    pub hr_bpm: Option<u16>,
    /// Blood oxygen saturation in percent, if it could be estimated.
    pub spo2_pct: Option<u8>,
    /// Movement class.
    pub activity: Activity,
    /// Confidence in the numbers above.
    pub quality: SignalQuality,
}

impl SummaryVitals {
    /// Checks that every present value is physiologically plausible.
    ///
    /// # Errors
    /// Returns [`DomainError::HeartRateOutOfRange`] or
    /// [`DomainError::Spo2OutOfRange`] for the first offending field.
    pub fn validate(&self) -> Result<(), DomainError> {
        if let Some(hr) = self.hr_bpm {
            if !(20..=250).contains(&hr) {
                return Err(DomainError::HeartRateOutOfRange(hr));
            }
        }
        if let Some(spo2) = self.spo2_pct {
            if !(50..=100).contains(&spo2) {
                return Err(DomainError::Spo2OutOfRange(spo2));
            }
        }
        Ok(())
    }
}

/// Key that makes uploads idempotent: the cloud ignores a repeat of the same key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdempotencyKey {
    /// Source device.
    pub device_id: DeviceId,
    /// Start of the window this record belongs to.
    pub window_start: UnixMs,
    /// Sequence number within that window.
    pub seq: u32,
}

/// Errors raised when a value violates a domain rule.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    /// The device id is not 32 lowercase hexadecimal characters.
    #[error("invalid device id: {0:?} (expected 32 lowercase hex characters)")]
    InvalidDeviceId(String),
    /// A heart rate outside the plausible human range.
    #[error("heart rate {0} bpm is outside 20..=250")]
    HeartRateOutOfRange(u16),
    /// An oxygen saturation above 100 percent.
    #[error("spo2 {0} % is outside 50..=100")]
    Spo2OutOfRange(u8),
}

/// A pseudonymous device identifier: 32 lowercase hex characters (a hash).
///
/// It never contains personal data; the real device serial is hashed before
/// it leaves the gateway.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DeviceId(String);

impl DeviceId {
    /// Creates a device id after validating its format.
    ///
    /// # Arguments
    /// * `raw` - the candidate id, e.g. `"0123456789abcdef0123456789abcdef"`.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidDeviceId`] if `raw` is not exactly 32
    /// lowercase hexadecimal characters.
    pub fn new(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        let ok = raw.len() == 32 && raw.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if ok {
            Ok(Self(raw))
        } else {
            Err(DomainError::InvalidDeviceId(raw))
        }
    }

    /// Returns the id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DeviceId {
    type Error = DomainError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DeviceId> for String {
    fn from(value: DeviceId) -> Self {
        value.0
    }
}

/// A point in time as milliseconds since the Unix epoch, always UTC.
///
/// One time model everywhere avoids timezone bugs between the MCU, the Pi and
/// the cloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UnixMs(pub i64);

/// What the wearer is doing, estimated from the accelerometer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    /// Little or no movement.
    Rest,
    /// Walking-level movement.
    Walk,
    /// Running-level movement.
    Run,
    /// The classifier could not decide.
    Unknown,
}

/// How trustworthy a measurement is, from 0 (useless) to 100 (clean).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SignalQuality(pub u8);

impl SignalQuality {
    /// Returns `true` if the quality is high enough to show to a user.
    #[must_use]
    pub fn is_usable(self) -> bool {
        self.0 >= 50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_ID: &str = "0123456789abcdef0123456789abcdef";

    fn vitals(hr: Option<u16>, spo2: Option<u8>) -> SummaryVitals {
        SummaryVitals {
            device_id: DeviceId::new(GOOD_ID).unwrap(),
            ts: UnixMs(1_700_000_000_000),
            hr_bpm: hr,
            spo2_pct: spo2,
            activity: Activity::Rest,
            quality: SignalQuality(90),
        }
    }

    #[test]
    fn device_id_accepts_32_lowercase_hex() {
        assert!(DeviceId::new(GOOD_ID).is_ok());
    }

    #[test]
    fn device_id_rejects_bad_input() {
        let upper = GOOD_ID.to_uppercase();
        let too_long = format!("{GOOD_ID}0");
        for bad in ["", "short", upper.as_str(), too_long.as_str()] {
            assert!(matches!(
                DeviceId::new(bad),
                Err(DomainError::InvalidDeviceId(_))
            ));
        }
    }

    #[test]
    fn vitals_validation_checks_ranges() {
        assert!(vitals(Some(72), Some(98)).validate().is_ok());
        assert_eq!(
            vitals(Some(10), None).validate(),
            Err(DomainError::HeartRateOutOfRange(10))
        );
        assert_eq!(
            vitals(None, Some(120)).validate(),
            Err(DomainError::Spo2OutOfRange(120))
        );
    }

    #[test]
    fn json_round_trip_keeps_value() {
        let original = vitals(Some(72), Some(98));
        let text = serde_json::to_string(&original).unwrap();
        let back: SummaryVitals = serde_json::from_str(&text).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn json_rejects_an_invalid_device_id() {
        let text = r#"{"device_id":"nope","ts":1,"hr_bpm":null,"spo2_pct":null,"activity":"rest","quality":50}"#;
        assert!(serde_json::from_str::<SummaryVitals>(text).is_err());
    }
}
