//! Link protocol negotiation of the BLE handshake (REQ-REL-029).
//!
//! The wearable reports the range of link protocol versions it speaks; the Pi
//! picks the newest version both sides know, or refuses the wearable. Keeping the
//! rule here, next to the contract, means edge-ingest and the device simulator
//! decide the same way.

use crate::generated::iomt::firmware::v1 as pb;

/// Inclusive range of link protocol versions, `min..=max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtoRange {
    /// Oldest supported version.
    pub min: u32,
    /// Newest supported version.
    pub max: u32,
}

/// Why no version could be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LinkRefusal {
    /// Every version of the wearable is newer than the Pi: update the Pi first.
    #[error("wearable protocol is newer than the gateway; update the gateway")]
    NodeTooNew,
    /// Every version of the wearable is older than the Pi supports: reflash it.
    #[error("wearable protocol is older than the gateway supports; reflash the wearable")]
    NodeTooOld,
    /// The wearable reported `min > max`.
    #[error("wearable reported an empty protocol range")]
    InvalidRange,
    /// The wearable sent `boot_id` 0, so a reboot could not be detected.
    #[error("wearable sent no boot id; update its firmware")]
    MissingBootId,
}

/// Chooses the link protocol version for a connection.
///
/// # Arguments
/// * `gateway` - versions this Pi speaks.
/// * `node` - versions the wearable reported in its `LinkReport`.
///
/// # Returns
/// The newest version present in both ranges.
///
/// # Errors
/// Returns [`LinkRefusal`] when the ranges do not overlap or the wearable's
/// range is empty; the Pi must then refuse the wearable's data and raise an alarm.
pub fn negotiate(gateway: ProtoRange, node: ProtoRange) -> Result<u32, LinkRefusal> {
    if node.min > node.max {
        return Err(LinkRefusal::InvalidRange);
    }
    if node.min > gateway.max {
        return Err(LinkRefusal::NodeTooNew);
    }
    if node.max < gateway.min {
        return Err(LinkRefusal::NodeTooOld);
    }
    Ok(node.max.min(gateway.max))
}

/// Builds the `LinkSelect` that ends a handshake.
///
/// # Arguments
/// * `session_id` - id from the `LinkHello` that started the handshake.
/// * `outcome` - result of [`negotiate`].
#[must_use]
pub fn select_message(session_id: u32, outcome: Result<u32, LinkRefusal>) -> pb::LinkSelect {
    let outcome = match outcome {
        Ok(version) => pb::link_select::Outcome::ProtoSelected(version),
        Err(refusal) => pb::link_select::Outcome::Refusal(pb::LinkRefusal::from(refusal) as i32),
    };
    pb::LinkSelect {
        session_id,
        outcome: Some(outcome),
        fw_target: None,
    }
}

impl From<LinkRefusal> for pb::LinkRefusal {
    fn from(refusal: LinkRefusal) -> Self {
        match refusal {
            LinkRefusal::NodeTooNew => Self::NodeTooNew,
            LinkRefusal::NodeTooOld => Self::NodeTooOld,
            LinkRefusal::InvalidRange => Self::InvalidRange,
            LinkRefusal::MissingBootId => Self::MissingBootId,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn range(min: u32, max: u32) -> ProtoRange {
        ProtoRange { min, max }
    }

    #[test]
    fn picks_the_newest_common_version() {
        assert_eq!(negotiate(range(1, 3), range(2, 5)), Ok(3));
        assert_eq!(negotiate(range(2, 5), range(1, 3)), Ok(3));
        assert_eq!(negotiate(range(1, 1), range(1, 2)), Ok(1));
    }

    #[test]
    fn touching_ranges_still_agree() {
        assert_eq!(negotiate(range(1, 2), range(2, 4)), Ok(2));
        assert_eq!(negotiate(range(2, 4), range(1, 2)), Ok(2));
    }

    #[test]
    fn a_newer_wearable_is_blocked_until_the_pi_is_updated() {
        assert_eq!(
            negotiate(range(1, 2), range(3, 4)),
            Err(LinkRefusal::NodeTooNew)
        );
    }

    #[test]
    fn an_older_wearable_must_be_reflashed() {
        assert_eq!(
            negotiate(range(3, 4), range(1, 2)),
            Err(LinkRefusal::NodeTooOld)
        );
    }

    #[test]
    fn an_empty_wearable_range_is_refused() {
        assert_eq!(
            negotiate(range(1, 9), range(3, 2)),
            Err(LinkRefusal::InvalidRange)
        );
    }

    #[test]
    fn every_refusal_has_a_wire_reason() {
        for (refusal, wire) in [
            (LinkRefusal::NodeTooNew, pb::LinkRefusal::NodeTooNew),
            (LinkRefusal::NodeTooOld, pb::LinkRefusal::NodeTooOld),
            (LinkRefusal::InvalidRange, pb::LinkRefusal::InvalidRange),
            (LinkRefusal::MissingBootId, pb::LinkRefusal::MissingBootId),
        ] {
            assert_eq!(pb::LinkRefusal::from(refusal), wire);
        }
    }

    #[test]
    fn the_select_message_carries_the_outcome() {
        let ok = select_message(7, Ok(2));
        assert_eq!(ok.session_id, 7);
        assert_eq!(ok.outcome, Some(pb::link_select::Outcome::ProtoSelected(2)));

        let refused = select_message(7, Err(LinkRefusal::NodeTooNew));
        assert_eq!(
            refused.outcome,
            Some(pb::link_select::Outcome::Refusal(
                pb::LinkRefusal::NodeTooNew as i32
            ))
        );
    }
}
