//! Conversions between the generated wire types (`telemetry.v1`) and the domain.
//!
//! The direction matters. Domain → wire cannot fail: a domain value is already
//! valid. Wire → domain is where untrusted bytes become trusted values, so every
//! conversion goes through the domain constructors and [`SummaryVitals::validate`];
//! code that receives telemetry never handles an unchecked heart rate or id.

use crate::domain::{
    Activity, DeviceId, DomainError, IdempotencyKey, SignalQuality, SummaryVitals, UnixMs,
};
use crate::generated::iomt::telemetry::v1 as pb;

/// Why a wire message could not become a domain value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConversionError {
    /// A message field that the contract requires was not set.
    #[error("required field `{0}` is missing")]
    MissingField(&'static str),
    /// An `Activity` that is unset or not defined in this contract version.
    #[error("activity value {0} is unset or unknown")]
    InvalidActivity(i32),
    /// A number too large for the domain type, before any range check.
    #[error("field `{field}` = {value} is too large")]
    TooLarge {
        /// Wire field name.
        field: &'static str,
        /// Value as received.
        value: u32,
    },
    /// The value is well formed but breaks a domain rule.
    #[error(transparent)]
    Domain(#[from] DomainError),
}

impl ConversionError {
    /// Returns the reason the backend reports to the gateway for this record.
    ///
    /// Every conversion failure is a property of the record itself, so resending
    /// it would fail again: the gateway must dead-letter it.
    #[must_use]
    pub fn reject_reason(&self) -> pb::RejectReason {
        pb::RejectReason::InvalidValue
    }
}

impl TryFrom<pb::RecordKey> for IdempotencyKey {
    type Error = ConversionError;
    fn try_from(key: pb::RecordKey) -> Result<Self, Self::Error> {
        if key.boot_id == 0 {
            return Err(ConversionError::MissingField("key.boot_id"));
        }
        Ok(Self {
            device_id: DeviceId::new(key.device_id)?,
            window_start: UnixMs(key.window_start_unix_ms),
            seq: key.seq,
            boot_id: key.boot_id,
        })
    }
}

impl From<IdempotencyKey> for pb::RecordKey {
    fn from(key: IdempotencyKey) -> Self {
        Self {
            device_id: key.device_id.into(),
            window_start_unix_ms: key.window_start.0,
            seq: key.seq,
            boot_id: key.boot_id,
        }
    }
}

impl TryFrom<pb::Activity> for Activity {
    type Error = ConversionError;
    fn try_from(activity: pb::Activity) -> Result<Self, Self::Error> {
        match activity {
            pb::Activity::Rest => Ok(Self::Rest),
            pb::Activity::Walk => Ok(Self::Walk),
            pb::Activity::Run => Ok(Self::Run),
            pb::Activity::Unknown => Ok(Self::Unknown),
            pb::Activity::Unspecified => Err(ConversionError::InvalidActivity(activity as i32)),
        }
    }
}

impl From<Activity> for pb::Activity {
    fn from(activity: Activity) -> Self {
        match activity {
            Activity::Rest => Self::Rest,
            Activity::Walk => Self::Walk,
            Activity::Run => Self::Run,
            Activity::Unknown => Self::Unknown,
        }
    }
}

/// Turns a received `SummaryVitals` into its idempotency key and domain value.
///
/// The device id of the vitals is the one in the key, so the two cannot disagree.
///
/// # Arguments
/// * `msg` - the message as decoded from the wire.
///
/// # Returns
/// The key used to store the record at most once, and the validated vitals.
///
/// # Errors
/// Returns [`ConversionError`] if the key is missing, a number does not fit,
/// the activity is unset or unknown, or a domain rule fails (including the
/// heart rate and SpO2 ranges checked by [`SummaryVitals::validate`]).
pub fn vitals_from_pb(
    msg: pb::SummaryVitals,
) -> Result<(IdempotencyKey, SummaryVitals), ConversionError> {
    let activity_raw = msg.activity;
    let activity = pb::Activity::try_from(activity_raw)
        .map_err(|_| ConversionError::InvalidActivity(activity_raw))
        .and_then(Activity::try_from)?;
    let key = IdempotencyKey::try_from(msg.key.ok_or(ConversionError::MissingField("key"))?)?;
    let hr_bpm = msg
        .hr_bpm
        .map(|v| u16::try_from(v).map_err(|_| too_large("hr_bpm", v)))
        .transpose()?;
    let spo2_pct = msg
        .spo2_pct
        .map(|v| u8::try_from(v).map_err(|_| too_large("spo2_pct", v)))
        .transpose()?;
    let quality = u8::try_from(msg.signal_quality)
        .map_err(|_| too_large("signal_quality", msg.signal_quality))?;

    let vitals = SummaryVitals {
        device_id: key.device_id.clone(),
        ts: UnixMs(msg.ts_unix_ms),
        hr_bpm,
        spo2_pct,
        activity,
        quality: SignalQuality::new(quality)?,
    };
    vitals.validate()?;
    Ok((key, vitals))
}

/// Builds the wire message for vitals about to be uploaded.
///
/// # Arguments
/// * `key` - idempotency key of the record; its device id is the one sent.
/// * `vitals` - validated vitals.
///
/// # Returns
/// The message ready to put in an `UploadBatchRequest`.
#[must_use]
pub fn vitals_to_pb(key: IdempotencyKey, vitals: &SummaryVitals) -> pb::SummaryVitals {
    pb::SummaryVitals {
        key: Some(key.into()),
        ts_unix_ms: vitals.ts.0,
        hr_bpm: vitals.hr_bpm.map(u32::from),
        spo2_pct: vitals.spo2_pct.map(u32::from),
        activity: pb::Activity::from(vitals.activity) as i32,
        signal_quality: u32::from(vitals.quality.value()),
    }
}

fn too_large(field: &'static str, value: u32) -> ConversionError {
    ConversionError::TooLarge { field, value }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_ID: &str = "0123456789abcdef0123456789abcdef";

    fn key() -> IdempotencyKey {
        IdempotencyKey {
            device_id: DeviceId::new(GOOD_ID).unwrap(),
            window_start: UnixMs(1_700_000_000_000),
            seq: 7,
            boot_id: 0xA1B2_C3D4,
        }
    }

    fn wire() -> pb::SummaryVitals {
        pb::SummaryVitals {
            key: Some(key().into()),
            ts_unix_ms: 1_700_000_000_500,
            hr_bpm: Some(72),
            spo2_pct: Some(98),
            activity: pb::Activity::Walk as i32,
            signal_quality: 90,
        }
    }

    #[test]
    fn valid_vitals_survive_a_round_trip() {
        let (k, vitals) = vitals_from_pb(wire()).unwrap();
        assert_eq!(k, key());
        assert_eq!(vitals.activity, Activity::Walk);
        assert_eq!(vitals_to_pb(k, &vitals), wire());
    }

    #[test]
    fn missing_values_stay_missing() {
        let mut msg = wire();
        msg.hr_bpm = None;
        msg.spo2_pct = None;
        let (_, vitals) = vitals_from_pb(msg).unwrap();
        assert_eq!((vitals.hr_bpm, vitals.spo2_pct), (None, None));
    }

    #[test]
    fn a_missing_key_is_rejected() {
        let mut msg = wire();
        msg.key = None;
        assert_eq!(
            vitals_from_pb(msg),
            Err(ConversionError::MissingField("key"))
        );
    }

    #[test]
    fn a_key_without_a_boot_id_is_rejected() {
        let mut msg = wire();
        if let Some(key) = msg.key.as_mut() {
            key.boot_id = 0;
        }
        assert_eq!(
            vitals_from_pb(msg),
            Err(ConversionError::MissingField("key.boot_id"))
        );
    }

    #[test]
    fn the_same_seq_in_two_boots_gives_two_keys() {
        let mut other = wire();
        if let Some(key) = other.key.as_mut() {
            key.boot_id = 0x0000_0042;
        }
        let (a, _) = vitals_from_pb(wire()).unwrap();
        let (b, _) = vitals_from_pb(other).unwrap();
        assert_eq!((a.seq, a.window_start), (b.seq, b.window_start));
        assert_ne!(a, b, "a reboot inside one window must not collide");
    }

    #[test]
    fn unset_and_unknown_activities_are_rejected() {
        for raw in [pb::Activity::Unspecified as i32, 99] {
            let mut msg = wire();
            msg.activity = raw;
            assert_eq!(
                vitals_from_pb(msg),
                Err(ConversionError::InvalidActivity(raw))
            );
        }
    }

    #[test]
    fn domain_rules_apply_to_wire_values() {
        let mut msg = wire();
        msg.hr_bpm = Some(999);
        assert_eq!(
            vitals_from_pb(msg),
            Err(ConversionError::Domain(DomainError::HeartRateOutOfRange(
                999
            )))
        );

        let mut msg = wire();
        msg.signal_quality = 101;
        assert_eq!(
            vitals_from_pb(msg),
            Err(ConversionError::Domain(DomainError::QualityOutOfRange(101)))
        );

        let mut msg = wire();
        msg.key = Some(pb::RecordKey {
            device_id: "SERIAL-123".into(),
            window_start_unix_ms: 0,
            seq: 0,
            boot_id: 1,
        });
        assert_eq!(
            vitals_from_pb(msg),
            Err(ConversionError::Domain(DomainError::InvalidDeviceId {
                len: 10
            }))
        );
    }

    #[test]
    fn numbers_too_large_for_the_domain_type_are_rejected() {
        let mut msg = wire();
        msg.hr_bpm = Some(70_000);
        assert_eq!(
            vitals_from_pb(msg),
            Err(ConversionError::TooLarge {
                field: "hr_bpm",
                value: 70_000
            })
        );
    }

    #[test]
    fn every_conversion_failure_is_dead_lettered() {
        let err = ConversionError::MissingField("key");
        assert_eq!(err.reject_reason(), pb::RejectReason::InvalidValue);
    }
}
