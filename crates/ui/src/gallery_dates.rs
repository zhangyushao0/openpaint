//! The dates the gallery shows for kept versions: day, month, year and time of day in the
//! device's time zone ("5 Oct 2026 14:32"), from seconds since 1970 UTC and the host's offset.

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The labels of versions kept at `times` (seconds since 1970 UTC), `offset` minutes from
/// UTC: the date and minute, with the seconds where two of them share a minute.
pub(crate) fn kept_labels(times: &[u64], offset: i32) -> Vec<String> {
    let minutes: Vec<String> = times
        .iter()
        .map(|at| kept_label(*at, offset, false))
        .collect();
    minutes
        .iter()
        .zip(times)
        .map(|(label, at)| {
            let shared = minutes.iter().filter(|other| *other == label).count() > 1;
            if shared {
                kept_label(*at, offset, true)
            } else {
                label.clone()
            }
        })
        .collect()
}

/// `seconds` since 1970 UTC as the device's clock showed it, `offset` minutes from UTC, to the
/// minute or, with `exact`, the second.
fn kept_label(seconds: u64, offset: i32, exact: bool) -> String {
    let local = i64::try_from(seconds).unwrap_or(i64::MAX) + i64::from(offset) * 60;
    let (year, month, day) = civil(local.div_euclid(86_400));
    let second = local.rem_euclid(86_400);
    let (hour, minute) = (second / 3600, second / 60 % 60);
    let month = MONTHS[month - 1];
    if exact {
        let second = second % 60;
        format!("{day} {month} {year} {hour:02}:{minute:02}:{second:02}")
    } else {
        format!("{day} {month} {year} {hour:02}:{minute:02}")
    }
}

/// The date `days` after 1970-01-01 in the proleptic Gregorian calendar: year, month (1 to 12)
/// and day (Howard Hinnant's `civil_from_days`).
fn civil(days: i64) -> (i64, usize, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, usize::try_from(month).unwrap_or(1), day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kept_times_read_as_the_local_date_and_time() {
        let label = |at, offset| kept_label(at, offset, false);
        assert_eq!(label(0, 0), "1 Jan 1970 00:00");
        assert_eq!(label(1_791_201_600, 0), "5 Oct 2026 12:00");
        assert_eq!(label(1_791_201_600, 120), "5 Oct 2026 14:00");
        assert_eq!(label(1_791_201_600 + 59, 0), "5 Oct 2026 12:00");
        assert_eq!(label(1_791_201_600, -13 * 60), "4 Oct 2026 23:00");
        assert_eq!(label(1_709_164_800 + 3_660, 0), "29 Feb 2024 01:01");
        assert_eq!(label(1_735_689_599, 0), "31 Dec 2024 23:59");
        assert_eq!(label(1_735_689_600, 330), "1 Jan 2025 05:30");
    }

    #[test]
    fn versions_of_one_minute_show_their_seconds() {
        let times = [1_791_201_600 + 125, 1_791_201_600 + 61, 1_791_201_600 + 7];
        assert_eq!(
            kept_labels(&times, 0),
            ["5 Oct 2026 12:02", "5 Oct 2026 12:01", "5 Oct 2026 12:00"]
        );
        let times = [1_791_201_600 + 125, 1_791_201_600 + 64, 1_791_201_600 + 61];
        assert_eq!(
            kept_labels(&times, 0),
            [
                "5 Oct 2026 12:02",
                "5 Oct 2026 12:01:04",
                "5 Oct 2026 12:01:01"
            ]
        );
    }
}
