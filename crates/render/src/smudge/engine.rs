//! `Smudges`: a canvas target's smudge residuals. Finished ones live in its arena, keyed by
//! stroke; the in-progress one in a residual of its own that grows by the contacts of the spans
//! each evaluation settles (S34: the last two follow at the commit) and, once committed, moves
//! into the arena as it is. A residual that is missing is evaluated cold from its stroke and the
//! scene, in layer order, so the layer pixels it carries already include the smudges before it.

use std::ops::Range;

use openpaint_core::{ActiveStroke, Artwork, LayerId, StrokeSample, StrokeStyle};

use super::arena::{Arena, Keep, Key, TileSet, key};
use super::dabs::{Dab, DabWalker, dabs, read_tiles};
use super::kernels::{Job, KernelBuffers, tile_job};
use super::material::Material;
use super::residual::{Working, node_words, words_buffer};
use crate::affine::Affine;
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::program::SceneOps;
use crate::protocol::encode;
use crate::scene::{Scene, SmudgeItem};
use crate::stroke::stroke_region;

/// A residual being evaluated and which evaluation of the material it needs.
struct Run {
    /// Names the material its tiles were evaluated for.
    owner: u64,
    working: Working,
    /// The kernels' bindings, for the working buffer they were built with.
    group: Option<(wgpu::Buffer, wgpu::BindGroup)>,
}

/// The in-progress smudge: its stroke as far as it was walked, over the version it was drawn on.
struct Active {
    layer: LayerId,
    /// It smudges the layer's mask.
    on_mask: bool,
    base: Artwork,
    style: StrokeStyle,
    samples: Vec<StrokeSample>,
    walker: DabWalker,
    run: Run,
}

/// The material texture, the kernels' buffers and an empty residual, made on first use.
struct Tools {
    material: Option<Material>,
    kernels: Option<KernelBuffers>,
    empty: wgpu::Buffer,
    /// The run whose material the texture holds, and the next run's name.
    owner: u64,
    next: u64,
    /// The last cold evaluation's residual, emptied for the next (S51): its buffers are made
    /// once, not per residual.
    scratch: Option<Run>,
}

pub(crate) struct Smudges {
    arena: Arena,
    active: Option<Active>,
    tools: Tools,
    /// Bytes the arena keeps within when it can (S51): the GPU memory budget's share for the
    /// screen's canvas, else no more than the device binds.
    room: u64,
    /// The most the arena binds, below the device's limit (tests: a small device's limit).
    limit: u64,
    /// A budget sets the room each frame (`set_room`), so what a frame's evaluations needed is
    /// kept together; without one, each evaluation counts as a frame of its own.
    framed: bool,
}

impl Smudges {
    pub(crate) fn new(core: &Core) -> Self {
        Self {
            arena: Arena::new(&core.gpu),
            room: u64::MAX,
            limit: u64::MAX,
            framed: false,
            active: None,
            tools: Tools {
                material: None,
                kernels: None,
                empty: words_buffer(&core.gpu, 4),
                owner: 0,
                next: 1,
                scratch: None,
            },
        }
    }

    /// The arena and the in-progress residual (an empty one without), for the evaluator.
    pub(crate) fn residuals(&self) -> [&wgpu::Buffer; 2] {
        let active = self.active.as_ref();
        let working = active.map_or(&self.tools.empty, |active| active.run.working.buffer());
        [self.arena.buffer(), working]
    }

    /// Forgets the in-progress smudge: none is drawn now.
    pub(crate) fn end_active(&mut self) {
        self.active = None;
    }

    /// Where each block of `keys` starts (0 while missing), into the scene's base words from
    /// `at`.
    fn write_bases(&self, core: &Core, ops: &SceneOps, at: u32, keys: &[Key]) {
        let bases: Vec<u32> = keys
            .iter()
            .map(|key| u32::try_from(self.arena.base(key).unwrap_or(0)).unwrap_or(0))
            .collect();
        if !bases.is_empty() {
            let at = u64::from(at) * 4;
            core.gpu.queue.write_buffer(&ops.masks, at, &encode(&bases));
        }
    }

    /// Extends the in-progress smudge `stroke` drawn over `artwork` by its new samples' contacts.
    /// Returns the pixels to evaluate again when its residual started over (a new stroke, or
    /// the version under it changed).
    ///
    /// # Errors
    /// As `prepare`.
    pub(crate) fn advance(
        &mut self,
        core: &Core,
        (scene, ops): (&Scene, &SceneOps),
        artwork: &Artwork,
        stroke: &ActiveStroke,
    ) -> Result<Option<PixelRect>, RenderError> {
        let Some(range) = scene.active_before.clone() else {
            self.active = None;
            return Ok(None);
        };
        let continuing = self
            .active
            .as_ref()
            .is_some_and(|active| active.continues(artwork, stroke));
        let mut restarted = None;
        if !continuing {
            let size = [artwork.canvas().width(), artwork.canvas().height()];
            self.active = Some(Active {
                layer: stroke.layer(),
                on_mask: stroke.on_mask(),
                base: artwork.clone(),
                style: stroke.style().clone(),
                samples: Vec::new(),
                walker: DabWalker::default(),
                run: self.tools.run(core, size),
            });
            restarted = stroke_region(stroke.style(), stroke.samples(), 0);
        }
        let Some(active) = self.active.as_mut() else {
            return Ok(None);
        };
        let samples = stroke.samples();
        let mut new = Vec::new();
        active
            .walker
            .advance(&active.style, (samples, false), &mut new);
        let done = active.samples.len();
        active.samples.extend_from_slice(&samples[done..]);
        // The in-progress stroke lies above every transform: on the canvas itself.
        self.tools.extend(
            core,
            (scene, ops, (range, Affine::IDENTITY)),
            &self.arena,
            &mut active.run,
            &new,
        )?;
        Ok(restarted)
    }
}

impl Active {
    /// True when `stroke` is this one with samples added, over the same version.
    fn continues(&self, artwork: &Artwork, stroke: &ActiveStroke) -> bool {
        self.layer == stroke.layer()
            && self.on_mask == stroke.on_mask()
            && self.style == *stroke.style()
            && stroke.samples().starts_with(&self.samples)
            && self.base.same_version(artwork)
    }

    /// True when `item` of `artwork` is this stroke committed onto the layer (or mask) it was
    /// drawn on: its newest stroke, with everything before it as it was under the residual.
    fn adopts(&self, item: &SmudgeItem, artwork: &Artwork) -> bool {
        let stroke = &item.stroke;
        if item.layer != self.layer
            || item.on_mask != self.on_mask
            || !item.newest
            || *stroke.style() != self.style
            || !stroke.samples().starts_with(&self.samples)
        {
            return false;
        }
        let content = |artwork: &'_ Artwork| {
            let layer = artwork.layer(self.layer)?;
            match (self.on_mask, layer.mask()) {
                (false, _) => Some((layer.strokes().clone(), layer.edits().to_vec())),
                (true, Some(mask)) => Some((mask.strokes().clone(), mask.edits().to_vec())),
                (true, None) => None,
            }
        };
        let (Some((old, old_edits)), Some((new, new_edits))) =
            (content(&self.base), content(artwork))
        else {
            return false;
        };
        let count = old.len();
        let strokes = new.len() == count + 1
            && old.last().is_none_or(|last| {
                new.get(count - 1)
                    .is_some_and(|same| same.shares_samples(last))
            });
        let edits = old_edits.len() == new_edits.len()
            && old_edits.iter().zip(&new_edits).all(|(a, b)| a.same(b));
        strokes && edits
    }
}

impl Tools {
    /// An empty residual under a new name.
    fn run(&mut self, core: &Core, size: [u32; 2]) -> Run {
        self.next += 1;
        Run {
            owner: self.next,
            working: Working::new(&core.gpu, size),
            group: None,
        }
    }

    /// An empty residual under a new name for a cold evaluation: the scratch one emptied, when
    /// there is one of `size`.
    fn cold_run(&mut self, core: &Core, size: [u32; 2]) -> Run {
        match self.scratch.take().filter(|run| run.working.size() == size) {
            Some(mut run) => {
                self.next += 1;
                run.owner = self.next;
                run.working.reset(&core.gpu);
                // Its kernels bind the material made now, which may be another texture.
                run.group = None;
                run
            }
            None => self.run(core, size),
        }
    }

    /// Applies `dabs` to `run`: allocates the tiles they read, evaluates the material of the
    /// new ones (all of the run's, when the texture holds another run's), resets the new tiles,
    /// then runs the contacts in order. `range` is the scene's items before the smudge and
    /// where its pixels lie on the canvas.
    fn extend(
        &mut self,
        core: &Core,
        (scene, ops, range): (&Scene, &SceneOps, (Range<usize>, Affine)),
        arena: &Arena,
        run: &mut Run,
        dabs: &[Dab],
    ) -> Result<(), RenderError> {
        if dabs.is_empty() {
            return Ok(());
        }
        let size = run.working.size();
        let new = run
            .working
            .allocate(&core.gpu, &read_tiles(dabs, size), binding_limit(core))?;
        if self.material.is_none() {
            self.material = Some(Material::new(core, size)?);
        }
        let Some(material) = self.material.as_mut() else {
            return Ok(());
        };
        let stale = self.owner != run.owner;
        let tiles = if stale { run.working.tiles() } else { &new[..] };
        let rects: Vec<PixelRect> = tiles.iter().map(|t| run.working.tile_rect(*t)).collect();
        material.evaluate(
            core,
            (scene, ops),
            range,
            &rects,
            [arena.buffer(), &self.empty],
        )?;
        self.owner = run.owner;
        let kernels = self
            .kernels
            .get_or_insert_with(|| KernelBuffers::new(&core.gpu));
        let buffer = run.working.buffer();
        if run.group.as_ref().is_none_or(|(built, _)| built != buffer) {
            let group = core
                .smudge
                .group(&core.gpu, kernels, buffer, material.view());
            run.group = Some((buffer.clone(), group));
        }
        let mut jobs: Vec<Job> = new.iter().map(|tile| tile_job(*tile, size)).collect();
        jobs.extend(
            dabs.iter()
                .filter_map(|dab| dab.rect(size).map(|rect| Job::Contact(*dab, rect))),
        );
        if let Some((_, group)) = &run.group {
            core.smudge.run(&core.gpu, (kernels, group), &jobs, size);
        }
        Ok(())
    }
}

#[path = "engine_budget.rs"]
mod budget;

#[path = "engine_needs.rs"]
mod needs;
pub(crate) use needs::{Wanted, reached};

#[path = "engine_prepare.rs"]
mod prepare;

/// The largest buffer the device binds, in bytes.
fn binding_limit(core: &Core) -> u64 {
    let limits = core.gpu.device.limits();
    limits
        .max_storage_buffer_binding_size
        .min(limits.max_buffer_size)
}

/// The material texture in `slot`, for a canvas of `size` pixels, made on first use.
fn material<'a>(
    slot: &'a mut Option<Material>,
    core: &Core,
    size: [u32; 2],
) -> Result<&'a mut Material, RenderError> {
    if slot.is_none() {
        *slot = Some(Material::new(core, size)?);
    }
    slot.as_mut()
        .ok_or(RenderError::Invalid("no material texture"))
}
