//! The frame budget for background work (S53, DECISIONS D80): what the screen shows waits for no
//! work it can show without. Each frame of the screen takes, of everything that waits (the
//! canvas's tiles filling in, a view tier's tiles, a smudge residual being evaluated), only what
//! its estimates say fits `BACKGROUND_NS` of GPU time; the rest waits for the next frames.
//!
//! S60 (DECISIONS D87): while nothing touches the screen, a step (a frame drawn, or a frame call
//! skipped while the GPU is behind) gives background work what keeps the GPU's queue of it within
//! `IDLE_BACKGROUND_NS` (`Core::background_queued`), a first piece whatever it costs only when
//! the queue is empty: so work waiting runs at the GPU's pace, between the frames drawn, and a
//! frame drawn when a hand arrives waits for at most that much. While the screen is touched each
//! frame drawn gives `BACKGROUND_NS`, as before.
//!
//! A dispatch's estimate follows how the GPU runs it: each plan tile is one workgroup whose 256
//! pixels each walk the tile's whole list, and the GPU runs about `PARALLEL` of them side by side,
//! so up to that many cost about what the costliest does, more about their sum over it (`cost.rs`
//! gives each tile's sum). The estimates are scaled by how long the GPU really took for earlier
//! frames' background work (`gpu_clock.rs`): the measured time over the estimated, decayed so the
//! latest count most. A frame that starts with budget left takes one piece whatever its estimate,
//! so a piece above the budget is still done; what a frame spends beyond its budget is owed by
//! the next ones, so on average no frame spends more.

use crate::gpu::Gpu;
use crate::gpu_clock::{GpuClock, Stamps, Timing};

/// The GPU time one frame gives background work: half a 120 Hz frame on the tablet, the rest
/// for what the frame shows (the canvas or tier drawn to the screen, the interface, a stroke).
pub const BACKGROUND_NS: f64 = 4_000_000.0;

/// The background work the GPU's queue holds at most while nothing touches the screen (S60; S54
/// gave a frame this much): the first frame of a touch waits for at most this much.
pub const IDLE_BACKGROUND_NS: f64 = 10_000_000.0;

/// Plan tiles the tablet's GPU (an Adreno 735) evaluates side by side, measured S53 with GPU
/// timestamps on the author's A4 artwork: one tile about 3.5 ms, two to six about 3.3 to 4.7,
/// nine 8.7 to 9.1, twelve about 10: the sum over 3.6 or the largest, whichever is more.
pub(crate) const PARALLEL: f64 = 4.0;

/// The correction before the first reading: what those readings gave on the tablet (a tile the
/// table estimates at 0.2 ms took 3.5), so the first frames after a start take no more than
/// their budget either.
const FIRST_RATIO: f64 = 16.0;

/// How much of the measured and estimated sums each reading keeps: about the last 30 readings
/// count.
const DECAY: f64 = 0.97;
/// The correction stays within these: one wild reading never stops background work (a desktop
/// GPU runs the table's tablet estimates about thirty times faster).
const RATIO: [f64; 2] = [0.01, 20.0];

/// The tiles of the dispatch being gathered: how many, their estimates' sum and largest; or
/// (`raster`, S59) one region the raster core draws, estimated by its own table.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Group {
    pub(crate) tiles: usize,
    pub(crate) sum: f64,
    pub(crate) most: f64,
    pub(crate) raster: bool,
}

impl Group {
    /// A region of the raster core estimated at `cost` ns.
    pub(crate) const fn raster(cost: f64) -> Self {
        Self {
            tiles: 1,
            sum: cost,
            most: cost,
            raster: true,
        }
    }

    fn with(self, cost: f64) -> Self {
        Self {
            tiles: self.tiles + 1,
            sum: self.sum + cost,
            most: self.most.max(cost),
            raster: false,
        }
    }

    /// What the GPU takes for them, estimated.
    fn time(self) -> f64 {
        (self.sum / PARALLEL).max(self.most)
    }
}

pub(crate) struct Pace {
    /// The budget a frame gives while the screen is touched, and the queue's most while it is
    /// not (S54, S60).
    budgets: [f64; 2],
    /// This step is one while nothing touches the screen (S60).
    idle: bool,
    /// What this frame may still give, in measured ns (below 0: owed by the next frames), whether
    /// it had any at its start, and whether it gave anything yet.
    left: f64,
    opened: bool,
    untouched: bool,
    /// The measured time over the estimated, from earlier frames' readings; the raster core's
    /// apart (S59: its table is the tablet's measured time, the evaluator's ranks kinds only).
    ratio: f64,
    sums: [f64; 2],
    raster_ratio: f64,
    raster_sums: [f64; 2],
    clock: Option<GpuClock>,
    group: Group,
    /// This step's spending so far (measured ns), its pieces, and its first piece's; how much of
    /// it is marked as queued on the GPU (S60).
    spent: f64,
    marked: f64,
    pieces: usize,
    first: f64,
    last: Background,
}

/// What the frame before took for background work (S53's logs and tests).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Background {
    /// GPU time it took by the corrected estimates, ns.
    pub spent: f64,
    /// The correction it was taken with: the measured time over the estimated.
    pub ratio: f64,
    /// GPU time measured for background work whose readings came back during the frame, ns;
    /// `None` without timestamp queries.
    pub measured: Option<f64>,
    /// GPU time measured for background work since the device was opened, ns (S60: steps between
    /// the frames drawn read some back too).
    pub total: f64,
    /// Pieces of work it took (plan tiles, a residual's material tiles and contacts), and what
    /// its first took.
    pub pieces: usize,
    pub first: f64,
}

impl Background {
    /// Whether the frame kept within `budget` ns by its corrected estimates, but for a first piece
    /// above it (and the tiles running beside that one).
    #[must_use]
    pub fn within(&self, budget: f64) -> bool {
        self.spent <= budget.max(self.first) * (1.0 + 1e-9)
    }
}

impl Pace {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        Self {
            budgets: [BACKGROUND_NS, IDLE_BACKGROUND_NS],
            idle: false,
            left: BACKGROUND_NS,
            opened: true,
            untouched: true,
            ratio: FIRST_RATIO,
            sums: [0.0; 2],
            raster_ratio: 1.0,
            raster_sums: [0.0; 2],
            clock: GpuClock::new(gpu),
            group: Group::default(),
            spent: 0.0,
            marked: 0.0,
            pieces: 0,
            first: 0.0,
            last: Background::default(),
        }
    }

    /// A new step: the readings back since are learnt; touched (`queued` `None`), the budget is
    /// whole again less what the frames before spent beyond theirs; idle, it is what keeps the
    /// background work `queued` on the GPU (ns) within its most, a first piece taken whatever it
    /// costs only when nothing is queued (S60).
    pub(crate) fn begin(&mut self, gpu: &Gpu, queued: Option<f64>) {
        let readings = self.clock.as_mut().map(|clock| clock.readings(gpu));
        self.last = Background {
            spent: self.spent,
            ratio: self.ratio,
            measured: (readings.as_ref()).map(|all| all.iter().map(|(_, measured)| measured).sum()),
            total: self.last.total
                + (readings.iter().flatten())
                    .map(|(_, measured)| measured)
                    .sum::<f64>(),
            pieces: self.pieces,
            first: self.first,
        };
        for (group, measured) in readings.iter().flatten() {
            let sums = if group.raster {
                &mut self.raster_sums
            } else {
                &mut self.sums
            };
            *sums = [sums[0] * DECAY + group.time(), sums[1] * DECAY + measured];
        }
        if self.sums[0] > 0.0 {
            self.ratio = (self.sums[1] / self.sums[0]).clamp(RATIO[0], RATIO[1]);
        }
        if self.raster_sums[0] > 0.0 {
            let ratio = self.raster_sums[1] / self.raster_sums[0];
            self.raster_ratio = ratio.clamp(RATIO[0], RATIO[1]);
        }
        self.open(queued);
    }

    /// The step's budget: see `begin`.
    fn open(&mut self, queued: Option<f64>) {
        match queued {
            Some(queued) => {
                self.left = self.budgets[1] - queued;
                self.opened = queued <= 0.0;
            }
            None => {
                // An idle step's first piece owes nothing to the touched frames after it.
                let owed = if self.idle { 0.0 } else { self.left.min(0.0) };
                self.left = self.budgets[0] + owed;
                self.opened = self.left > 0.0;
            }
        }
        self.idle = queued.is_some();
        self.untouched = true;
        self.group = Group::default();
        (self.spent, self.marked, self.pieces, self.first) = (0.0, 0.0, 0, 0.0);
    }

    /// Whether this step is one while nothing touches the screen (S60).
    pub(crate) const fn idle(&self) -> bool {
        self.idle
    }

    /// The GPU time of the background work this step gave since the last call, by its corrected
    /// estimates: what was submitted since, to be marked as queued (each dispatch apart, so the
    /// queue's measure falls as the GPU works through a step's pieces).
    pub(crate) fn unmarked(&mut self) -> f64 {
        let unmarked = self.spent - self.marked;
        self.marked = self.spent;
        unmarked
    }

    /// The most background work the GPU's queue holds while nothing touches the screen (S60).
    pub(crate) const fn most_queued(&self) -> f64 {
        self.budgets[1]
    }

    /// The background work a frame drawn while the screen is touched gives (S60: the most the
    /// raster worker queues then).
    pub(crate) const fn share(&self) -> f64 {
        self.budgets[0]
    }

    /// Adds a tile estimated at `cost` ns to the dispatch being gathered when what it adds fits
    /// what is left of the frame's budget (or the frame, opened with budget, gave nothing yet);
    /// false leaves it waiting.
    pub(crate) fn take(&mut self, cost: f64) -> bool {
        let group = self.group.with(cost);
        let extra = (group.time() - self.group.time()) * self.ratio;
        if !(self.untouched && self.opened) && extra > self.left {
            return false;
        }
        self.group = group;
        self.charge(extra);
        true
    }

    /// Work of `sum` ns of tiles, the costliest `most`, taken only when it fits what is left of
    /// the frame's budget, given anything before or not: what a frame may show at once or let
    /// wait. It is a dispatch of its own.
    pub(crate) fn fits(&mut self, sum: f64, most: f64) -> bool {
        self.close();
        let time = Group {
            tiles: 0,
            sum,
            most,
            raster: false,
        }
        .time()
            * self.ratio;
        if time > self.left {
            return false;
        }
        self.charge(time);
        true
    }

    /// Work that runs alone, one dispatch after another (a smudge's contacts), estimated at
    /// `cost` ns as measured: taken as `take` takes a tile.
    pub(crate) fn take_serial(&mut self, cost: f64) -> bool {
        if !(self.untouched && self.opened) && cost > self.left {
            return false;
        }
        self.charge(cost);
        true
    }

    /// Work of `time` ns another thread submits and marks itself (S60: a tile the raster worker
    /// draws while the screen is touched): taken from what is left of the frame's budget as
    /// `take_serial` takes it, but not counted as this step's own.
    pub(crate) fn promise(&mut self, time: f64) -> bool {
        if !(self.untouched && self.opened) && time > self.left {
            return false;
        }
        self.left -= time;
        self.untouched = false;
        true
    }

    /// A region of the raster core estimated at `cost` ns by its own table (S59), drawn on a
    /// worker (S60): its GPU time by the raster core's own correction, and the timestamps that
    /// measure it when a slot is free (`raster_finish` once submitted).
    pub(crate) fn raster_start(&mut self, cost: f64) -> (f64, Option<Stamps>) {
        let stamps = self.clock.as_mut().and_then(|clock| {
            let timing = clock.start(Group::raster(cost))?;
            Some(clock.stamps(timing))
        });
        (cost * self.raster_ratio, stamps)
    }

    /// The region `raster_start` measured was submitted (`drawn`): its timestamps go to be read
    /// back; else its slot is free again.
    pub(crate) fn raster_finish(&mut self, gpu: &Gpu, stamps: &Stamps, drawn: bool) {
        match (drawn, self.clock.as_mut()) {
            (true, _) => self.finish(gpu, stamps.timing),
            (false, Some(clock)) => clock.abandon(stamps.timing),
            (false, None) => {}
        }
    }

    /// A raster region of `cost` ns drawn on this thread (S62: a smudge's material over frames),
    /// by the raster core's correction: taken as `take_serial` takes work.
    pub(crate) fn take_raster(&mut self, cost: f64) -> bool {
        self.take_serial(cost * self.raster_ratio)
    }

    /// Raster regions of `cost` ns taken only when they fit what is left of the frame's budget,
    /// given anything before or not: what a frame may show at once or let wait (as `fits`).
    pub(crate) fn fits_raster(&mut self, cost: f64) -> bool {
        self.close();
        let time = cost * self.raster_ratio;
        if time > self.left {
            return false;
        }
        self.charge(time);
        true
    }

    fn charge(&mut self, time: f64) {
        if self.untouched {
            self.first = time;
        }
        self.left -= time;
        self.spent += time;
        self.untouched = false;
        self.pieces += 1;
    }

    /// The dispatch gathered is complete: its tiles, for its GPU time's reading; the next tiles
    /// gather a new one.
    pub(crate) fn close(&mut self) -> Group {
        std::mem::take(&mut self.group)
    }

    /// Whether what is left of the frame's budget is gone.
    pub(crate) fn spent(&self) -> bool {
        !(self.untouched && self.opened) && self.left <= 0.0
    }

    /// The budget is `ns` from this frame on, whole again, touched or not (idle: the queue's
    /// most).
    pub(crate) fn set_budget(&mut self, ns: f64) {
        self.budgets = [ns; 2];
        self.left = ns;
        self.opened = true;
        self.untouched = true;
    }

    pub(crate) const fn last(&self) -> Background {
        self.last
    }

    /// Measures a background evaluation of `group`, when a slot is free.
    pub(crate) fn time(&mut self, group: Group) -> Option<Timing> {
        self.clock.as_mut()?.start(group)
    }

    pub(crate) fn clock(&self) -> Option<&GpuClock> {
        self.clock.as_ref()
    }

    pub(crate) fn finish(&mut self, gpu: &Gpu, timing: Timing) {
        if let Some(clock) = self.clock.as_mut() {
            clock.finish(gpu, timing);
        }
    }
}

#[cfg(test)]
#[path = "pace_tests.rs"]
mod tests;
