//! `cargo device-frames [since] [--lines]` and `cargo device-frames --clear`: what the debug
//! build's recorder of frames in real use kept (`crates/runtime/src/recorder/`: a line a stretch of
//! motion, at most 5,000, in the app's `files/frames.txt`), read out of the app's private storage
//! as `device-backup` reads the library (`run-as`), nothing on the tablet changed, and summed up:
//! the stretches by the hand, by the screen and by the display's rate during them (as
//! Choreographer delivered it), how many, the mean percent of the vsyncs a hand or a motion wanted
//! a frame at that got one drawn (frame calls at rest are left out: since E0, S127, so a line
//! from before reads lower), the share under 90 % and under 70 %, and the median of their frame
//! work p50 and p90. `since` (`YYYY-MM-DD[THH[:MM[:SS]]]`, the tablet's local time) keeps the
//! stretches from then on; `--lines` prints them too. `--clear` empties the file (truncated in
//! place: the app opens it for each line).

use super::adb::{Adb, DeviceError, PACKAGE, Result};

/// The recorder's file, relative to the app's data directory (`FRAMES_FILE` in the runtime).
const FILE: &str = "files/frames.txt";
const USAGE: &str =
    "cargo device-frames [since] [--lines] | --clear, since YYYY-MM-DD[THH[:MM[:SS]]]";
/// What `since` may be: a prefix of this, each '0' a digit.
const SINCE: &str = "0000-00-00T00:00:00";

/// `cargo device-frames`.
pub fn frames(args: &[String]) -> Result<()> {
    let clear = args.iter().any(|arg| arg == "--clear");
    let listing = args.iter().any(|arg| arg == "--lines");
    let rest: Vec<&String> = args.iter().filter(|arg| !arg.starts_with("--")).collect();
    let since = match rest[..] {
        [] => None,
        [since] if !clear && valid_since(since) => Some(since.as_str()),
        _ => return Err(DeviceError::Usage(USAGE)),
    };
    if clear && (listing || args.len() > 1) {
        return Err(DeviceError::Usage(USAGE));
    }
    let adb = Adb::connected()?;
    let files = adb.shell(&["run-as", PACKAGE, "ls", "files"])?;
    if !files.split_whitespace().any(|name| name == "frames.txt") {
        println!("frames: no {FILE} on the tablet (the debug build records once it runs)");
        return Ok(());
    }
    if clear {
        adb.shell(&["run-as", PACKAGE, "truncate", "-s", "0", FILE])?;
        println!("frames: {FILE} emptied");
        return Ok(());
    }
    let text = adb.text(&["exec-out", "run-as", PACKAGE, "cat", FILE])?;
    let (stretches, unread) = read(&text, since);
    if listing {
        for stretch in &stretches {
            println!("{}", stretch.line);
        }
    }
    for line in summary(&stretches, since) {
        println!("{line}");
    }
    if unread > 0 {
        println!("frames: {unread} lines not understood, left out");
    }
    Ok(())
}

/// Whether `since` is `YYYY-MM-DD`, with `THH`, `:MM` and `:SS` after it or not.
fn valid_since(since: &str) -> bool {
    [10, 13, 16, 19].contains(&since.len())
        && (since.bytes().zip(SINCE.bytes())).all(|(byte, shape)| {
            if shape == b'0' {
                byte.is_ascii_digit()
            } else {
                byte == shape
            }
        })
}

/// One recorded stretch.
#[derive(Debug, Clone, PartialEq)]
struct Stretch<'a> {
    line: &'a str,
    when: &'a str,
    drawn: u32,
    wanted: u32,
    /// Its frame work p50 and p90, ms; none when no frame was drawn.
    work: Option<[f64; 2]>,
    /// The display's rate as Choreographer delivered it ("120Hz"), "-" unknown.
    display: &'a str,
    hand: &'a str,
    screen: &'a str,
}

impl Stretch<'_> {
    fn percent(&self) -> f64 {
        100.0 * f64::from(self.drawn) / f64::from(self.wanted.max(1))
    }
}

/// The stretches of `text` from `since` on, and how many lines were not understood.
fn read<'a>(text: &'a str, since: Option<&str>) -> (Vec<Stretch<'a>>, usize) {
    let (mut stretches, mut unread) = (Vec::new(), 0);
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match parse(line) {
            Some(stretch) if since.is_none_or(|since| stretch.when >= since) => {
                stretches.push(stretch);
            }
            Some(_) => {}
            None => unread += 1,
        }
    }
    (stretches, unread)
}

/// "<when> <seconds>s drawn <drawn>/<wanted> <percent>% work <p50/p90/max> ui <p50/p90/max>
/// display <hz>Hz hand <hand> screen <screen>", as the runtime's recorder writes it (a line from
/// before 1.23.2 has no display: "-").
fn parse(line: &str) -> Option<Stretch<'_>> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let (when, counts, work, display, hand, screen) = match words[..] {
        [
            when,
            _,
            "drawn",
            counts,
            _,
            "work",
            work,
            "ui",
            _,
            "display",
            display,
            "hand",
            hand,
            "screen",
            screen,
        ] => (when, counts, work, display, hand, screen),
        [
            when,
            _,
            "drawn",
            counts,
            _,
            "work",
            work,
            "ui",
            _,
            "hand",
            hand,
            "screen",
            screen,
        ] => (when, counts, work, "-", hand, screen),
        _ => return None,
    };
    let (drawn, wanted) = counts.split_once('/')?;
    let spread: Vec<f64> = work.split('/').filter_map(|ms| ms.parse().ok()).collect();
    let work = match spread[..] {
        [p50, p90, _] => Some([p50, p90]),
        _ if work == "-" => None,
        _ => return None,
    };
    Some(Stretch {
        line,
        when,
        drawn: drawn.parse().ok()?,
        wanted: wanted.parse().ok()?,
        work,
        display,
        hand,
        screen,
    })
}

/// A group of stretches summed up.
#[derive(Debug, PartialEq)]
struct Group {
    count: usize,
    mean: f64,
    under_90: f64,
    under_70: f64,
    /// The median of their frame work p50 and of their p90, ms.
    work: Option<[f64; 2]>,
}

impl Group {
    fn of(stretches: &[&Stretch<'_>]) -> Self {
        let count = stretches.len();
        let percents: Vec<f64> = stretches.iter().map(|s| s.percent()).collect();
        let share = |below: f64| {
            let under = percents.iter().filter(|percent| **percent < below).count();
            100.0 * under as f64 / count.max(1) as f64
        };
        let works: Vec<[f64; 2]> = stretches.iter().filter_map(|s| s.work).collect();
        let median_of = |at: usize| median(works.iter().map(|work| work[at]).collect());
        Self {
            count,
            mean: percents.iter().sum::<f64>() / count.max(1) as f64,
            under_90: share(90.0),
            under_70: share(70.0),
            work: median_of(0).zip(median_of(1)).map(|(p50, p90)| [p50, p90]),
        }
    }

    fn row(&self, label: &str) -> String {
        let [p50, p90] = self.work.map_or(["-".to_owned(), "-".to_owned()], |w| {
            w.map(|ms| format!("{ms:.1}"))
        });
        format!(
            "  {label:<15}{:>9}{:>8.1}{:>9.0}{:>9.0}{p50:>10}{p90:>10}",
            self.count, self.mean, self.under_90, self.under_70
        )
    }
}

/// The middle value (the mean of the two middle ones for an even count); none of nothing.
fn median(mut values: Vec<f64>) -> Option<f64> {
    values.sort_by(f64::total_cmp);
    let half = values.len() / 2;
    match values.len() {
        0 => None,
        n if n % 2 == 1 => Some(values[half]),
        _ => Some(f64::midpoint(values[half - 1], values[half])),
    }
}

/// The summary's lines: how many stretches over what time, then a row for all, each hand and each
/// screen that has any.
fn summary(stretches: &[Stretch<'_>], since: Option<&str>) -> Vec<String> {
    let from = since.map_or_else(String::new, |since| format!(" since {since}"));
    let (Some(first), Some(last)) = (stretches.first(), stretches.last()) else {
        return vec![format!("frames: no stretch recorded{from}")];
    };
    let mut lines = vec![
        format!(
            "frames: {} stretches{from}, {} to {}",
            stretches.len(),
            first.when,
            last.when
        ),
        "                 stretches  mean %  under 90  under 70  work p50  work p90".to_owned(),
    ];
    let all: Vec<&Stretch<'_>> = stretches.iter().collect();
    lines.push(Group::of(&all).row("all"));
    let hands = ["pen", "finger", "none"].map(|name| ("hand", name));
    let screens = ["canvas", "sheet", "page", "home"].map(|name| ("screen", name));
    for (by, name) in hands.into_iter().chain(screens) {
        let of = |s: &Stretch<'_>| if by == "hand" { s.hand } else { s.screen } == name;
        let group: Vec<&Stretch<'_>> = stretches.iter().filter(|s| of(s)).collect();
        if !group.is_empty() {
            lines.push(Group::of(&group).row(&format!("{by} {name}")));
        }
    }
    // By the display's rate during them: a stretch shown at 60 Hz reads as such.
    let mut rates: Vec<&str> = stretches.iter().map(|s| s.display).collect();
    rates.sort_unstable();
    rates.dedup();
    for rate in rates {
        let group: Vec<&Stretch<'_>> = stretches.iter().filter(|s| s.display == rate).collect();
        lines.push(Group::of(&group).row(&format!("display {rate}")));
    }
    lines.push(
        "  (mean %: of the vsyncs a hand or a motion wanted a frame at, those that got one, calls \
         at rest left out; under 90 and 70: the share of stretches below, %; work: the medians \
         of the stretches' own frame work, ms)"
            .to_owned(),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
2026-11-14T10:00:00 1.00s drawn 120/120 100% work 2.0/3.0/5.0 ui 1.0/1.5/2.0 display 120Hz hand pen \
screen canvas
2026-11-14T10:05:00 0.50s drawn 51/60 85% work 4.0/6.0/9.0 ui 1.0/1.5/2.0 hand pen screen canvas
not a line
2026-11-14T10:10:00 0.50s drawn 36/60 60% work 8.0/12.0/20.0 ui 2.0/3.0/4.0 display 60Hz hand none \
screen home
2026-11-14T10:20:00 0.25s drawn 0/30 0% work - ui - hand finger screen sheet
";

    #[test]
    fn the_summary_sums_the_stretches_up_by_hand_and_by_screen() {
        let (stretches, unread) = read(FIXTURE, None);
        assert_eq!((stretches.len(), unread), (4, 1));
        let all: Vec<&Stretch<'_>> = stretches.iter().collect();
        let group = Group::of(&all);
        assert_eq!(
            group,
            Group {
                count: 4,
                mean: 61.25,
                under_90: 75.0,
                under_70: 50.0,
                work: Some([4.0, 6.0]),
            }
        );
        let pen: Vec<&Stretch<'_>> = stretches.iter().filter(|s| s.hand == "pen").collect();
        let pen = Group::of(&pen);
        assert_eq!(
            (pen.count, pen.mean, pen.under_90, pen.under_70),
            (2, 92.5, 50.0, 0.0)
        );
        assert_eq!(
            pen.work,
            Some([3.0, 4.5]),
            "the mean of the two middle ones"
        );
        let lines = summary(&stretches, None);
        assert_eq!(
            lines[0],
            "frames: 4 stretches, 2026-11-14T10:00:00 to 2026-11-14T10:20:00"
        );
        assert_eq!(
            lines[2],
            "  all                    4    61.2       75       50       4.0       6.0"
        );
        let rows: Vec<&str> = lines[3..lines.len() - 1]
            .iter()
            .map(|row| row.split_whitespace().nth(1).unwrap_or(""))
            .collect();
        assert_eq!(
            rows,
            [
                "pen", "finger", "none", "canvas", "sheet", "home", "-", "120Hz", "60Hz"
            ]
        );
        assert!(
            lines
                .iter()
                .any(|row| row.starts_with("  hand finger") && row.ends_with("-         -"))
        );
    }

    #[test]
    fn since_keeps_the_stretches_from_then_and_must_be_a_time() {
        let (stretches, _) = read(FIXTURE, Some("2026-11-14T10:05"));
        let whens: Vec<&str> = stretches.iter().map(|s| s.when).collect();
        assert_eq!(
            whens,
            [
                "2026-11-14T10:05:00",
                "2026-11-14T10:10:00",
                "2026-11-14T10:20:00"
            ]
        );
        for since in [
            "2026-11-14",
            "2026-11-14T10",
            "2026-11-14T10:05",
            "2026-11-14T10:05:59",
        ] {
            assert!(valid_since(since), "{since}");
        }
        for since in [
            "",
            "2026-11",
            "today",
            "2026-11-14 10:05",
            "2026-11-14T1",
            "2026/11/14",
        ] {
            assert!(!valid_since(since), "{since}");
        }
        assert_eq!(
            summary(&[], Some("2027-01-01")),
            ["frames: no stretch recorded since 2027-01-01"]
        );
    }
}
