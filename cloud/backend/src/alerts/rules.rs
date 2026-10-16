//! Threshold rules with hysteresis, as pure functions (slide "Alert rule có hysteresis").
//!
//! A rule opens when the value stays past its threshold for `on_ms`, and
//! resolves only when the value stays back past a *different* clear threshold
//! for `off_ms`. The gap between the two thresholds (the hysteresis margin)
//! stops an alert from flapping while the heart rate hovers around the limit.
//! Samples with a signal quality below 50 are ignored: a noisy sensor must not
//! raise or clear an alert.

use crate::domain::{AlertType, Severity};

/// Thresholds of the vital-sign rules. The defaults are placeholders that
/// clinicians must confirm (open item in the design review).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    /// Heart rate that opens `hr_high`, bpm (strictly above).
    pub hr_high_bpm: u16,
    /// Heart rate under which `hr_high` clears, bpm (strictly below).
    pub hr_high_clear_bpm: u16,
    /// Heart rate from which `hr_high` is critical, bpm.
    pub hr_high_critical_bpm: u16,
    /// Heart rate that opens `hr_low`, bpm (strictly below).
    pub hr_low_bpm: u16,
    /// Heart rate above which `hr_low` clears, bpm (strictly above).
    pub hr_low_clear_bpm: u16,
    /// Heart rate under which `hr_low` is critical, bpm.
    pub hr_low_critical_bpm: u16,
    /// SpO₂ that opens `spo2_low`, percent (strictly below).
    pub spo2_low_pct: u16,
    /// SpO₂ above which `spo2_low` clears, percent (strictly above).
    pub spo2_low_clear_pct: u16,
    /// SpO₂ under which `spo2_low` is critical, percent.
    pub spo2_low_critical_pct: u16,
    /// How long a value must stay past the threshold to open, ms (`T_on`).
    pub on_ms: i64,
    /// How long a value must stay past the clear threshold to resolve, ms (`T_off`).
    pub off_ms: i64,
    /// Lowest signal quality whose samples are considered, 0..=100.
    pub min_quality: u8,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            hr_high_bpm: 140,
            hr_high_clear_bpm: 130,
            hr_high_critical_bpm: 170,
            hr_low_bpm: 40,
            hr_low_clear_bpm: 45,
            hr_low_critical_bpm: 35,
            spo2_low_pct: 90,
            spo2_low_clear_pct: 92,
            spo2_low_critical_pct: 85,
            on_ms: 10_000,
            off_ms: 30_000,
            min_quality: 50,
        }
    }
}

/// A change an evaluation asks the engine to apply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Transition {
    /// Open a new alert.
    Open {
        /// Kind of alert.
        alert_type: AlertType,
        /// Initial severity.
        severity: Severity,
        /// Value that crossed the threshold (bpm or percent).
        value: f64,
        /// Time of the sample that completed `T_on`, epoch ms.
        at_ms: i64,
    },
    /// Raise an open alert to critical.
    Raise {
        /// Kind of alert.
        alert_type: AlertType,
        /// Value that crossed the critical threshold.
        value: f64,
    },
    /// Resolve the open alert of this kind.
    Resolve {
        /// Kind of alert.
        alert_type: AlertType,
        /// Time of the sample that completed `T_off`, epoch ms.
        at_ms: i64,
    },
}

/// Which side of the threshold counts as abnormal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Above,
    Below,
}

/// State of one rule for one device.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuleState {
    breach_since_ms: Option<i64>,
    clear_since_ms: Option<i64>,
    active: Option<Severity>,
}

impl RuleState {
    /// State of a rule whose alert is already open, e.g. after a restart.
    #[must_use]
    pub const fn open(severity: Severity) -> Self {
        Self {
            breach_since_ms: None,
            clear_since_ms: None,
            active: Some(severity),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Rule {
    alert_type: AlertType,
    direction: Direction,
    threshold: u16,
    clear: u16,
    critical: u16,
}

impl Rule {
    fn breaches(self, v: u16) -> bool {
        match self.direction {
            Direction::Above => v > self.threshold,
            Direction::Below => v < self.threshold,
        }
    }
    fn clears(self, v: u16) -> bool {
        match self.direction {
            Direction::Above => v < self.clear,
            Direction::Below => v > self.clear,
        }
    }
    fn severity(self, v: u16) -> Severity {
        let critical = match self.direction {
            Direction::Above => v >= self.critical,
            Direction::Below => v <= self.critical,
        };
        if critical {
            Severity::Critical
        } else {
            Severity::Warning
        }
    }

    fn step(self, st: &mut RuleState, v: u16, at_ms: i64, t: &Thresholds) -> Option<Transition> {
        match st.active {
            None => {
                if !self.breaches(v) {
                    st.breach_since_ms = None;
                    return None;
                }
                let since = *st.breach_since_ms.get_or_insert(at_ms);
                if at_ms - since < t.on_ms {
                    return None;
                }
                let severity = self.severity(v);
                *st = RuleState::open(severity);
                Some(Transition::Open {
                    alert_type: self.alert_type,
                    severity,
                    value: f64::from(v),
                    at_ms,
                })
            }
            Some(current) => {
                if self.clears(v) {
                    let since = *st.clear_since_ms.get_or_insert(at_ms);
                    if at_ms - since < t.off_ms {
                        return None;
                    }
                    *st = RuleState::default();
                    return Some(Transition::Resolve {
                        alert_type: self.alert_type,
                        at_ms,
                    });
                }
                st.clear_since_ms = None;
                if current == Severity::Warning && self.severity(v) == Severity::Critical {
                    st.active = Some(Severity::Critical);
                    return Some(Transition::Raise {
                        alert_type: self.alert_type,
                        value: f64::from(v),
                    });
                }
                None
            }
        }
    }
}

/// One summary sample as the rules see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Measurement time, epoch ms.
    pub ts_ms: i64,
    /// Heart rate, bpm.
    pub hr_bpm: Option<u16>,
    /// Oxygen saturation, percent.
    pub spo2_pct: Option<u16>,
    /// Signal quality, 0..=100.
    pub quality: u8,
}

/// Rule states of one device.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeviceRules {
    /// `hr_high` state.
    pub hr_high: RuleState,
    /// `hr_low` state.
    pub hr_low: RuleState,
    /// `spo2_low` state.
    pub spo2_low: RuleState,
}

impl DeviceRules {
    /// Returns the state slot of a vital-sign alert type, if it has one.
    pub fn slot(&mut self, alert_type: AlertType) -> Option<&mut RuleState> {
        match alert_type {
            AlertType::HrHigh => Some(&mut self.hr_high),
            AlertType::HrLow => Some(&mut self.hr_low),
            AlertType::Spo2Low => Some(&mut self.spo2_low),
            _ => None,
        }
    }

    /// Feeds one sample through every rule.
    ///
    /// Samples must arrive in time order per device.
    ///
    /// # Returns
    /// The transitions to apply, in rule order.
    pub fn evaluate(&mut self, s: Sample, t: &Thresholds) -> Vec<Transition> {
        if s.quality < t.min_quality {
            return Vec::new();
        }
        let hr_high = Rule {
            alert_type: AlertType::HrHigh,
            direction: Direction::Above,
            threshold: t.hr_high_bpm,
            clear: t.hr_high_clear_bpm,
            critical: t.hr_high_critical_bpm,
        };
        let hr_low = Rule {
            alert_type: AlertType::HrLow,
            direction: Direction::Below,
            threshold: t.hr_low_bpm,
            clear: t.hr_low_clear_bpm,
            critical: t.hr_low_critical_bpm,
        };
        let spo2_low = Rule {
            alert_type: AlertType::Spo2Low,
            direction: Direction::Below,
            threshold: t.spo2_low_pct,
            clear: t.spo2_low_clear_pct,
            critical: t.spo2_low_critical_pct,
        };
        let mut out = Vec::new();
        if let Some(hr) = s.hr_bpm {
            out.extend(hr_high.step(&mut self.hr_high, hr, s.ts_ms, t));
            out.extend(hr_low.step(&mut self.hr_low, hr, s.ts_ms, t));
        }
        if let Some(spo2) = s.spo2_pct {
            out.extend(spo2_low.step(&mut self.spo2_low, spo2, s.ts_ms, t));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(sec: i64, hr: u16) -> Sample {
        Sample {
            ts_ms: sec * 1000,
            hr_bpm: Some(hr),
            spo2_pct: Some(97),
            quality: 90,
        }
    }

    fn run(rules: &mut DeviceRules, from: i64, to: i64, hr: u16) -> Vec<Transition> {
        (from..to)
            .flat_map(|s| rules.evaluate(sample(s, hr), &Thresholds::default()))
            .collect()
    }

    #[test]
    fn a_short_spike_does_not_open() {
        let mut r = DeviceRules::default();
        assert!(run(&mut r, 0, 5, 150).is_empty());
        assert!(run(&mut r, 5, 20, 100).is_empty());
    }

    #[test]
    fn a_sustained_breach_opens_once() {
        let mut r = DeviceRules::default();
        let out = run(&mut r, 0, 30, 150);
        assert_eq!(
            out,
            vec![Transition::Open {
                alert_type: AlertType::HrHigh,
                severity: Severity::Warning,
                value: 150.0,
                at_ms: 10_000
            }]
        );
    }

    #[test]
    fn hovering_inside_the_margin_does_not_flap() {
        let mut r = DeviceRules::default();
        run(&mut r, 0, 11, 150);
        // 135 is below the open threshold but above the clear threshold.
        assert!(run(&mut r, 11, 200, 135).is_empty());
        assert_eq!(r.hr_high.active, Some(Severity::Warning));
    }

    #[test]
    fn a_sustained_clear_resolves() {
        let mut r = DeviceRules::default();
        run(&mut r, 0, 11, 150);
        let out = run(&mut r, 11, 60, 100);
        assert_eq!(
            out,
            vec![Transition::Resolve {
                alert_type: AlertType::HrHigh,
                at_ms: 41_000
            }]
        );
    }

    #[test]
    fn a_critical_value_raises_an_open_warning() {
        let mut r = DeviceRules::default();
        run(&mut r, 0, 11, 150);
        let out = run(&mut r, 11, 13, 180);
        assert_eq!(
            out,
            vec![Transition::Raise {
                alert_type: AlertType::HrHigh,
                value: 180.0
            }]
        );
    }

    #[test]
    fn noisy_samples_are_ignored() {
        let mut r = DeviceRules::default();
        let t = Thresholds::default();
        for s in 0..30 {
            let mut x = sample(s, 160);
            x.quality = 20;
            assert!(r.evaluate(x, &t).is_empty());
        }
    }

    #[test]
    fn low_spo2_and_low_heart_rate_open_their_own_alerts() {
        let mut r = DeviceRules::default();
        let t = Thresholds::default();
        let mut out = Vec::new();
        for s in 0..12 {
            out.extend(r.evaluate(
                Sample {
                    ts_ms: s * 1000,
                    hr_bpm: Some(30),
                    spo2_pct: Some(84),
                    quality: 90,
                },
                &t,
            ));
        }
        let kinds: Vec<_> = out
            .iter()
            .map(|x| match x {
                Transition::Open {
                    alert_type,
                    severity,
                    ..
                } => (*alert_type, *severity),
                _ => (AlertType::Anomaly, Severity::Info),
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                (AlertType::HrLow, Severity::Critical),
                (AlertType::Spo2Low, Severity::Critical)
            ]
        );
    }
}
