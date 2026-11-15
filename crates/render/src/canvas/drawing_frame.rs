//! One frame of the stroke being drawn on the raster core (`drawing.rs` holds the slots): its new
//! spans laid into the kept coverage of the output tiles they reach, then the regions the frame
//! changes (the spans settled since, its tail now and last frame, its mirrored copies, a dropped
//! stroke's pixels) drawn from the snapshots of what lies under the stroke, the stroke from its
//! kept coverage and tail, and the units above it (`drawing_parts.rs`), all one batch, one
//! submission. The frame after the lift draws the committed stroke (its end tapered) where it
//! changed, from the same snapshots: so every frame and the lift equal the canvas
//! drawn cold, and nothing is drawn again after the lift. While a view tier shows, its drawing
//! draws the stroke's frames and the canvas only those the tier cannot draw at every tile they
//! reach (`stroke_left`), from where it last drew; its lift's frame draws on the canvas all it
//! lacks (`ActiveTrack::pass`), so the canvas equals it drawn cold from the lift on.

use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use openpaint_core::{ActiveStroke, StrokeTool};

use super::CanvasTarget;
use super::Drawing;
use crate::canvas::NO_SCENE;
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::{Bounds, PixelRect};
use crate::output::Output;
use crate::profile::{StrokeDrew, StrokeParts, canvas_drew, since};
use crate::protocol::Op;
use crate::raster::{Finish, Region, SceneBuffers};
use crate::raster_lay::coverage;
use crate::raster_plan::{
    Coverage, InProgress, KeptTile, Pass, RasterPlan, Restored, SlotAt, StrokeDrawn,
};
use crate::scene::{ActivePlace, Fresh, Scene};
use crate::stroke::stroke_region;
use crate::view_tiles::TIER_TILE;

/// Where a drawing's regions go: the output's scale, the texture holding it (its view and
/// format), and where an output tile's pixels lie in it (`None`: the tile is not drawn now): side
/// by side at their own pixels (the canvas), or each in a slot of its own (a view tier's atlas).
pub(crate) struct Onto<'a> {
    pub(crate) scale: u32,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) place: &'a dyn Fn([u32; 2]) -> Option<[u32; 2]>,
}

/// What one frame of a drawing drew: its regions (output pixels) and the output tiles it drew and
/// passed over; the snapshot parts it took, the units its regions laid again over snapshots that
/// lagged them and where its time went (for the pen latency log, `profile.rs`).
#[derive(Debug, Default)]
pub(crate) struct StrokeWork {
    pub(crate) parts: Vec<PixelRect>,
    pub(crate) drawn: Vec<[u32; 2]>,
    pub(crate) skipped: Vec<[u32; 2]>,
    pub(crate) snapshots: usize,
    pub(crate) laid_again: usize,
    pub(crate) spent: StrokeParts,
}

impl StrokeWork {
    /// What it drew, counted.
    pub(crate) fn drew(&self) -> StrokeDrew {
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        StrokeDrew {
            regions: count(self.parts.len()),
            snapshots: count(self.snapshots),
            laid_again: count(self.laid_again),
            parts: self.spent,
        }
    }
}

/// One region of a frame's batch: its plan, where its output lies, its paper, its finish.
pub(super) struct Part<'a> {
    pub(super) plan: RasterPlan,
    pub(super) output: Output,
    pub(super) paper: [f32; 4],
    pub(super) finish: Option<Finish<'a>>,
}

/// The stroke as one frame draws it: its ops' places in the scene's op buffer, the spans settled
/// since and its tail (each its op's place, the op, its bounds), how its spans lay; or a smudge's
/// op and the canvas pixels it can change.
pub(super) struct Pen {
    smudge: Option<(u32, PixelRect)>,
    paint: (u32, Op),
    settled: Vec<(u32, Op, Bounds)>,
    tail: Vec<(u32, Op, Bounds)>,
    coverage: Coverage,
    contact: bool,
}

impl Pen {
    /// What a region meeting output tiles `tiles` lays of it: its kept coverage there.
    pub(super) fn in_progress(&self, drawing: &Drawing, tiles: &[[u32; 2]]) -> InProgress {
        if let Some((op, reach)) = self.smudge {
            return InProgress::Smudge { op, reach };
        }
        let kept = (tiles.iter())
            .filter_map(|tile| {
                let kept = drawing.kept.get(tile)?;
                Some(KeptTile {
                    slots: kept.slots,
                    origin: tile.map(|at| at * TIER_TILE),
                    held: kept.held,
                    laid: kept.laid,
                })
            })
            .collect();
        InProgress::Stroke(StrokeDrawn {
            paint: self.paint,
            settled: self.settled.clone(),
            tail: self.tail.clone(),
            coverage: self.coverage,
            contact: self.contact,
            kept,
        })
    }
}

impl CanvasTarget {
    /// `drawing` (of an output at `scale`) follows `request`'s artwork, the scene the canvas
    /// built for it naming the unit the next stroke lays at; `dirty`, the canvas pixels a change
    /// reached (`None`: unknown).
    pub(crate) fn follow(
        &self,
        drawing: &mut Drawing,
        request: &FrameRequest<'_>,
        (dirty, scale): (Option<&[PixelRect]>, u32),
    ) {
        let place = ActivePlace::of_request(request.artwork, request.active);
        let at = (self.scene.as_ref())
            .filter(|cache| cache.artwork.same_version(request.artwork) && cache.active == place)
            .and_then(|cache| {
                let at = cache.scene.raster.active()?;
                Some((at, Restored::at(&cache.scene, (at, place.on_mask))))
            });
        let output = Output::at(scale, [0, 0]).unwrap_or(Output::CANVAS);
        drawing.follow(
            request.artwork,
            (place.layer, place.on_mask),
            at,
            (dirty, output),
        );
    }

    /// One frame of the canvas's own drawing (the 1x canvas, its tiles side by side): the stroke
    /// being drawn, or the frame after its lift or drop, with the regions `moved` holds; the
    /// canvas pixels drawn (the raster worker's tiles they reach drawn again once answered).
    pub(super) fn stroke_frame(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<Vec<PixelRect>, RenderError> {
        // It draws everything since the canvas last drew the stroke.
        self.behind = false;
        let mut drawing = std::mem::take(&mut self.drawing);
        let moved = std::mem::take(&mut self.moved);
        let (view, format, size) = (self.raster.view.clone(), self.texture.format(), self.size());
        let place = move |tile: [u32; 2]| {
            let at = tile.map(|at| at * TIER_TILE);
            (at[0] < size[0] && at[1] < size[1]).then_some(at)
        };
        let onto = Onto {
            scale: 1,
            view: &view,
            format,
            place: &place,
        };
        let work = self.draw_stroke(core, request, &mut drawing, (&onto, &moved));
        self.drawing = drawing;
        self.moved = moved;
        let work = work?;
        canvas_drew(work.drew());
        let parts = work.parts;
        self.raster.stale(&parts);
        Ok(parts)
    }

    /// After the view tier shown drew this frame of the stroke (`covered`: at every tile it
    /// reached): the frame the canvas left to it (`drawn`) drawn now when the tier could not,
    /// with everything the canvas lacks of the stroke since it last drew it; the canvas pixels
    /// drawn, which the other tiers take as any change. A frame it covered the canvas takes at
    /// the lift, or at the next frame it draws (a read's among them).
    ///
    /// # Errors
    /// As the raster core.
    pub(crate) fn stroke_left(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        covered: bool,
    ) -> Result<Vec<PixelRect>, RenderError> {
        if !std::mem::take(&mut self.left) || covered {
            return Ok(Vec::new());
        }
        // The canvas's drawing, as `canvas()` times the rest of it.
        let started = std::time::Instant::now();
        let drawn = self.stroke_frame(core, request);
        crate::profile::drawn(started.elapsed());
        drawn
    }

    /// One frame of `drawing` onto `onto` for `request`, whose scene the canvas built this frame
    /// and the drawing followed: the stroke being drawn where it changed (with `moved`, the
    /// canvas pixels the version under it changed: its mirrored copies), or the frame after its
    /// lift or drop.
    pub(crate) fn draw_stroke(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        drawing: &mut Drawing,
        (onto, moved): (&Onto<'_>, &[PixelRect]),
    ) -> Result<StrokeWork, RenderError> {
        let cache = self.scene.as_ref().ok_or(NO_SCENE)?;
        let scene = Arc::clone(&cache.scene);
        let drawn = scene.has_active();
        let started = Instant::now();
        let step = drawing
            .track
            .advance(request.artwork, request.active, drawn);
        let mut spent = StrokeParts {
            track_ns: since(started),
            ..StrokeParts::NONE
        };
        let mut batch = Vec::new();
        let Some(active) = request.active.filter(|_| drawn) else {
            let changed: Vec<PixelRect> = step.redraw.into_iter().chain(moved.to_vec()).collect();
            let (planning, changes) = (Instant::now(), (&changed[..], None));
            let work = self.draw_parts(core, request, drawing, (onto, &scene), changes, &mut batch);
            spent.plan_ns = since(planning);
            let drawn = work.and_then(|work| {
                let spent = self.draw_batch(core, drawing, &batch, spent)?;
                Ok(StrokeWork { spent, ..work })
            });
            drawing.end_stroke();
            return drawn;
        };
        if step.reset {
            drawing.end_stroke();
        }
        let pen = self.pen(core, (active, step.kept.clone()), &scene, &mut spent)?;
        let planning = Instant::now();
        self.note_kept(core, drawing, (&pen, onto))?;
        batch.extend(zeroed(drawing.take_taken()));
        let changed: Vec<PixelRect> = (step.fresh_region.into_iter())
            .chain(step.redraw)
            .chain(moved.iter().copied())
            .collect();
        let reach = (onto, &*scene);
        let changes = (&changed[..], Some(&pen));
        let work = self.draw_parts(core, request, drawing, reach, changes, &mut batch)?;
        spent.plan_ns = since(planning);
        let spent = self.draw_batch(core, drawing, &batch, spent)?;
        Ok(StrokeWork { spent, ..work })
    }

    /// The stroke's ops this frame written behind the scene's (its paint op, the spans settled
    /// since `kept.start`, its tail), and how they lay; their making and writing timed into
    /// `spent`.
    fn pen(
        &self,
        core: &Core,
        (active, kept): (&ActiveStroke, Range<usize>),
        scene: &Scene,
        spent: &mut StrokeParts,
    ) -> Result<Pen, RenderError> {
        let cache = self.scene.as_ref().ok_or(NO_SCENE)?;
        let started = Instant::now();
        let fresh = Fresh::new(active, kept.start, scene.active_clip);
        spent.ops_ns = since(started);
        let started = Instant::now();
        cache.ops.with_fresh(&core.gpu, &fresh.ops);
        spent.writes_ns = since(started);
        let first = cache.ops.count();
        let paint = (first, fresh.ops[0]);
        let mut pen = Pen {
            smudge: None,
            paint,
            settled: Vec::new(),
            tail: Vec::new(),
            coverage: Coverage::Cover,
            contact: false,
        };
        if active.style().tool() == StrokeTool::Smudge {
            let reach = stroke_region(active.style(), active.samples(), 0);
            pen.smudge = Some((first, reach.unwrap_or(PixelRect::full([0, 0]))));
            return Ok(pen);
        }
        let spans: Vec<(u32, Op, Bounds)> = (1..)
            .zip(&fresh.ops[1..])
            .zip(&fresh.bounds)
            .map(|((at, op), bounds)| (first + at, *op, *bounds))
            .collect();
        let settled = kept.len().min(spans.len());
        pen.settled = spans[..settled].to_vec();
        pen.tail = spans[settled..].to_vec();
        if let Some((_, op, _)) = spans.first() {
            pen.coverage = coverage(op);
        }
        pen.contact = pen.coverage == Coverage::Density && paint.1.on_paper();
        Ok(pen)
    }

    /// The frame's `batch` encoded with the frame loop's targets into one command buffer and
    /// submitted; `spent` with its encoding, its buffer writes and its submission.
    fn draw_batch(
        &mut self,
        core: &Core,
        drawing: &Drawing,
        batch: &[Part<'_>],
        mut spent: StrokeParts,
    ) -> Result<StrokeParts, RenderError> {
        if batch.is_empty() {
            return Ok(spent);
        }
        let held = self.held().ok_or(NO_SCENE)?;
        let cache = self.scene.as_ref().ok_or(NO_SCENE)?;
        let buffers = SceneBuffers::of(&cache.ops, held);
        let targets = self.raster.targets.as_mut().ok_or(NO_SCENE)?;
        let regions: Vec<Region<'_>> = (batch.iter())
            .map(|part| Region {
                plan: &part.plan,
                output: part.output,
                paper: part.paper,
                finish: part.finish.as_ref(),
            })
            .collect();
        let started = Instant::now();
        let buffer = targets.prepare_all(core, &buffers, (&regions, drawing.pages()), None)?;
        let (prepared, uploaded) = (since(started), targets.uploaded_ns);
        spent.encode_ns = prepared.saturating_sub(uploaded);
        spent.writes_ns += uploaded;
        let started = Instant::now();
        core.gpu.submit([buffer]);
        spent.submit_ns = since(started);
        let drawn = batch.iter().filter(|part| part.finish.is_some()).count();
        core.tally.rastered(drawn);
        self.raster.drew = true;
        Ok(spent)
    }
}

#[path = "drawing_kept.rs"]
mod kept;
#[path = "drawing_parts.rs"]
mod parts;

/// The batch's first part when kept coverage slots were taken this frame (`taken`): they are set
/// to nothing first.
fn zeroed<'a>(taken: Vec<SlotAt>) -> Option<Part<'a>> {
    (!taken.is_empty()).then(|| Part {
        plan: RasterPlan {
            size: [1, 1],
            passes: vec![Pass::Zero(taken)],
            ..RasterPlan::default()
        },
        output: Output::CANVAS,
        paper: [0.0; 4],
        finish: None,
    })
}
