//! The main thread's frames as frame pacing sees them (S56), logged as they happen: a frame
//! whose work or one of whose waits for the GPU passes `SLOW_NS` (what lets Android's input
//! timeout come near when it repeats), and every run of frame calls skipped while the GPU was
//! behind that lasted as long, with the input batches taken meanwhile (input is never starved).

use openpaint_runtime::{Paced, Spent};

/// A frame, a wait or a run of skipped frames this long is logged.
pub const SLOW_NS: i64 = 100_000_000;

#[derive(Debug, Default)]
pub struct PacingLog {
    /// Input batches taken since the last frame call.
    batches: u32,
    /// The frame calls skipped in a row so far, when the last one was.
    run: Option<Run>,
}

#[derive(Debug, Clone, Copy)]
struct Run {
    since_ns: i64,
    frames: u32,
    batches: u32,
}

/// One frame call: when it started, its work (ns), what pacing did, what the thread spent.
#[derive(Debug, Clone, Copy)]
pub struct Called {
    pub start_ns: i64,
    pub work_ns: i64,
    pub paced: Paced,
    pub spent: Spent,
}

impl PacingLog {
    /// An input batch reached the app.
    pub fn input(&mut self) {
        self.batches += 1;
    }

    /// A frame call ran; returns the lines to log about it.
    pub fn frame(&mut self, called: &Called) -> Vec<String> {
        let batches = std::mem::take(&mut self.batches);
        if called.paced.skipped {
            let run = self.run.get_or_insert(Run {
                since_ns: called.start_ns,
                frames: 0,
                batches: 0,
            });
            run.frames += 1;
            run.batches += batches;
            return Vec::new();
        }
        let mut lines = Vec::new();
        if let Some(run) = self.run.take()
            && called.start_ns - run.since_ns >= SLOW_NS
        {
            lines.push(format!(
                "the GPU was behind for {} ms: {} frame calls skipped, {} input batches taken \
                 meanwhile",
                (called.start_ns - run.since_ns) / 1_000_000,
                run.frames,
                run.batches + batches
            ));
        }
        let spent = &called.spent;
        let longest = i64::try_from(spent.longest_ns).unwrap_or(i64::MAX);
        if called.work_ns >= SLOW_NS || longest >= SLOW_NS {
            let ms = |ns: u64| ns as f64 / 1e6;
            lines.push(format!(
                "slow frame: {:.1} ms, waited {:.1} ms for the GPU (longest {:.1} ms in {}), \
                 {} scenes built in {:.1} ms, {} frames in flight at its start, {batches} input \
                 batches since the frame call before",
                called.work_ns as f64 / 1e6,
                ms(spent.waited_ns),
                ms(spent.longest_ns),
                spent.longest,
                spent.scenes,
                ms(spent.scene_ns),
                called.paced.in_flight
            ));
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: i64 = 1_000_000;

    fn called(start_ms: i64, work_ms: i64, skipped: bool, longest_ms: u64) -> Called {
        Called {
            start_ns: start_ms * MS,
            work_ns: work_ms * MS,
            paced: Paced {
                skipped,
                in_flight: 2,
            },
            spent: Spent {
                waited_ns: longest_ms * 1_000_000,
                longest_ns: longest_ms * 1_000_000,
                longest: "acquire",
                scenes: 0,
                scene_ns: 0,
                evaluated_ns: 0,
            },
        }
    }

    #[test]
    fn slow_frames_and_long_skipped_runs_are_logged() {
        let mut log = PacingLog::default();
        assert!(log.frame(&called(0, 5, false, 1)).is_empty());
        log.input();
        assert!(log.frame(&called(8, 0, true, 0)).is_empty());
        log.input();
        log.input();
        assert!(log.frame(&called(60, 0, true, 0)).is_empty());
        let lines = log.frame(&called(150, 120, false, 110));
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].starts_with("the GPU was behind for 142 ms: 2 frame calls skipped"));
        assert!(
            lines[0].ends_with("3 input batches taken meanwhile"),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].contains("longest 110.0 ms in acquire"),
            "{}",
            lines[1]
        );
        // A short run is not logged.
        assert!(log.frame(&called(160, 0, true, 0)).is_empty());
        assert!(log.frame(&called(170, 4, false, 1)).is_empty());
    }
}
