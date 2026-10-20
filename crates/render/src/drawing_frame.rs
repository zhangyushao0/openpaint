//! One frame of the stroke being drawn on the raster core (S63, DECISIONS D90; `drawing.rs` holds
//! the slots): its new spans laid into the kept coverage of the output tiles they reach, then the
//! regions the frame changes (the spans settled since, its tail now and last frame, its mirrored
//! copies, a dropped stroke's pixels) drawn from the snapshots of what lies under the stroke, the
//! stroke from its kept coverage and tail, and the units above it (`drawing_parts.rs`), all one
//! batch, one submission. The frame after the lift draws the committed stroke (its end tapered,
//! D71) where it changed, from the same snapshots: so every frame and the lift equal the canvas
//! drawn cold, and nothing is drawn again after the lift.

use std::ops::Range;
use std::sync::Arc;

use openpaint_core::{ActiveStroke, StrokeTool};

use super::CanvasTarget;
use crate::core::Core;
use crate::drawing::Drawing;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::{Bounds, PixelRect};
use crate::output::Output;
use crate::protocol::Op;
use crate::raster::{Finish, Region, SceneBuffers};
use crate::raster_plan::{
    Coverage, InProgress, KeptTile, Pass, RasterPlan, Restored, SlotAt, StrokeDrawn,
};
use crate::scene::{ActivePlace, Fresh, Scene};
use crate::stroke::stroke_region;
use crate::view_tiles::TIER_TILE;

/// Where a drawing's regions go: the output's scale, the texture holding it (its view and
/// format), where an output tile's pixels lie in it (`None`: the tile is not drawn now), and
/// whether its tiles lie there side by side at their own pixels (the canvas: a region may span
/// tiles), or each in a slot of its own (a view tier's atlas).
pub(crate) struct Onto<'a> {
    pub(crate) scale: u32,
    pub(crate) view: &'a wgpu::TextureView,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) place: &'a dyn Fn([u32; 2]) -> Option<[u32; 2]>,
    pub(crate) joined: bool,
}

/// What one frame of a drawing drew: its regions (output pixels) and the output tiles it drew and
/// passed over.
#[derive(Debug, Default)]
pub(crate) struct StrokeWork {
    pub(crate) parts: Vec<PixelRect>,
    pub(crate) drawn: Vec<[u32; 2]>,
    pub(crate) skipped: Vec<[u32; 2]>,
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
        let dirty: Option<Vec<PixelRect>> = dirty.map(|rects| {
            let output = Output::at(scale, [0, 0]).unwrap_or(Output::CANVAS);
            rects.iter().map(|rect| output.place(rect)).collect()
        });
        drawing.follow(
            request.artwork,
            (place.layer, place.on_mask),
            at,
            dirty.as_deref(),
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
            joined: true,
        };
        let work = self.draw_stroke(core, request, &mut drawing, (&onto, &moved));
        self.drawing = drawing;
        self.moved = moved;
        let parts = work?.parts;
        self.raster.stale(&parts);
        Ok(parts)
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
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene"))?;
        let scene = Arc::clone(&cache.scene);
        let drawn = scene.has_active();
        let step = drawing
            .track
            .advance(request.artwork, request.active, drawn);
        let mut batch = Vec::new();
        let Some(active) = request.active.filter(|_| drawn) else {
            let changed: Vec<PixelRect> = step.redraw.into_iter().chain(moved.to_vec()).collect();
            let work = self.draw_parts(
                core,
                request,
                drawing,
                (onto, &scene),
                (&changed, None),
                &mut batch,
            );
            let drawn = work.and_then(|work| self.draw_batch(core, drawing, &batch).map(|()| work));
            drawing.end_stroke();
            return drawn;
        };
        if step.reset {
            drawing.end_stroke();
        }
        let pen = self.pen(core, (active, step.kept.clone()), &scene)?;
        self.note_kept(core, drawing, (&pen, onto))?;
        batch.extend(zeroed(drawing.take_taken()));
        let changed: Vec<PixelRect> = (step.fresh_region.into_iter())
            .chain(step.redraw)
            .chain(moved.iter().copied())
            .collect();
        let reach = (onto, &*scene);
        let changes = (&changed[..], Some(&pen));
        let work = self.draw_parts(core, request, drawing, reach, changes, &mut batch)?;
        self.draw_batch(core, drawing, &batch)?;
        Ok(work)
    }

    /// The stroke's ops this frame written behind the scene's (its paint op, the spans settled
    /// since `kept.start`, its tail), and how they lay.
    fn pen(
        &self,
        core: &Core,
        (active, kept): (&ActiveStroke, Range<usize>),
        scene: &Scene,
    ) -> Result<Pen, RenderError> {
        let cache = self
            .scene
            .as_ref()
            .ok_or(RenderError::Invalid("no scene"))?;
        let fresh = Fresh::new(active, kept.start, scene.active_clip);
        cache.ops.with_fresh(&core.gpu, &fresh.ops);
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
            pen.coverage = match (op.is_stamped(), op.covers()) {
                (true, true) => Coverage::StampCover,
                (true, false) => Coverage::StampDensity,
                (false, true) => Coverage::Cover,
                (false, false) => Coverage::Density,
            };
        }
        pen.contact = pen.coverage == Coverage::Density && paint.1.on_paper();
        Ok(pen)
    }

    /// The frame's `batch` encoded with the frame loop's targets into one command buffer and
    /// submitted.
    fn draw_batch(
        &mut self,
        core: &Core,
        drawing: &Drawing,
        batch: &[Part<'_>],
    ) -> Result<(), RenderError> {
        if batch.is_empty() {
            return Ok(());
        }
        let missing = || RenderError::Invalid("no scene was built");
        let held = self.held().ok_or_else(missing)?;
        let cache = self.scene.as_ref().ok_or_else(missing)?;
        let buffers = SceneBuffers::of(&cache.ops, held);
        let targets = self.raster.targets.as_mut().ok_or_else(missing)?;
        let regions: Vec<Region<'_>> = (batch.iter())
            .map(|part| Region {
                plan: &part.plan,
                output: part.output,
                paper: part.paper,
                finish: part.finish.as_ref(),
            })
            .collect();
        let buffer = targets.prepare_all(core, &buffers, (&regions, drawing.pages()), None)?;
        core.gpu.submit([buffer]);
        let drawn = batch.iter().filter(|part| part.finish.is_some()).count();
        core.tally.rastered(drawn);
        self.raster.drew = true;
        Ok(())
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

/// The output tiles meeting output pixels `rect` inside an output of `size` pixels, each with that
/// part of `rect`.
fn meeting(rect: PixelRect, size: [u32; 2]) -> impl Iterator<Item = ([u32; 2], PixelRect)> {
    let rect = rect.intersect(&PixelRect::full(size));
    let side = TIER_TILE as i32;
    let span = |axis: usize| {
        if rect.is_empty() {
            return 0..0;
        }
        rect.min[axis] / side..(rect.max[axis] + side - 1) / side
    };
    let (columns, rows) = (span(0), span(1));
    rows.flat_map(move |row| columns.clone().map(move |column| [column, row]))
        .map(move |[column, row]| {
            let min = [column * side, row * side];
            let tile = PixelRect {
                min,
                max: [min[0] + side, min[1] + side],
            };
            ([column, row].map(i32::unsigned_abs), rect.intersect(&tile))
        })
}
