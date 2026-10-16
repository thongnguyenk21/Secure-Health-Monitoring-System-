//! Enumerations stored in the database and exchanged over the REST API.
//!
//! Each enum maps to a `TEXT` column with a `CHECK` constraint; `as_str`
//! methods give the stored spelling and `parse` reads it back.

use serde::{Deserialize, Serialize};

/// Generates `as_str` and `parse` for a string-backed enum.
macro_rules! text_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($(#[$vmeta])* $variant),+
        }

        impl $name {
            /// Returns the spelling used in the database and the API.
            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            /// Reads the database or API spelling.
            ///
            /// # Returns
            /// `None` for an unknown spelling.
            #[must_use]
            pub fn parse(text: &str) -> Option<Self> {
                match text {
                    $($text => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}

text_enum!(
    /// Role of an account (D26): three roles only.
    Role {
        /// Wears a device; uses the mobile app; sees only their own data.
        Patient => "patient",
        /// Reads every patient of the tenant; acknowledges alerts.
        Clinician => "clinician",
        /// Manages devices, users and models; sees no health data.
        Admin => "admin",
    }
);

text_enum!(
    /// Kind of alert, as in the OpenAPI `AlertType`.
    AlertType {
        /// Heart rate above the high threshold.
        HrHigh => "hr_high",
        /// Heart rate below the low threshold.
        HrLow => "hr_low",
        /// Oxygen saturation below the threshold.
        Spo2Low => "spo2_low",
        /// The edge model flagged a window as abnormal.
        Anomaly => "anomaly",
        /// No data from an assigned wearable for 5 minutes.
        DeviceOffline => "device_offline",
        /// Wearable battery low (not raised yet: battery is not in the telemetry).
        BatteryLow => "battery_low",
    }
);

text_enum!(
    /// Severity of an alert.
    Severity {
        /// For information only.
        Info => "info",
        /// Needs attention.
        Warning => "warning",
        /// Needs action now.
        Critical => "critical",
    }
);

impl Severity {
    /// Orders severities: a higher rank is more urgent.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Warning => 1,
            Self::Critical => 2,
        }
    }
}

text_enum!(
    /// Life cycle of an alert.
    AlertStatus {
        /// Raised and not yet seen by a person.
        Open => "open",
        /// Seen by the patient or a clinician; escalation stops.
        Acknowledged => "acknowledged",
        /// The condition cleared.
        Resolved => "resolved",
    }
);

text_enum!(
    /// Device kind.
    DeviceKind {
        /// Body-worn sensor node.
        Wearable => "wearable",
        /// Raspberry Pi gateway at home.
        Gateway => "gateway",
    }
);

text_enum!(
    /// Device life cycle.
    DeviceStatus {
        /// Registered, no valid data yet.
        Pending => "pending",
        /// Sending data.
        Active => "active",
        /// Refused from now on.
        Revoked => "revoked",
    }
);

text_enum!(
    /// Priority level of a patient in the clinician list, highest first.
    PatientStatus {
        /// An open critical alert.
        Critical => "critical",
        /// An open warning alert.
        Warning => "warning",
        /// The wearable has been silent for more than 5 minutes.
        Offline => "offline",
        /// Data arrives and nothing is open.
        Normal => "normal",
        /// No wearable is assigned.
        NoDevice => "no_device",
    }
);

impl PatientStatus {
    /// Orders levels for the clinician list: a lower value is shown first.
    #[must_use]
    pub const fn order(self) -> u8 {
        match self {
            Self::Critical => 0,
            Self::Warning => 1,
            Self::Offline => 2,
            Self::Normal => 3,
            Self::NoDevice => 4,
        }
    }
}

text_enum!(
    /// Scope of a consent decision (D32 adds `model_training`).
    ConsentScope {
        /// Collection and display for care.
        Monitoring => "monitoring",
        /// Pseudonymised research export.
        ResearchExport => "research_export",
        /// Use of the subject's records to train models.
        ModelTraining => "model_training",
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spellings_round_trip() {
        for role in [Role::Patient, Role::Clinician, Role::Admin] {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }
        assert_eq!(AlertType::parse("spo2_low"), Some(AlertType::Spo2Low));
        assert_eq!(Role::parse("researcher"), None);
    }

    #[test]
    fn serde_uses_the_same_spelling() {
        let json = serde_json::to_string(&AlertType::DeviceOffline).unwrap_or_default();
        assert_eq!(json, "\"device_offline\"");
    }

    #[test]
    fn critical_patients_come_first() {
        assert!(PatientStatus::Critical.order() < PatientStatus::Warning.order());
        assert!(PatientStatus::Offline.order() < PatientStatus::Normal.order());
        assert!(Severity::Critical.rank() > Severity::Warning.rank());
    }
}
