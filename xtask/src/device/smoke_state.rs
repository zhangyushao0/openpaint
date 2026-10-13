//! What the device smoke reads and where it draws (S52): what the debug build answers to
//! `device-input state` (`DebugControls.state`, the fields separated by tabs, each "key value");
//! how the smoke tells the test artwork it made from the author's (by the artwork ids the gallery
//! lists, the one changed last first); its pen paths; the system's window in front; the shared
//! PNG; the driver's lines.

use std::time::{Duration, Instant};

use super::adb::{Adb, DeviceError, PACKAGE};

/// Pen points along a stroke at most this far apart, in screen pixels (one every 8 ms).
pub(super) const SPACING: f32 = 24.0;
/// How long a step waits for the app or the system to get somewhere.
pub(super) const WAIT: Duration = Duration::from_secs(8);

/// "dev.openpaint <versionName> (version code <n>)" as the package manager has it installed.
pub(super) fn installed(adb: &Adb) -> Result<String, DeviceError> {
    let dump = adb.shell(&["dumpsys", "package", PACKAGE])?;
    let field = |key: &str| {
        dump.lines()
            .find_map(|line| {
                line.split_whitespace()
                    .find_map(|word| word.strip_prefix(key))
            })
            .unwrap_or("?")
            .to_owned()
    };
    let (name, code) = (field("versionName="), field("versionCode="));
    Ok(format!("{PACKAGE} {name} (version code {code})"))
}

/// A smoke step's failure, saying why.
pub(super) fn failed(why: impl Into<String>) -> DeviceError {
    DeviceError::Failed {
        command: "device-smoke".into(),
        output: why.into(),
    }
}

/// Waits until the window with the input focus is one `done` accepts, at most `WAIT`.
pub(super) fn focus_until(adb: &Adb, done: impl Fn(&str) -> bool) -> Result<(), DeviceError> {
    let started = Instant::now();
    loop {
        let dump = adb.shell_within(&["dumpsys", "window"], WAIT)?;
        let focus = dump
            .lines()
            .find_map(|line| line.trim().strip_prefix("mCurrentFocus="))
            .unwrap_or("none");
        if done(focus) {
            return Ok(());
        }
        if started.elapsed() >= WAIT {
            return Err(failed(format!("the focus stayed on {focus}")));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The app's share cache (`cache/share`, only the newest share kept) holds one PNG: the canvas
/// at `fit` at 1x, 2x or 4x.
pub(super) fn shared(adb: &Adb, fit: [f32; 4]) -> Result<(), DeviceError> {
    let listed = adb.shell(&["run-as", PACKAGE, "ls", "cache/share"])?;
    let names: Vec<&str> = listed.lines().map(str::trim).collect();
    let [name] = names[..] else {
        return Err(failed(format!("the share cache holds {names:?}")));
    };
    if !name.ends_with(".png") || name.contains('\'') {
        return Err(failed(format!("the share cache holds {name}")));
    }
    // The file's first 24 bytes as decimal numbers: text, as every other answer.
    let path = format!("'cache/share/{name}'");
    let words = [
        "run-as", PACKAGE, "od", "-A", "n", "-t", "u1", "-N", "24", &path,
    ];
    let dump = adb.shell(&words)?;
    let head: Vec<u8> = dump
        .split_whitespace()
        .filter_map(|n| n.parse().ok())
        .collect();
    let canvas = [fit[2], fit[3]].map(|side| side.round() as u32);
    match png_size(&head) {
        Some(size) if [1, 2, 4].iter().any(|k| size == canvas.map(|s| s * k)) => Ok(()),
        size => Err(failed(format!(
            "{name}: {size:?}, the canvas {canvas:?}; od said {}",
            dump.trim()
        ))),
    }
}

/// What the screen shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Screen {
    /// No runtime, or the library still opening.
    Starting,
    Canvas,
    Gallery,
}

/// One answer.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct State {
    pub(super) screen: Screen,
    /// The gallery has its list from the library.
    pub(super) listed: bool,
    /// The gallery's artworks by id, the one changed last first.
    pub(super) cards: Vec<u64>,
    /// Where a canvas fits on the screen, in pixels: left, top, width, height.
    pub(super) fit: Option<[f32; 4]>,
    /// Every control's name in the last frame (`name #n` where several share one).
    pub(super) names: Vec<String>,
}

/// The names of a card's picture and bin (`components/artwork_card.rs`).
pub(super) const OPEN_CARD: &str = "Open artwork";
pub(super) const DELETE_CARD: &str = "Delete artwork";

impl State {
    pub(super) fn parse(answer: &str) -> Result<Self, String> {
        let mut state = Self {
            screen: Screen::Starting,
            listed: false,
            cards: Vec::new(),
            fit: None,
            names: Vec::new(),
        };
        for field in answer.split('\t') {
            let (key, value) = field.split_once(' ').unwrap_or((field, ""));
            match key {
                "screen" => {
                    state.screen = match value {
                        "canvas" => Screen::Canvas,
                        "gallery" => Screen::Gallery,
                        _ => Screen::Starting,
                    };
                }
                "listed" => state.listed = value == "1",
                "cards" => state.cards = numbers(value)?,
                "fit" => {
                    let fit = numbers(value)?;
                    let [x, y, w, h] = fit[..] else {
                        return Err(format!("fit {value}"));
                    };
                    state.fit = Some([x, y, w, h].map(|n| n as f32));
                }
                "name" => state.names.push(value.to_owned()),
                _ => return Err(format!("unknown field in the app's state: {field}")),
            }
        }
        Ok(state)
    }

    /// How many controls are called `name`, alone or as `name #n`.
    pub(super) fn count(&self, name: &str) -> usize {
        self.names
            .iter()
            .filter(|shown| {
                *shown == name
                    || shown
                        .strip_prefix(name)
                        .and_then(|rest| rest.strip_prefix(" #"))
                        .is_some_and(|n| n.parse::<usize>().is_ok())
            })
            .count()
    }

    /// Whether a control is called `name` (exactly, `name #n` included).
    pub(super) fn shows(&self, name: &str) -> bool {
        self.names.iter().any(|shown| shown == name)
    }

    /// The gallery shows every card once, in its order, so a card's `name #n` is `cards[n - 1]`
    /// (no search hides one, none lies off the screen); otherwise why not.
    pub(super) fn every_card_shown(&self) -> Result<(), String> {
        let cards = self.cards.len();
        let (open, delete) = (self.count(OPEN_CARD), self.count(DELETE_CARD));
        if self.screen != Screen::Gallery || !self.listed {
            return Err("the gallery is not listed".into());
        }
        if open != cards || delete != cards {
            return Err(format!(
                "the gallery lists {cards} artworks but shows {open} cards and {delete} bins \
                 (a search, or more than one screen holds)"
            ));
        }
        Ok(())
    }
}

fn numbers(text: &str) -> Result<Vec<u64>, String> {
    text.split_whitespace()
        .map(|n| n.parse().map_err(|_| format!("not a number: {n}")))
        .collect()
}

/// The test artwork in `after`, the gallery once the canvas made after `before` was left: the
/// first card (changed last), an id `before` did not hold, every card of `before` still there
/// and nothing else new, every card shown. Otherwise why it cannot be told.
pub(super) fn identify(before: &[u64], after: &State) -> Result<u64, String> {
    after.every_card_shown()?;
    let new: Vec<u64> = after
        .cards
        .iter()
        .copied()
        .filter(|id| !before.contains(id))
        .collect();
    let kept = before.iter().all(|id| after.cards.contains(id));
    match (new.as_slice(), after.cards.first()) {
        (&[id], Some(&first)) if kept && first == id => Ok(id),
        _ => Err(format!(
            "cannot tell the test artwork: before {before:?}, now {:?}",
            after.cards
        )),
    }
}

/// `ours` is the first card and every card shows, so `Delete artwork #1` deletes it and nothing
/// else; otherwise why not.
pub(super) fn first_is(ours: u64, state: &State) -> Result<(), String> {
    state.every_card_shown()?;
    match state.cards.first() {
        Some(&first) if first == ours => Ok(()),
        first => Err(format!(
            "the first card is {first:?}, not the test artwork {ours}: refusing to delete"
        )),
    }
}

/// `at`, a fraction of the canvas's `fit` (left, top, width, height), in screen pixels.
pub(super) fn point([left, top, width, height]: [f32; 4], [fx, fy]: [f32; 2]) -> [f32; 2] {
    [left + fx * width, top + fy * height]
}

/// The pen path through `waypoints` (fractions of `fit`), points at most `SPACING` apart, as
/// `device-input pen` takes it.
pub(super) fn path(fit: [f32; 4], waypoints: &[[f32; 2]]) -> String {
    let mut points = vec![point(fit, waypoints[0])];
    for pair in waypoints.windows(2) {
        let ([ax, ay], [bx, by]) = (point(fit, pair[0]), point(fit, pair[1]));
        let steps = ((bx - ax).hypot(by - ay) / SPACING).ceil().max(1.0) as usize;
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            points.push([ax + t * (bx - ax), ay + t * (by - ay)]);
        }
    }
    let words: Vec<String> = points
        .iter()
        .map(|[x, y]| format!("{},{}", x.round(), y.round()))
        .collect();
    words.join(",")
}

/// The Adreno driver's harmless lines at each start: a 4 x 4 buffer of a format it lacks
/// (`Gralloc4`, `GraphicBufferAllocator`, `AHardwareBuffer`; STATUS, device commands).
pub(super) fn gralloc(line: &str) -> bool {
    let tag = line.split_whitespace().nth(5).unwrap_or("");
    matches!(
        tag,
        "Gralloc4:" | "GraphicBufferAllocator:" | "AHardwareBuffer:"
    )
}

/// Width and height from a PNG's signature and header chunk.
pub(super) fn png_size(head: &[u8]) -> Option<[u32; 2]> {
    let signature = head.get(..8)? == b"\x89PNG\r\n\x1a\n" && head.get(12..16)? == b"IHDR";
    let side = |at: usize| Some(u32::from_be_bytes(head.get(at..at + 4)?.try_into().ok()?));
    signature.then_some([side(16)?, side(20)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_pngs_and_the_drivers_lines() {
        let fit = [0.0, 84.0, 2880.0, 1716.0];
        assert_eq!(point(fit, [0.5, 0.5]), [1440.0, 942.0]);
        assert_eq!(path(fit, &[[0.0, 0.0], [0.01, 0.0]]), "0,84,14,84,29,84");
        let numbers: Vec<f32> = path(fit, &[[0.25, 0.4], [0.33, 0.3], [0.41, 0.42]])
            .split(',')
            .map(|n| n.parse().expect("a number"))
            .collect();
        let points: Vec<&[f32]> = numbers.chunks(2).collect();
        let apart = |p: &[&[f32]]| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]);
        assert!(points.len() > 10 && points.windows(2).all(|p| apart(p) <= SPACING + 1.0));
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        png.extend(2880u32.to_be_bytes());
        png.extend(1716u32.to_be_bytes());
        assert_eq!(png_size(&png), Some([2880, 1716]));
        assert_eq!(png_size(&png[..20]), None);
        assert_eq!(png_size(b"GIF89a"), None);
        let line = "10-12 04:02:30.341  5698  5698 E Gralloc4: isSupported(1, 59, ...) failed";
        assert!(gralloc(line));
        assert!(!gralloc(
            "10-12 04:02:30.341  5698  5698 E openpaint: saving failed"
        ));
    }

    fn gallery(cards: &[u64], shown: usize) -> State {
        let ids: Vec<String> = cards.iter().map(u64::to_string).collect();
        let mut answer = format!("screen gallery\tlisted 1\tcards {}", ids.join(" "));
        answer.push_str("\tfit 0 84 2880 1716\tname New artwork");
        for n in 1..=shown {
            answer.push_str(&format!(
                "\tname Open artwork #{n}\tname Delete artwork #{n}"
            ));
        }
        State::parse(&answer).expect("an answer")
    }

    #[test]
    fn answers_parse() {
        let state = gallery(&[9, 4, 2], 3);
        assert_eq!(state.screen, Screen::Gallery);
        assert!(state.listed && state.shows("New artwork") && !state.shows("New"));
        assert_eq!(state.cards, [9, 4, 2]);
        assert_eq!(state.fit, Some([0.0, 84.0, 2880.0, 1716.0]));
        assert_eq!((state.count(OPEN_CARD), state.count("Open")), (3, 0));
        let one =
            State::parse("screen gallery\tlisted 1\tcards 5\tname Open artwork").expect("one card");
        assert_eq!(one.count(OPEN_CARD), 1);
        let canvas = State::parse("screen canvas\tname Gallery\tname Hue, Saturation, Brightness")
            .expect("canvas");
        assert_eq!(canvas.screen, Screen::Canvas);
        assert!(canvas.shows("Hue, Saturation, Brightness"));
        assert_eq!(
            State::parse("screen none").map(|s| s.screen),
            Ok(Screen::Starting)
        );
        assert!(State::parse("cards 1 x").is_err() && State::parse("colour red").is_err());
        assert!(State::parse("fit 1 2 3").is_err());
    }

    #[test]
    fn only_the_one_new_first_card_is_ours() {
        let before = [7, 5, 3];
        assert_eq!(identify(&before, &gallery(&[8, 7, 5, 3], 4)), Ok(8));
        // Not first, two new, one gone, a card hidden by a search or off the screen.
        assert!(identify(&before, &gallery(&[7, 8, 5, 3], 4)).is_err());
        assert!(identify(&before, &gallery(&[9, 8, 7, 5, 3], 5)).is_err());
        assert!(identify(&before, &gallery(&[8, 7, 5], 3)).is_err());
        assert!(identify(&before, &gallery(&[8, 7, 5, 3], 3)).is_err());
        assert!(identify(&before, &gallery(&[7, 5, 3], 3)).is_err());
        let mut starting = gallery(&[8, 7, 5, 3], 4);
        starting.listed = false;
        assert!(identify(&before, &starting).is_err());
        assert_eq!(first_is(8, &gallery(&[8, 7, 5, 3], 4)), Ok(()));
        assert!(first_is(8, &gallery(&[7, 8, 5, 3], 4)).is_err());
        assert!(first_is(8, &gallery(&[8, 7, 5, 3], 2)).is_err());
        assert!(first_is(8, &gallery(&[], 0)).is_err());
    }
}
