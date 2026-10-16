//! Conversions between database timestamps and the API's epoch milliseconds.

use chrono::{DateTime, Utc};

/// Returns the current time as UTC epoch milliseconds.
#[must_use]
pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// Converts a database timestamp to UTC epoch milliseconds.
#[must_use]
pub fn to_ms(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

/// Converts UTC epoch milliseconds to a timestamp.
///
/// # Returns
/// `None` when the value is outside the range chrono can represent.
#[must_use]
pub fn from_ms(ms: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_millis(ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milliseconds_round_trip() {
        let ms = 1_700_000_000_123;
        assert_eq!(from_ms(ms).map(to_ms), Some(ms));
    }

    #[test]
    fn out_of_range_values_are_rejected() {
        assert_eq!(from_ms(i64::MAX), None);
    }
}
