use chrono::{DateTime, Utc};

/// Get current UTC timestamp
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// Convert timestamp in milliseconds to DateTime<Utc>
pub fn from_millis(millis: i64) -> Option<DateTime<Utc>> {
    DateTime::from_timestamp_millis(millis)
}

/// Convert DateTime<Utc> to milliseconds timestamp
pub fn to_millis(dt: DateTime<Utc>) -> i64 {
    dt.timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_conversion() {
        let now = now();
        let millis = to_millis(now);
        let converted = from_millis(millis).unwrap();

        // Allow for small precision differences
        assert!((now.timestamp_millis() - converted.timestamp_millis()).abs() < 2);
    }
}
