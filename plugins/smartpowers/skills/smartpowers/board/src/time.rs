//! Timestamp formatting. std has no local-timezone support, and the UI only
//! shows relative times, so everything is UTC.

use std::time::{SystemTime, UNIX_EPOCH};

struct Civil {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
}

fn civil(time: SystemTime) -> Civil {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    Civil {
        year: yoe + era * 400 + i64::from(month <= 2),
        month,
        day,
        hour: rem / 3_600,
        minute: rem % 3_600 / 60,
        second: rem % 60,
    }
}

/// RFC 3339, e.g. `2026-10-17T10:15:30Z`. Sorts chronologically as text.
pub fn iso_utc(time: SystemTime) -> String {
    let c = civil(time);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        c.year, c.month, c.day, c.hour, c.minute, c.second
    )
}

/// RFC 3339 with milliseconds, e.g. `2026-10-17T10:15:30.123Z`, for
/// comments and log entries that must keep their order within a second.
pub fn iso_utc_millis(time: SystemTime) -> String {
    let c = civil(time);
    let millis = time
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_millis());
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        c.year, c.month, c.day, c.hour, c.minute, c.second
    )
}

/// Compact form for identifiers, e.g. `20261017-101530`.
pub fn compact_utc(time: SystemTime) -> String {
    let c = civil(time);
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        c.year, c.month, c.day, c.hour, c.minute, c.second
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn formats_utc_timestamps() {
        assert_eq!(iso_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        let sample = UNIX_EPOCH + Duration::from_secs(1_788_000_000);
        assert_eq!(iso_utc(sample), "2026-08-29T10:40:00Z");
        assert_eq!(compact_utc(sample), "20260829-104000");
        assert_eq!(
            iso_utc_millis(sample + Duration::from_millis(7)),
            "2026-08-29T10:40:00.007Z"
        );
        // Leap day.
        assert_eq!(
            iso_utc(UNIX_EPOCH + Duration::from_secs(951_782_400)),
            "2000-02-29T00:00:00Z"
        );
    }
}
