//! What the debug build answers to `device-input state` (`DebugControls.state`, the fields
//! separated by tabs, each "key value"), asking it until the app gets somewhere, and how the
//! smoke and `open-file` tell a test artwork they made from the author's: by the artwork ids the
//! gallery lists, the one changed last first.

use std::time::{Duration, Instant};

use super::adb::{Adb, DeviceError, Result};
use super::input::{answer, extras, instrument};

/// The names of a card's picture and bin (`components/artwork_card.rs`).
pub(super) const OPEN_CARD: &str = "Open artwork";
pub(super) const DELETE_CARD: &str = "Delete artwork";
/// How long a tap waits for its control to show, in milliseconds.
const WITHIN: &str = "3000";

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

impl State {
    pub(super) fn parse(answer: &str) -> std::result::Result<Self, String> {
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

    /// The gallery shows its list.
    pub(super) fn listed(&self) -> bool {
        self.screen == Screen::Gallery && self.listed
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
    pub(super) fn every_card_shown(&self) -> std::result::Result<(), String> {
        let cards = self.cards.len();
        let (open, delete) = (self.count(OPEN_CARD), self.count(DELETE_CARD));
        if !self.listed() {
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

fn numbers(text: &str) -> std::result::Result<Vec<u64>, String> {
    text.split_whitespace()
        .map(|n| n.parse().map_err(|_| format!("not a number: {n}")))
        .collect()
}

/// A finger tap on the control `name`, waiting up to `WITHIN` for it to show.
pub(super) fn tap(adb: &Adb, name: &str) -> Result<()> {
    let mut words = extras(&["tap", name])?;
    words.push(("within", WITHIN.to_owned()));
    instrument(adb, &words).map(drop)
}

/// What the app shows now.
pub(super) fn state(adb: &Adb) -> Result<State> {
    let output = instrument(adb, &extras(&["state"])?)?;
    let state = answer(&output, "state").ok_or_else(|| unanswered("the app gave no state"))?;
    State::parse(state).map_err(unanswered)
}

/// Asks what the app shows until `done` holds, at most `limit`.
pub(super) fn until(adb: &Adb, limit: Duration, done: impl Fn(&State) -> bool) -> Result<State> {
    let started = Instant::now();
    loop {
        let state = state(adb)?;
        if done(&state) {
            return Ok(state);
        }
        if started.elapsed() >= limit {
            return Err(unanswered(format!(
                "waited {} s; the app shows {:?}, cards {:?}, controls {:?}",
                limit.as_secs(),
                state.screen,
                state.cards,
                state.names
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Why asking the app what it shows failed.
fn unanswered(why: impl Into<String>) -> DeviceError {
    DeviceError::Failed {
        command: "device-input state".into(),
        output: why.into(),
    }
}

/// The test artwork in `after`, the gallery once the canvas made after `before` was left: the
/// first card (changed last), an id `before` did not hold, every card of `before` still there
/// and nothing else new, every card shown. Otherwise why it cannot be told.
pub(super) fn identify(before: &[u64], after: &State) -> std::result::Result<u64, String> {
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
pub(super) fn first_is(ours: u64, state: &State) -> std::result::Result<(), String> {
    state.every_card_shown()?;
    match state.cards.first() {
        Some(&first) if first == ours => Ok(()),
        first => Err(format!(
            "the first card is {first:?}, not the test artwork {ours}: refusing to delete"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
