//! The recorder of frames in real use (the debug build only): while the app is used by hand, not
//! under the bench, every stretch of motion is measured with the bench's own counters and kept as
//! one line in a file among the app's files (`FRAMES_FILE`, at most `FRAMES_KEPT` lines, the
//! newest), which `cargo device-frames` reads.
//!
//! A stretch is a run of consecutive vsyncs (`bench::VSYNC`, 120 Hz) that want a frame: one the
//! frame call before asked for (it returned `again`), or one an input batch asked for at it (the
//! nearest); it ends at a vsync none wants, found at the next frame call or input after it, or
//! when the app is left. A frame call comes `n` vsyncs after the one before (its start's distance,
//! rounded): the vsyncs between that wanted a frame found none drawn (missed), as the bench counts
//! them, and a call skipped while the GPU is behind draws none either. Stretches under `SHORTEST`
//! vsyncs (a quarter of a second) are not kept. The main thread away from frames for `AWAY` while one
//! was wanted (a debug chore or the bench running frames of its own) ends a stretch, the time away
//! not counted. A stretch's line also carries the display's refresh rate during it, as
//! Choreographer delivered its vsyncs (`delivered`: the shortest interval between two consecutive
//! calls' vsyncs), so a stretch the panel showed at 60 Hz reads as such.
//!
//! Its cost a frame: two clock reads, a few counters and two pushes into buffers allocated once;
//! a line is formed when a stretch ends, before the next frame's work, and written on a worker.

mod file;
mod stretch;

use std::path::PathBuf;

use openpaint_practice_app::PracticeSheet;

use crate::bench::VSYNC;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::MotionBatch;
use crate::screen::Screen;
pub use file::FRAMES_KEPT;
use file::FramesFile;
use stretch::{Counted, Hands, Samples, ScreenKind, Stretch};

/// The recorder's file among the app's files (`Context.getFilesDir()`).
pub const FRAMES_FILE: &str = "frames.txt";
/// The fewest vsyncs a stretch kept lasts: a quarter of a second at 120 Hz.
const SHORTEST: u32 = 30;
/// Seconds the main thread may spend away from frames while one is wanted before the stretch ends.
const AWAY: f64 = 1.0;

/// Measures the host's frame calls into stretches of motion and keeps a line for each.
pub struct FrameRecorder {
    /// The frame call before: its start and end on the frame clock, s, and whether it wanted the
    /// next.
    last: Option<Last>,
    /// Choreographer's vsync for the frame call to come and the one before's, ns (`delivered`).
    vsync: Option<i64>,
    last_vsync: Option<i64>,
    /// The earliest input since the frame call before that asked for a frame, s.
    requested: Option<f64>,
    open: Option<Stretch>,
    samples: Samples,
    hands: Hands,
    file: FramesFile,
}

#[derive(Debug, Clone, Copy)]
struct Last {
    at: f64,
    end: f64,
    again: bool,
}

impl FrameRecorder {
    /// A recorder keeping its lines in the file at `path`.
    ///
    /// # Errors
    /// `Worker` when the file's worker cannot start.
    pub fn writing_to(path: PathBuf) -> Result<Self, RuntimeError> {
        Ok(Self {
            last: None,
            vsync: None,
            last_vsync: None,
            requested: None,
            open: None,
            samples: Samples::default(),
            hands: Hands::default(),
            file: FramesFile::at(path)?,
        })
    }

    /// The app is left (or the recorder dropped): the stretch open ends here, kept if long enough;
    /// the next frame call starts afresh.
    pub fn end(&mut self) {
        self.close();
        self.last = None;
        (self.vsync, self.last_vsync) = (None, None);
        self.requested = None;
    }

    /// The vsync Choreographer delivered the next frame call at (`frameTimeNanos`, ns): two of
    /// consecutive calls in a stretch are the display's actual frame interval, which its line
    /// carries.
    pub const fn delivered(&mut self, vsync_ns: i64) {
        self.vsync = Some(vsync_ns);
    }

    /// An input batch the app took at `now` (s), which asked for a frame when `wanted`. A vsync
    /// since the frame call before that wanted none has ended the stretch open.
    fn input(&mut self, batch: &MotionBatch, wanted: bool, now: f64) {
        self.hands.saw(batch);
        if let Some(last) = self.last
            && self.requested.is_none()
            && !last.again
            && vsyncs(last.at, now) > 1.5
        {
            self.close();
        }
        if wanted {
            self.requested = Some(self.requested.map_or(now, |since| since.min(now)));
        }
    }

    /// Before a frame call at `at` (s): the vsyncs since the call before that wanted a frame and
    /// found none drawn, and whether the stretch open goes on through them (no vsync between
    /// wanted none); when it does not, it has ended, kept if long enough.
    fn before(&mut self, at: f64) -> u32 {
        let missed = self.last.and_then(|last| {
            if at - last.end > AWAY {
                return None;
            }
            let n = vsyncs(last.at, at).round().max(1.0) as u32;
            let first = match (last.again, self.requested) {
                (true, _) => 1,
                // Input comes at a vsync (Android hands it over batched): the nearest one.
                (false, Some(since)) => {
                    ((vsyncs(last.at, since) - 0.5).ceil().max(1.0) as u32).min(n)
                }
                // Asked by the host for its own reasons (a typed key, the Pomodoro's tick).
                (false, None) => n,
            };
            Some((first == 1, n - first))
        });
        let (goes_on, missed) = missed.unwrap_or((false, 0));
        if !goes_on {
            self.close();
        }
        missed
    }

    /// The stretch open ends: its line to the file when it lasted `SHORTEST` vsyncs or more.
    fn close(&mut self) {
        if let Some(stretch) = self.open.take() {
            if stretch.vsyncs() >= SHORTEST {
                let line = stretch.line(&mut self.samples);
                self.file.keep(line);
            }
            self.samples.clear();
        }
    }
}

/// Vsyncs from `from` to `to` (s), fractions kept.
fn vsyncs(from: f64, to: f64) -> f64 {
    (to - from) / VSYNC.as_secs_f64()
}

impl Drop for FrameRecorder {
    fn drop(&mut self) {
        self.close();
    }
}

impl Runtime {
    /// `motion`, the batch's hand and its asking for a frame told to `recorder`.
    pub fn motion_recorded(&mut self, batch: &MotionBatch, recorder: &mut FrameRecorder) -> bool {
        let wanted = self.motion(batch);
        recorder.input(batch, wanted, self.frame_clock.seconds());
        wanted
    }

    /// One frame as the host draws it (into the window; offscreen without one), measured by
    /// `recorder` as the bench measures a frame. Returns whether another frame is wanted at once.
    ///
    /// # Errors
    /// As `frame`.
    pub fn frame_recorded(&mut self, recorder: &mut FrameRecorder) -> Result<bool, RuntimeError> {
        let at = self.frame_clock.seconds();
        let consecutive = recorder.last.is_some_and(|last| last.again);
        let missed = recorder.before(at);
        let vsync = recorder.vsync.take();
        let interval = (vsync.zip(recorder.last_vsync))
            .filter(|_| consecutive)
            .map(|(now, before)| now - before);
        recorder.last_vsync = vsync;
        let (again, work) = self.frame_timed()?;
        let call = Counted {
            missed,
            drawn: !self.paced().skipped,
            work,
            ui: self.ui_ns as f64 / 1e6,
            hand: recorder.hands.take(),
            screen: self.screen_kind(),
        };
        let began = || (self.frame_clock.wall_seconds(), self.utc_offset);
        let stretch = recorder.open.get_or_insert_with(|| Stretch::new(began()));
        stretch.count(call, &mut recorder.samples);
        if let Some(interval) = interval {
            stretch.delivered(interval);
        }
        let end = self.frame_clock.seconds();
        recorder.last = Some(Last { at, end, again });
        recorder.requested = None;
        Ok(again)
    }

    /// Which screen the last frame drew.
    fn screen_kind(&self) -> ScreenKind {
        match &self.screen {
            Screen::Canvas(canvas) if PracticeSheet::of(canvas.session()).is_some() => {
                ScreenKind::Sheet
            }
            Screen::Canvas(_) => ScreenKind::Canvas,
            Screen::Gallery(..) if self.practice_screens.page_shows() => ScreenKind::Page,
            Screen::Gallery(..) | Screen::Opening(_) => ScreenKind::Home,
        }
    }
}
