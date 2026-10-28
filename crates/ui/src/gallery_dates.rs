//! The dates the gallery shows for kept versions: day, month, year and time of day in the
//! device's time zone ("5 Oct 2026 14:32", "2026年10月5日 14:32"), from seconds since 1970 UTC
//! and the host's offset.

use openpaint_app::Language;
use openpaint_practice::Date;

use crate::text::kept_date;

/// The labels of versions kept at `times` (seconds since 1970 UTC), `offset` minutes from
/// UTC: the date and minute, with the seconds where two of them share a minute; each as its
/// row's name (in English) and in `language`.
pub(crate) fn kept_labels(times: &[u64], offset: i32, language: Language) -> Vec<(String, String)> {
    let minutes: Vec<String> = times
        .iter()
        .map(|at| kept_label(*at, offset, (Language::English, false)))
        .collect();
    minutes
        .iter()
        .zip(times)
        .map(|(label, at)| {
            let exact = minutes.iter().filter(|other| *other == label).count() > 1;
            let name = kept_label(*at, offset, (Language::English, exact));
            (name, kept_label(*at, offset, (language, exact)))
        })
        .collect()
}

/// `seconds` since 1970 UTC as the device's clock showed it, `offset` minutes from UTC, in
/// `language`, to the minute or, with `exact`, the second.
fn kept_label(seconds: u64, offset: i32, (language, exact): (Language, bool)) -> String {
    let local = i64::try_from(seconds).unwrap_or(i64::MAX) + i64::from(offset) * 60;
    let date = Date::of(local.div_euclid(86_400));
    let second = local.rem_euclid(86_400);
    let time = (
        second / 3600,
        second / 60 % 60,
        exact.then_some(second % 60),
    );
    kept_date(language, date, time)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kept_times_read_as_the_local_date_and_time() {
        let label = |at, offset| kept_label(at, offset, (Language::English, false));
        assert_eq!(label(0, 0), "1 Jan 1970 00:00");
        assert_eq!(label(1_791_201_600, 0), "5 Oct 2026 12:00");
        assert_eq!(label(1_791_201_600, 120), "5 Oct 2026 14:00");
        assert_eq!(label(1_791_201_600 + 59, 0), "5 Oct 2026 12:00");
        assert_eq!(label(1_791_201_600, -13 * 60), "4 Oct 2026 23:00");
        assert_eq!(label(1_709_164_800 + 3_660, 0), "29 Feb 2024 01:01");
        assert_eq!(label(1_735_689_599, 0), "31 Dec 2024 23:59");
        assert_eq!(label(1_735_689_600, 330), "1 Jan 2025 05:30");
        let chinese = kept_label(1_791_201_600 + 7, 120, (Language::Chinese, true));
        assert_eq!(chinese, "2026年10月5日 14:00:07");
    }

    #[test]
    fn versions_of_one_minute_show_their_seconds() {
        let shown = |times: &[u64], language| -> Vec<String> {
            kept_labels(times, 0, language)
                .into_iter()
                .map(|(_, label)| label)
                .collect()
        };
        let times = [1_791_201_600 + 125, 1_791_201_600 + 61, 1_791_201_600 + 7];
        assert_eq!(
            shown(&times, Language::English),
            ["5 Oct 2026 12:02", "5 Oct 2026 12:01", "5 Oct 2026 12:00"]
        );
        let times = [1_791_201_600 + 125, 1_791_201_600 + 64, 1_791_201_600 + 61];
        assert_eq!(
            shown(&times, Language::English),
            [
                "5 Oct 2026 12:02",
                "5 Oct 2026 12:01:04",
                "5 Oct 2026 12:01:01"
            ]
        );
        let labels = kept_labels(&times, 0, Language::Chinese);
        assert_eq!(labels[1].0, "5 Oct 2026 12:01:04", "named in English");
        assert_eq!(labels[1].1, "2026年10月5日 12:01:04");
    }
}
