//! The frame budget for background work (S53, DECISIONS D80): what the screen shows waits for no
//! work it can show without. Each frame of the screen takes, of everything that waits (the
//! canvas's tiles filling in, a view tier's tiles, a smudge residual being evaluated), only what
//! its estimates say fits `BACKGROUND_NS` of GPU time; the rest waits for the next frames.
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
use crate::gpu_clock::{GpuClock, Timing};

/// The GPU time one frame gives background work: half a 120 Hz frame on the tablet, the rest
/// for what the frame shows (the canvas or tier drawn to the screen, the interface, a stroke).
pub const BACKGROUND_NS: f64 = 4_000_000.0;

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

/// The tiles of the dispatch being gathered: how many, their estimates' sum and largest.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Group {
    pub(crate) tiles: usize,
    pub(crate) sum: f64,
    pub(crate) most: f64,
}

impl Group {
    fn with(self, cost: f64) -> Self {
        Self {
            tiles: self.tiles + 1,
            sum: self.sum + cost,
            most: self.most.max(cost),
        }
    }

    /// What the GPU takes for them, estimated.
    fn time(self) -> f64 {
        (self.sum / PARALLEL).max(self.most)
    }
}

pub(crate) struct Pace {
    budget: f64,
    /// What this frame may still give, in measured ns (below 0: owed by the next frames), whether
    /// it had any at its start, and whether it gave anything yet.
    left: f64,
    opened: bool,
    untouched: bool,
    /// The measured time over the estimated, from earlier frames' readings.
    ratio: f64,
    sums: [f64; 2],
    clock: Option<GpuClock>,
    group: Group,
    /// This frame's spending so far (measured ns), its pieces, and its first piece's.
    spent: f64,
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
            budget: BACKGROUND_NS,
            left: BACKGROUND_NS,
            opened: true,
            untouched: true,
            ratio: FIRST_RATIO,
            sums: [0.0; 2],
            clock: GpuClock::new(gpu),
            group: Group::default(),
            spent: 0.0,
            pieces: 0,
            first: 0.0,
            last: Background::default(),
        }
    }

    /// A new frame: the readings back since are learnt, the budget is whole again less what the
    /// frames before spent beyond theirs.
    pub(crate) fn begin(&mut self, gpu: &Gpu) {
        let readings = self.clock.as_mut().map(|clock| clock.readings(gpu));
        self.last = Background {
            spent: self.spent,
            ratio: self.ratio,
            measured: (readings.as_ref()).map(|all| all.iter().map(|(_, measured)| measured).sum()),
            pieces: self.pieces,
            first: self.first,
        };
        for (group, measured) in readings.iter().flatten() {
            self.sums = [
                self.sums[0] * DECAY + group.time(),
                self.sums[1] * DECAY + measured,
            ];
        }
        if self.sums[0] > 0.0 {
            self.ratio = (self.sums[1] / self.sums[0]).clamp(RATIO[0], RATIO[1]);
        }
        self.left = self.budget + self.left.min(0.0);
        self.opened = self.left > 0.0;
        self.untouched = true;
        self.group = Group::default();
        (self.spent, self.pieces, self.first) = (0.0, 0, 0.0);
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

    /// The budget is `ns` from this frame on, whole again.
    pub(crate) fn set_budget(&mut self, ns: f64) {
        self.budget = ns;
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
