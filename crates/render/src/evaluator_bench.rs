//! The evaluator's micro-benchmark (S55): an artwork evaluated over one square of output pixels
//! at a scale, its plan binned once, then dispatched again and again with each evaluation timed
//! by GPU timestamps (or, on a device without them, by the wall time of a waited submission),
//! so what the GPU takes for each kind of op is read without the CPU's scene and binning. The
//! runtime's `bench` builds the scenes; the tablet runs it through the debug build, the dev
//! machine through the harness.

use std::time::Instant;

use openpaint_core::{Artwork, Color};

use super::{CanvasTarget, RegionEvaluator};
use crate::affine::Affine;
use crate::core::Core;
use crate::cost::{item_kinds, tile_cost};
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::gpu_clock::{GpuClock, Timing};
use crate::output::Output;
use crate::pace::Group;
use crate::program::Dispatch;
use crate::protocol::{FRESH, OpKind};

/// Evaluations dispatched before the timed ones, each waited for, at least this many and for at
/// least `WARM_TIME`: the pipeline and caches settle, and a mobile GPU's clock rises to what
/// sustained work gets (the tablet's ran short scenes several times slower without).
const WARMUP: usize = 3;
const WARM_TIME: std::time::Duration = std::time::Duration::from_millis(250);
/// Timed evaluations at most (the GPU clock's slots).
const MOST: usize = 16;

/// What evaluating a square of output pixels took.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timed {
    /// GPU time of each evaluation, ns, fastest first.
    pub times: Vec<f64>,
    /// Measured by GPU timestamps; else by the wall time of each submission waited for.
    pub stamped: bool,
    pub tiles: usize,
    /// Op entries the tiles' lists hold, which every pixel of a tile walks.
    pub entries: usize,
    pub listed: Listed,
    /// What the frame budget's table (`cost.rs`) estimates the tiles cost one after another, ns.
    pub estimate: f64,
}

impl Timed {
    /// The middle time, ns.
    #[must_use]
    pub fn median(&self) -> f64 {
        self.times.get(self.times.len() / 2).copied().unwrap_or(0.0)
    }
}

/// The entries of a plan by what the evaluator does for them (`protocol_cost.rs`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Listed {
    /// Continuous spans, and the capsules they flatten into, summed.
    pub spans: usize,
    pub pieces: f64,
    /// Stamped spans, the stamps a pixel tests for them summed, and how many are textures.
    pub stamped: usize,
    pub stamps: f64,
    pub textured: usize,
    pub fills: usize,
    pub patches: usize,
    pub smudges: usize,
    pub filters: usize,
    /// Ends, frames, moves, masks, adjustments.
    pub others: usize,
}

impl Listed {
    fn add(&mut self, kind: OpKind) {
        match kind {
            OpKind::Span { pieces } => {
                self.spans += 1;
                self.pieces += f64::from(pieces);
            }
            OpKind::Stamped { stamps, texture } => {
                self.stamped += 1;
                self.stamps += f64::from(stamps);
                self.textured += usize::from(texture);
            }
            OpKind::Fill => self.fills += 1,
            OpKind::Patch => self.patches += 1,
            OpKind::Smudge => self.smudges += 1,
            OpKind::Filter => self.filters += 1,
            OpKind::Other => self.others += 1,
        }
    }
}

impl Core {
    /// Evaluates `artwork` on white paper over the square of `side` output pixels whose top-left
    /// lies at canvas pixel `at`, at `scale` output pixels per canvas pixel, once to build its
    /// scene, smudge residuals and filter results and bin its plan; then that plan `repeats`
    /// times (at most 16) after a few untimed ones, each timed on the GPU.
    ///
    /// # Errors
    /// `Invalid` for a scale other than 1, 2, 4 or 8; as the evaluator otherwise.
    pub fn time_evaluation(
        &self,
        artwork: &Artwork,
        (scale, at): (u32, [u32; 2]),
        side: u32,
        repeats: usize,
    ) -> Result<Timed, RenderError> {
        let output = Output::at(scale, at.map(|v| v * scale))?;
        let mut target = CanvasTarget::exact(self, [side, side])?;
        let request = FrameRequest {
            artwork,
            active: None,
            background: Color::WHITE,
        };
        RegionEvaluator::new(self, &mut target).evaluate_piece(
            &request,
            output,
            PixelRect::full([side, side]),
        )?;
        let mut timed = target.listing()?;
        let warming = Instant::now();
        let mut warmed = 0;
        while warmed < WARMUP || warming.elapsed() < WARM_TIME {
            target.redispatch(self, output, None)?;
            self.wait_idle()?;
            warmed += 1;
        }
        let repeats = repeats.clamp(1, MOST);
        match GpuClock::new(&self.gpu) {
            Some(mut clock) => {
                timed.times = target.stamped(self, output, &mut clock, repeats)?;
                timed.stamped = true;
            }
            None => {
                for _ in 0..repeats {
                    let started = Instant::now();
                    target.redispatch(self, output, None)?;
                    self.wait_idle()?;
                    timed.times.push(started.elapsed().as_secs_f64() * 1e9);
                }
            }
        }
        timed.times.sort_by(f64::total_cmp);
        Ok(timed)
    }
}

impl CanvasTarget {
    /// What the kept plan lists, and the cost table's estimate for it.
    fn listing(&self) -> Result<Timed, RenderError> {
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let plan = (cache.kept_plan()).ok_or(RenderError::Invalid("no plan was kept"))?;
        let kinds: Vec<OpKind> = item_kinds(&cache.scene).collect();
        let mut listed = Listed::default();
        for &entry in &plan.indices {
            match kinds.get(entry as usize) {
                Some(kind) if entry < FRESH => listed.add(*kind),
                _ => listed.add(OpKind::Other),
            }
        }
        Ok(Timed {
            tiles: plan.tiles.len(),
            entries: plan.indices.len(),
            listed,
            estimate: (plan.tiles.iter())
                .map(|tile| tile_cost(plan, tile, &cache.costs))
                .sum(),
            ..Timed::default()
        })
    }

    /// `repeats` dispatches of the kept plan, each timed by `clock`: their GPU times, ns.
    fn stamped(
        &mut self,
        core: &Core,
        output: Output,
        clock: &mut GpuClock,
        repeats: usize,
    ) -> Result<Vec<f64>, RenderError> {
        for _ in 0..repeats {
            let timing = clock
                .start(Group::default())
                .ok_or(RenderError::Invalid("no free timestamp slot"))?;
            self.redispatch(core, output, Some((clock, timing)))?;
            clock.finish(&core.gpu, timing);
        }
        let mut times = Vec::new();
        // Each reading comes back once the GPU is done and its buffer mapped.
        for _ in 0..100 {
            core.wait_idle()?;
            times.extend(clock.readings(&core.gpu).into_iter().map(|(_, ns)| ns));
            if times.len() >= repeats {
                break;
            }
        }
        Ok(times)
    }

    /// The kept plan dispatched again, its evaluation timed by `timing`.
    fn redispatch(
        &mut self,
        core: &Core,
        output: Output,
        timing: Option<(&GpuClock, Timing)>,
    ) -> Result<(), RenderError> {
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let plan = (cache.kept_plan()).ok_or(RenderError::Invalid("no plan was kept"))?;
        core.evaluate.dispatch(
            &core.gpu,
            &mut self.bindings,
            &Dispatch {
                ops: &cache.ops,
                fresh: &[],
                plan,
                only: None,
                output,
                background: [1.0; 4],
                reset: false,
                residuals: self.smudges.residuals(),
                base: Affine::IDENTITY,
                snapshot: None,
                timing,
            },
        )
    }
}
