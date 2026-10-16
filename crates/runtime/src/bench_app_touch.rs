//! `bench touch` (S60, `bench_app.rs`): a hand arriving while the screen fills in. The view tiers
//! let go (as leaving the app does), from fit over the densest place a pinch to 8x (the deepest
//! tier the artwork reaches), then while the tier's
//! tiles fill a pen touching down at the screen's middle (held, lifted; its dot undone at the
//! end), and later two fingers landing and panning. For each: from the event to the start of the
//! first frame drawn after it, to the return of that frame's present, and to the GPU finishing
//! it (with every frame before it: what the GPU had queued ahead of it), and the frame calls
//! skipped meanwhile. D81 bounds what a touch's first frame waits for.

use std::time::{Duration, Instant};

use openpaint_app::{HistoryIntent, Intent};

use super::Watch;
use super::cases::{batch, busiest, strokes};
use crate::budget::TRIM_UI_HIDDEN;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::{MotionPhase, ToolKind};
use crate::screen::Screen;

const LIMIT: Duration = Duration::from_secs(60);
/// How far into the fill the hand arrives, and how long it stays.
const INTO: Duration = Duration::from_millis(750);
const HELD: Duration = Duration::from_millis(150);
/// The fingers' batches: one each 8 ms of the input clock, 8 screen pixels further.
const STEP_MS: i64 = 8;
const STEP_PX: f32 = 8.0;

/// The hand arriving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hand {
    Pen,
    Fingers,
}

impl Runtime {
    /// `touch`: see the module.
    pub(crate) fn bench_touch(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let place = busiest(&artwork);
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        self.bench_centre(place)?;
        // The tiers held go as leaving the app lets them go: the pinch fills its tier anew.
        self.release(TRIM_UI_HIDDEN);
        let pinch = self.bench_pinch(8.0)?;
        lines.push(format!(
            "pinch to 8x over ({:.0}, {:.0}): {pinch}",
            place[0], place[1]
        ));
        for hand in [Hand::Pen, Hand::Fingers] {
            let mut watch = Watch::paced(self);
            self.bench_for(&mut watch, INTO)?;
            let filling = (self.screen.canvas()).is_some_and(|c| c.view_pending() || c.filling());
            let first = self.bench_arrival(hand)?;
            let (who, when) = (
                if hand == Hand::Pen {
                    "a pen touching down"
                } else {
                    "two fingers panning"
                },
                if filling { "during" } else { "after" },
            );
            lines.push(format!("{who} {when} the fill: {first}"));
        }
        let mut watch = Watch::paced(self);
        let sharp = self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        let state = if sharp {
            "sharp after"
        } else {
            "NOT sharp within the limit:"
        };
        lines.push(format!("{state} {}", watch.line(self)));
        // The pen's dot goes again: the artwork is left as it was found.
        let found = strokes(&artwork);
        for _ in 0..2 {
            if self.artwork().is_none_or(|now| strokes(now) <= found) {
                break;
            }
            if let Screen::Canvas(canvas) = &mut self.screen {
                let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
                let undo = Intent::History(HistoryIntent::Undo);
                canvas.dispatch(&self.core, vec![undo], app);
            }
            let mut watch = Watch::paced(self);
            self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        }
        Ok(())
    }

    /// Frames for `time`.
    fn bench_for(&mut self, watch: &mut Watch, time: Duration) -> Result<(), RuntimeError> {
        let started = Instant::now();
        while started.elapsed() < time {
            self.bench_frame(watch)?;
        }
        Ok(())
    }

    /// `hand` arriving at the screen's middle, its first frame timed, then held (the fingers
    /// panning) for `HELD` and lifted.
    fn bench_arrival(&mut self, hand: Hand) -> Result<String, RuntimeError> {
        let at = self.bench_middle()?;
        let (tool, points) = match hand {
            Hand::Pen => (ToolKind::Stylus, vec![at]),
            Hand::Fingers => (
                ToolKind::Finger,
                vec![[at[0] - 150.0, at[1]], [at[0] + 150.0, at[1]]],
            ),
        };
        let start = self.clock.now_ms() + 50;
        let mut watch = Watch::paced(self);
        let queued = self.core.background_queued() / 1e6;
        let event = Instant::now();
        self.motion(&batch(MotionPhase::Down, tool, &points[..1], start));
        if hand == Hand::Fingers {
            let second = MotionPhase::PointerDown { index: 1 };
            self.motion(&batch(second, tool, &points, start));
        }
        let first = self.bench_first(&mut watch, event)?;
        let held = Instant::now();
        let mut moved = points.clone();
        let mut time = start;
        while held.elapsed() < HELD {
            // A pen held still sends nothing; the fingers pan.
            time += STEP_MS;
            if hand == Hand::Fingers {
                moved = moved.iter().map(|p| [p[0] + STEP_PX, p[1]]).collect();
                self.motion(&batch(MotionPhase::Move, tool, &moved, time));
            }
            self.bench_frame(&mut watch)?;
        }
        if hand == Hand::Fingers {
            let second = MotionPhase::PointerUp { index: 1 };
            self.motion(&batch(second, tool, &moved, time));
        }
        self.motion(&batch(MotionPhase::Up, tool, &moved[..1], time));
        self.bench_drawn(&mut watch)?;
        Ok(format!(
            "{first}; the GPU's queue held {queued:.1} ms of background work then"
        ))
    }

    /// Frame calls until one is drawn after `event`: from the event to that frame's start, to its
    /// present's return and to the GPU finishing it, and the calls skipped before it.
    fn bench_first(&mut self, watch: &mut Watch, event: Instant) -> Result<String, RuntimeError> {
        let skipped = watch.skipped();
        loop {
            let drawn = watch.frames.len();
            let began = Instant::now();
            self.bench_call(watch)?;
            if watch.frames.len() == drawn {
                watch.wait(true);
                continue;
            }
            let presented = Instant::now();
            while self.core.frames_in_flight() > 0 && presented.elapsed() < Duration::from_secs(1) {
                std::thread::sleep(Duration::from_micros(200));
            }
            let finished = Instant::now();
            watch.wait(false);
            let ms = |at: Instant| (at - event).as_secs_f64() * 1e3;
            return Ok(format!(
                "to its first frame {:.1} ms, to its present {:.1} ms, to the GPU done with it \
                 {:.1} ms ({} calls skipped)",
                ms(began),
                ms(presented),
                ms(finished),
                watch.skipped() - skipped
            ));
        }
    }
}
