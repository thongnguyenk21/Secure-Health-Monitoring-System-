//! Plausible, reproducible vital signs for one simulated wearer.
//!
//! The wearer alternates rest, walking and running. Heart rate moves toward a
//! target set by the activity instead of jumping, signal quality drops with
//! movement, and a poor signal makes the values unavailable, as on a real PPG
//! sensor. Everything comes from a seeded generator: the same seed always gives
//! the same wearer.

use iomt_frames::Activity;

/// Small xorshift64* generator: deterministic and dependency-free.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Creates a generator; every seed, including 0, gives a usable state.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Returns a number in `0.0..1.0`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Returns a number in `lo..hi`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

/// One second of measurements, in the units of the 16-byte record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Heart rate in beats per minute; `0` means not available.
    pub hr_bpm: u16,
    /// Blood oxygen saturation in percent; `0` means not available.
    pub spo2_pct: u8,
    /// Steps during this second.
    pub steps: u16,
    /// Movement class.
    pub activity: Activity,
    /// Signal quality, 0 to 100.
    pub quality: u8,
}

/// Window, in seconds since start, when the heart rate is forced high.
#[derive(Debug, Clone, Copy)]
pub struct HighHrEpisode {
    /// Start, seconds since the simulation started.
    pub at_s: u32,
    /// Duration in seconds.
    pub for_s: u32,
}

/// State of one simulated wearer.
#[derive(Debug, Clone)]
pub struct Body {
    rng: Rng,
    rest_hr: f64,
    hr: f64,
    activity: Activity,
    activity_left_s: u32,
    high_hr: Option<HighHrEpisode>,
}

/// Below this quality the sensor cannot estimate heart rate or SpO2.
const MIN_QUALITY_FOR_VALUES: f64 = 40.0;
/// Heart rate targeted during a forced high episode, above the 140 bpm alert.
const HIGH_HR_TARGET_BPM: f64 = 155.0;

impl Body {
    /// Creates a wearer with a resting heart rate drawn from 58 to 78 bpm.
    ///
    /// # Arguments
    /// * `seed` - generator seed; one seed per simulated device.
    /// * `high_hr` - optional episode of abnormally high heart rate.
    #[must_use]
    pub fn new(seed: u64, high_hr: Option<HighHrEpisode>) -> Self {
        let mut rng = Rng::new(seed);
        let rest_hr = rng.range(58.0, 78.0);
        Self {
            rng,
            rest_hr,
            hr: rest_hr,
            activity: Activity::Rest,
            activity_left_s: 30,
            high_hr,
        }
    }

    /// Advances the wearer by one second.
    ///
    /// # Arguments
    /// * `t_s` - seconds since the simulation started.
    pub fn step(&mut self, t_s: u32) -> Sample {
        if self.activity_left_s == 0 {
            self.pick_next_activity();
        }
        self.activity_left_s -= 1;

        let episode = self
            .high_hr
            .is_some_and(|e| t_s >= e.at_s && t_s < e.at_s.saturating_add(e.for_s));
        let target = if episode {
            HIGH_HR_TARGET_BPM
        } else {
            self.rest_hr
                + match self.activity {
                    Activity::Walk => 25.0,
                    Activity::Run => 60.0,
                    Activity::Rest | Activity::Unknown => 0.0,
                }
        };
        // Heart rate drifts toward its target over tens of seconds, with jitter.
        self.hr += (target - self.hr) * 0.15 + self.rng.range(-2.0, 2.0);
        self.hr = self.hr.clamp(35.0, 210.0);

        let (quality_mean, quality_spread) = match self.activity {
            Activity::Rest => (92.0, 4.0),
            Activity::Walk => (80.0, 8.0),
            Activity::Run => (60.0, 14.0),
            Activity::Unknown => (70.0, 10.0),
        };
        let quality =
            (quality_mean + self.rng.range(-quality_spread, quality_spread)).clamp(0.0, 100.0);

        let run_penalty = if self.activity == Activity::Run {
            1.0
        } else {
            0.0
        };
        let spo2 = (98.0 + self.rng.range(-1.2, 1.0) - run_penalty).clamp(90.0, 100.0);

        let busy = self.rng.unit() < 0.7;
        let steps = match self.activity {
            Activity::Walk => {
                if busy {
                    2
                } else {
                    1
                }
            }
            Activity::Run => {
                if busy {
                    3
                } else {
                    2
                }
            }
            Activity::Rest | Activity::Unknown => 0,
        };

        let readable = quality >= MIN_QUALITY_FOR_VALUES;
        Sample {
            hr_bpm: if readable { self.hr.round() as u16 } else { 0 },
            spo2_pct: if readable { spo2.round() as u8 } else { 0 },
            steps,
            activity: self.activity,
            quality: quality.round() as u8,
        }
    }

    fn pick_next_activity(&mut self) {
        let roll = self.rng.unit();
        let (activity, lo, hi) = if roll < 0.02 {
            (Activity::Unknown, 5.0, 15.0)
        } else if roll < 0.5 {
            (Activity::Rest, 60.0, 180.0)
        } else if roll < 0.85 {
            (Activity::Walk, 45.0, 120.0)
        } else {
            (Activity::Run, 20.0, 60.0)
        };
        self.activity = activity;
        self.activity_left_s = self.rng.range(lo, hi).round() as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(seed: u64, high_hr: Option<HighHrEpisode>, seconds: u32) -> Vec<Sample> {
        let mut body = Body::new(seed, high_hr);
        (0..seconds).map(|t| body.step(t)).collect()
    }

    #[test]
    fn the_same_seed_gives_the_same_wearer() {
        assert_eq!(run(42, None, 600), run(42, None, 600));
        assert_ne!(run(42, None, 600), run(43, None, 600));
    }

    #[test]
    fn an_hour_of_samples_stays_in_valid_ranges() {
        for s in run(7, None, 3600) {
            assert!(s.quality <= 100);
            assert!(s.hr_bpm == 0 || (20..=250).contains(&s.hr_bpm));
            assert!(s.spo2_pct == 0 || (50..=100).contains(&s.spo2_pct));
            assert_eq!(s.hr_bpm == 0, s.spo2_pct == 0, "both values drop together");
        }
    }

    #[test]
    fn all_activities_appear_over_an_hour() {
        let samples = run(7, None, 3600);
        for activity in [Activity::Rest, Activity::Walk, Activity::Run] {
            assert!(samples.iter().any(|s| s.activity == activity));
        }
    }

    #[test]
    fn a_high_episode_pushes_heart_rate_over_the_alert_threshold() {
        let episode = HighHrEpisode {
            at_s: 60,
            for_s: 120,
        };
        let samples = run(3, Some(episode), 180);
        let peak = samples[150..180]
            .iter()
            .map(|s| s.hr_bpm)
            .max()
            .unwrap_or(0);
        assert!(peak > 140, "peak was {peak} bpm");
    }
}
