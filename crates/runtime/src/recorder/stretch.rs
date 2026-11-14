//! One stretch of motion as the recorder measures it, the hand and the screen of each frame call,
//! and the line a kept stretch ends as.

use openpaint_ui::dates::Date;

use crate::bench::{VSYNC, rank};
use crate::input::{MotionBatch, MotionPhase, ToolKind};

/// Drawn frames' samples a stretch keeps for its spreads (136 s at 120 Hz); a longer stretch's
/// later frames are counted, not sampled. Allocated once, with the recorder.
const SAMPLES: usize = 16_384;

/// What the hand did at a frame call: the pen down, a finger down, or nothing (an animation after
/// a tap, a worker's result). Kinds in the order a tie goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HandKind {
    Pen,
    Finger,
    Nothing,
}

const HANDS: [&str; 3] = ["pen", "finger", "none"];

/// Which screen a frame call drew: a painting canvas, a practice sheet, a page of practice's over
/// the home (the path, the practice book), or the home (a canvas about to open on it included).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ScreenKind {
    Canvas,
    Sheet,
    Page,
    Home,
}

const SCREENS: [&str; 4] = ["canvas", "sheet", "page", "home"];

/// The pen and the fingers on the glass now, and whether each touched since the last frame call.
#[derive(Debug, Default)]
pub(super) struct Hands {
    on: [bool; 2],
    since: [bool; 2],
}

impl Hands {
    /// An input batch the app took.
    pub(super) fn saw(&mut self, batch: &MotionBatch) {
        let stays = match batch.phase {
            MotionPhase::Down
            | MotionPhase::Move
            | MotionPhase::PointerDown { .. }
            | MotionPhase::PointerUp { .. }
            | MotionPhase::PointerCancel { .. } => true,
            MotionPhase::Up | MotionPhase::Cancel => false,
            // Hovering and buttons put nothing on the glass.
            _ => return,
        };
        let tool = |kinds: &[ToolKind]| batch.pointers.iter().any(|p| kinds.contains(&p.tool));
        let present = [
            tool(&[ToolKind::Stylus, ToolKind::Eraser]),
            tool(&[ToolKind::Finger]),
        ];
        for (index, here) in present.into_iter().enumerate() {
            if here {
                self.since[index] = true;
                self.on[index] = stays;
            }
        }
    }

    /// The hand at a frame call now: what is on the glass or touched since the call before.
    pub(super) fn take(&mut self) -> HandKind {
        let [pen, finger] = [0, 1].map(|index| self.on[index] || self.since[index]);
        self.since = self.on;
        match (pen, finger) {
            (true, _) => HandKind::Pen,
            (false, true) => HandKind::Finger,
            (false, false) => HandKind::Nothing,
        }
    }
}

/// The drawn frames' work and UI pass of the stretch open, ms; reused stretch after stretch.
#[derive(Debug)]
pub(super) struct Samples {
    work: Vec<f64>,
    ui: Vec<f64>,
}

impl Default for Samples {
    fn default() -> Self {
        Self {
            work: Vec::with_capacity(SAMPLES),
            ui: Vec::with_capacity(SAMPLES),
        }
    }
}

impl Samples {
    /// Emptied for the next stretch, its room kept.
    pub(super) fn clear(&mut self) {
        self.work.clear();
        self.ui.clear();
    }
}

/// What one frame call did, as a stretch counts it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Counted {
    /// Vsyncs that wanted a frame before this call's and found none drawn.
    pub(super) missed: u32,
    pub(super) drawn: bool,
    /// Its work and UI pass, ms (the bench's: `frame_timed`, `Runtime::ui_ns`).
    pub(super) work: f64,
    pub(super) ui: f64,
    pub(super) hand: HandKind,
    pub(super) screen: ScreenKind,
}

/// A run of consecutive vsyncs that wanted a frame, so far.
#[derive(Debug)]
pub(super) struct Stretch {
    /// The wall clock at its first frame call, seconds since 1970 UTC, and the device's offset
    /// from UTC then, minutes.
    began: (u64, i32),
    wanted: u32,
    drawn: u32,
    hands: [u32; 3],
    screens: [u32; 4],
    /// The shortest interval between two consecutive frame calls' vsyncs, ns: the display's.
    interval: Option<i64>,
}

impl Stretch {
    pub(super) const fn new(began: (u64, i32)) -> Self {
        Self {
            began,
            wanted: 0,
            drawn: 0,
            hands: [0; 3],
            screens: [0; 4],
            interval: None,
        }
    }

    /// A frame call's vsync and the ones it missed; its samples into `samples` while they have
    /// room (no allocation).
    pub(super) fn count(&mut self, call: Counted, samples: &mut Samples) {
        self.wanted += call.missed + 1;
        self.hands[call.hand as usize] += 1;
        self.screens[call.screen as usize] += 1;
        if call.drawn {
            self.drawn += 1;
            if samples.work.len() < SAMPLES {
                samples.work.push(call.work);
                samples.ui.push(call.ui);
            }
        }
    }

    /// The interval between two consecutive frame calls' vsyncs as Choreographer delivered them,
    /// ns.
    pub(super) fn delivered(&mut self, interval: i64) {
        if interval > 0 {
            self.interval = Some(self.interval.map_or(interval, |kept| kept.min(interval)));
        }
    }

    /// How many vsyncs it lasted.
    pub(super) const fn vsyncs(&self) -> u32 {
        self.wanted
    }

    /// Its line, `samples` emptied for the next: "<local time> <seconds>s drawn <drawn>/<wanted>
    /// <percent>% work <p50/p90/max> ui <p50/p90/max> display <hz>Hz hand <pen|finger|none>
    /// screen <canvas|sheet|page|home>", ms; a spread with no frame drawn is "-", a display rate
    /// not delivered "-".
    pub(super) fn line(&self, samples: &mut Samples) -> String {
        let seconds = f64::from(self.wanted) * VSYNC.as_secs_f64();
        let percent = 100.0 * f64::from(self.drawn) / f64::from(self.wanted.max(1));
        let work = spread(&mut samples.work);
        let ui = spread(&mut samples.ui);
        let display = self
            .interval
            .map_or_else(|| "-".to_owned(), |ns| format!("{:.0}Hz", 1e9 / ns as f64));
        let most = |counts: &[u32]| {
            let top = counts.iter().copied().max().unwrap_or(0);
            counts.iter().position(|count| *count == top).unwrap_or(0)
        };
        format!(
            "{} {seconds:.2}s drawn {}/{} {percent:.0}% work {work} ui {ui} display {display} hand \
             {} screen {}",
            local_time(self.began),
            self.drawn,
            self.wanted,
            HANDS[most(&self.hands)],
            SCREENS[most(&self.screens)]
        )
    }
}

/// "p50/p90/max" (nearest rank, `bench::rank`) of `values`, which are sorted in place and emptied.
fn spread(values: &mut Vec<f64>) -> String {
    if values.is_empty() {
        return "-".to_owned();
    }
    values.sort_by(f64::total_cmp);
    let at = |share| rank(values, share);
    let spread = format!("{:.1}/{:.1}/{:.1}", at(0.5), at(0.9), at(1.0));
    values.clear();
    spread
}

/// "YYYY-MM-DDTHH:MM:SS" on the device's clock: `wall` seconds since 1970 UTC, `offset` minutes
/// east of UTC.
fn local_time((wall, offset): (u64, i32)) -> String {
    let local = i64::try_from(wall).unwrap_or(0) + i64::from(offset) * 60;
    let (days, second) = (local.div_euclid(86_400), local.rem_euclid(86_400));
    let date = Date::of(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        date.year,
        date.month,
        date.day,
        second / 3600,
        second / 60 % 60,
        second % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Buttons, Pointer, PointerSample};

    fn batch(phase: MotionPhase, tool: ToolKind) -> MotionBatch {
        let sample = PointerSample {
            position: [0.0, 0.0],
            pressure: 0.5,
            tilt: 0.0,
            orientation: 0.0,
            time_ms: 0,
        };
        MotionBatch {
            phase,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool,
                samples: vec![sample],
            }],
        }
    }

    #[test]
    fn the_hand_of_a_call_is_what_touched_since_the_call_before() {
        let mut hands = Hands::default();
        assert_eq!(hands.take(), HandKind::Nothing);
        hands.saw(&batch(MotionPhase::HoverMove, ToolKind::Stylus));
        assert_eq!(
            hands.take(),
            HandKind::Nothing,
            "a pen hovering is not down"
        );
        hands.saw(&batch(MotionPhase::Down, ToolKind::Finger));
        hands.saw(&batch(MotionPhase::Up, ToolKind::Finger));
        assert_eq!(hands.take(), HandKind::Finger, "a tap between two calls");
        assert_eq!(hands.take(), HandKind::Nothing, "lifted");
        hands.saw(&batch(MotionPhase::Down, ToolKind::Stylus));
        hands.saw(&batch(MotionPhase::Down, ToolKind::Finger));
        assert_eq!(hands.take(), HandKind::Pen, "the pen goes first");
        hands.saw(&batch(MotionPhase::Up, ToolKind::Stylus));
        assert_eq!(hands.take(), HandKind::Pen, "lifted since the call before");
        assert_eq!(hands.take(), HandKind::Finger, "the finger still down");
    }

    #[test]
    fn a_line_says_when_how_long_and_what_was_drawn_of_what_was_wanted() {
        // 2026-11-14 10:32:05 at UTC+8.
        let mut stretch = Stretch::new((1_794_623_525, 480));
        let mut samples = Samples::default();
        let call = |missed, drawn, work| Counted {
            missed,
            drawn,
            work,
            ui: work / 2.0,
            hand: HandKind::Pen,
            screen: ScreenKind::Canvas,
        };
        for k in 0..10 {
            stretch.count(call(0, true, f64::from(k + 1)), &mut samples);
        }
        stretch.count(call(1, false, 50.0), &mut samples);
        let line = stretch.line(&mut samples);
        assert_eq!(
            line,
            "2026-11-14T10:32:05 0.10s drawn 10/12 83% work 5.0/9.0/10.0 ui 2.5/4.5/5.0 display - \
             hand pen screen canvas"
        );
        assert!(samples.work.is_empty() && samples.work.capacity() == SAMPLES);
    }

    #[test]
    fn a_line_carries_the_display_s_rate_as_choreographer_delivered_it() {
        // A stretch the panel showed at 60 Hz: two vsyncs of 120 Hz between calls, one missed.
        let mut stretch = Stretch::new((1_794_623_525, 0));
        let mut samples = Samples::default();
        for interval in [33_333_333, 16_666_667, 16_666_666, 0] {
            stretch.delivered(interval);
        }
        let line = stretch.line(&mut samples);
        assert!(line.contains(" display 60Hz "), "{line}");
        stretch.delivered(8_333_333);
        assert!(stretch.line(&mut samples).contains(" display 120Hz "));
    }
}
