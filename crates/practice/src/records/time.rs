//! Practice time (S89): only the time a hand practises counts. A record keeps the spans of its
//! sheet's activity: a span runs from one use of the pen (touching or hovering), a finger or a
//! control to the next while the app is in front, as long as no gap between two uses passes
//! `PAUSE`; a longer gap counts nothing, not even `PAUSE` of it (the author leaves the app open an
//! hour or four while doing something else), and neither does time with the app hidden. So a
//! span's time is its length. Spans are kept in seconds since 1970 UTC; the book counts them by
//! the device's local day, split at its midnights, so a copy painted over two days shows on both.
//! A record from before S89 gets its spans from the times its session began, its exercises were
//! first compared and it finished, by the same rule (the store's upgrade).
//!
//! The book lays its days by number (`local_day`); the screen names them as dates.

/// The longest gap between two uses that still counts, in seconds.
pub const PAUSE: u64 = 300;

/// Seconds in a day.
const DAY: i64 = 86_400;

/// A stretch of practice: from one use to the last before a longer gap, in seconds since 1970 UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub from: u64,
    pub to: u64,
}

impl Span {
    /// Its time, in seconds.
    #[must_use]
    pub const fn seconds(self) -> u64 {
        self.to.saturating_sub(self.from)
    }
}

/// A use at `now`: the last span runs on to it when it ended at most `PAUSE` before and the app
/// was not `hidden` since; otherwise a new span starts at `now`. Returns whether one started.
pub fn practised(spans: &mut Vec<Span>, now: u64, hidden: bool) -> bool {
    if let Some(last) = spans.last_mut()
        && !hidden
        && now >= last.to
        && now - last.to <= PAUSE
    {
        last.to = now;
        return false;
    }
    spans.push(Span { from: now, to: now });
    true
}

/// The spans of uses at `times` (in any order) by the same rule, the app never hidden.
#[must_use]
pub fn spans_of(times: &[u64]) -> Vec<Span> {
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    let mut spans = Vec::new();
    for at in sorted {
        practised(&mut spans, at, false);
    }
    spans
}

/// The time `spans` hold, in seconds.
#[must_use]
pub fn seconds(spans: &[Span]) -> u64 {
    spans.iter().map(|span| span.seconds()).sum()
}

fn signed(at: u64) -> i64 {
    i64::try_from(at).unwrap_or(i64::MAX)
}

/// The local day of `at` (seconds since 1970 UTC) on a clock `offset` minutes from UTC: days
/// since 1970-01-01 there.
#[must_use]
pub fn local_day(at: u64, offset: i32) -> i64 {
    (signed(at) + i64::from(offset) * 60).div_euclid(DAY)
}

/// The seconds `spans` practised on each local day (`offset` minutes from UTC), oldest first: a
/// span over midnight counts on both days.
#[must_use]
pub fn by_day(spans: &[Span], offset: i32) -> Vec<(i64, u64)> {
    let shift = i64::from(offset) * 60;
    let mut days: Vec<(i64, u64)> = Vec::new();
    for span in spans {
        let (mut from, to) = (signed(span.from) + shift, signed(span.to) + shift);
        while from < to {
            let day = from.div_euclid(DAY);
            let end = ((day + 1) * DAY).min(to);
            let seconds = u64::try_from(end - from).unwrap_or(0);
            match days.iter_mut().find(|(kept, _)| *kept == day) {
                Some((_, kept)) => *kept += seconds,
                None => days.push((day, seconds)),
            }
            from = end;
        }
    }
    days.sort_by_key(|(day, _)| *day);
    days
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-05 12:00 UTC.
    const NOON: u64 = 1_791_201_600;

    #[test]
    fn a_gap_up_to_five_minutes_counts_and_a_longer_one_counts_nothing() {
        let mut spans = Vec::new();
        assert!(practised(&mut spans, NOON, false));
        assert!(!practised(&mut spans, NOON + 60, false));
        assert!(
            !practised(&mut spans, NOON + 60 + PAUSE, false),
            "exactly a pause"
        );
        assert!(practised(&mut spans, NOON + 60 + 2 * PAUSE + 1, false));
        assert_eq!(seconds(&spans), 60 + PAUSE);
        // Two hours untouched: the next use starts again, the two hours count nothing.
        let later = NOON + 2 * 3600;
        assert!(practised(&mut spans, later, false));
        assert!(!practised(&mut spans, later + 90, false));
        assert_eq!(seconds(&spans), 60 + PAUSE + 90);
    }

    #[test]
    fn time_hidden_counts_nothing() {
        let mut spans = Vec::new();
        practised(&mut spans, NOON, false);
        practised(&mut spans, NOON + 100, false);
        // Hidden for a minute: the minute is not practice.
        assert!(practised(&mut spans, NOON + 160, true));
        practised(&mut spans, NOON + 200, false);
        assert_eq!(seconds(&spans), 100 + 40);
        assert_eq!(spans.len(), 2);
    }

    #[test]
    fn spans_from_times_follow_the_same_rule() {
        let times = [
            NOON + 400,
            NOON,
            NOON + 200,
            NOON + 3 * 3600,
            NOON + 3 * 3600 + 30,
        ];
        assert_eq!(
            spans_of(&times),
            [
                Span {
                    from: NOON,
                    to: NOON + 400
                },
                Span {
                    from: NOON + 3 * 3600,
                    to: NOON + 3 * 3600 + 30
                }
            ]
        );
        assert!(spans_of(&[]).is_empty());
    }

    #[test]
    fn a_span_over_midnight_counts_on_both_days() {
        // 23:50 to 00:20 local at UTC+8.
        let offset = 8 * 60;
        let from = NOON + 3600 * 3 + 50 * 60;
        let spans = [Span {
            from,
            to: from + 30 * 60,
        }];
        let day = local_day(from, offset);
        assert_eq!(by_day(&spans, offset), [(day, 600), (day + 1, 1200)]);
        // In UTC it is one afternoon.
        assert_eq!(by_day(&spans, 0), [(local_day(from, 0), 1800)]);
        assert_eq!(local_day(NOON, 0), 20_731);
        assert_eq!(local_day(NOON, -13 * 60), 20_730);
    }
}
