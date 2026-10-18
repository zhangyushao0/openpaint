//! The screen canvas drawn by the raster core (S62, DECISIONS D89; every scene since S65, D92;
//! `canvas_raster.rs` holds what waits): its changes drawn at once when small (`draw_now`), else
//! as the worker's tiles (`canvas_raster_hand.rs`); the stroke being drawn and its lift at once
//! where they change the canvas (S63, `drawing_frame.rs`).

use super::{CanvasTarget, RegionEvaluator};
use crate::core::Core;
use crate::error::RenderError;
use crate::frame::FrameRequest;
use crate::geom::PixelRect;
use crate::output::Output;
use crate::raster::{Finish, RasterTargets, SceneBuffers};
use crate::raster_plan::RasterPlan;
use crate::readback::wait;
use crate::scene::ActivePlace;
use crate::smudge::{Held, around_output};
use crate::view_fill::EDIT_PIXELS;
use crate::view_tiles::TIER_TILE;

/// A region's plan, the output it lies at, and where it goes in the destination.
pub(crate) type Planned = (RasterPlan, Output, [u32; 2]);

impl RegionEvaluator<'_> {
    /// `request` into the canvas (`evaluate`, `evaluate_frame`): the stroke being drawn (or just
    /// lifted or dropped) drawn at once where it changed, by the raster core from its kept
    /// coverage over its snapshots (S63, `drawing_frame.rs`), with the regions passed meanwhile
    /// (its mirrored copies, a smudge started over); with none, the regions passed are drawn or
    /// wait (`frame`: one of the screen's). Returns the canvas pixels drawn now.
    pub(super) fn drawn(
        &mut self,
        request: &FrameRequest<'_>,
        regions: &[PixelRect],
        frame: bool,
    ) -> Result<Vec<PixelRect>, RenderError> {
        let core = self.core;
        let place = ActivePlace::of_request(request.artwork, request.active);
        let drawn = self
            .target
            .scene(core, request.artwork, place)?
            .scene
            .has_active();
        let target = &mut *self.target;
        let stroke = request.active.is_some() && drawn;
        if target.blank {
            target.show_paper(core, request);
        }
        let restarted = target.smudged(core, request, drawn)?;
        // What a change reached: the snapshots under the next stroke there go (S63), its view
        // tiers' too.
        let dirty: Vec<PixelRect> = restarted.into_iter().chain(regions.to_vec()).collect();
        let mut drawing = std::mem::take(&mut target.drawing);
        target.follow(&mut drawing, request, (Some(&dirty), 1));
        target.drawing = drawing;
        target.dirtied(&dirty);
        target.moved.clear();
        let mut changed = Vec::new();
        if stroke || target.drawing.tracking() {
            target.moved.extend(dirty);
            changed = target.stroke_frame(core, request)?;
        } else {
            target.raster.change(regions);
        }
        if !stroke {
            changed.extend(if frame {
                target.raster_frame(core, request)?
            } else {
                target.raster_all(core, request)?
            });
        }
        Ok(changed)
    }
}

impl CanvasTarget {
    /// One frame of the screen with no stroke drawn: the worker's tiles applied, the changes
    /// drawn at once when they fit what is left of the frame, else waiting; the waiting tiles
    /// handed to the worker; residuals warm while nothing waits (S54). Returns the canvas pixels
    /// drawn now.
    fn raster_frame(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<Vec<PixelRect>, RenderError> {
        let mut changed = self.apply_answers(core);
        let fresh = std::mem::take(&mut self.raster.fresh);
        if !fresh.is_empty() {
            match self.draw_now(core, request, &fresh, true)? {
                Some(drawn) => changed.extend(drawn),
                None => self.raster.waiting.add(&fresh),
            }
        }
        self.hand_over(core, request)?;
        if !self.raster.pending() && !core.pace().spent() {
            self.warm(core, request)?;
            self.snapshots_ahead(core, request)?;
        }
        Ok(changed)
    }

    /// Everything waiting drawn now (a canvas evaluated whole, read): the tiles with the worker
    /// too, which are not applied when answered.
    fn raster_all(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
    ) -> Result<Vec<PixelRect>, RenderError> {
        let raster = &mut self.raster;
        let mut rects = std::mem::take(&mut raster.fresh);
        rects.extend(raster.waiting.take_all());
        // A stale one's pixels are drawn already, or wait among the others.
        for handed in raster.handed.iter_mut().filter(|handed| !handed.stale) {
            handed.stale = true;
            rects.push(handed.rect);
        }
        // Answered, not applied: their slots are free again.
        rects.extend(raster.done.drain(..).map(|(handed, _)| handed.rect));
        Ok(self
            .draw_now(core, request, &rects, false)?
            .unwrap_or_default())
    }

    /// Draws the canvas pixels `rects` now, on this thread, in parts each inside one of the
    /// canvas's 256-px tiles; `budgeted` (a frame's change), only when they are few, the residuals
    /// they read are at hand and their estimate fits what is left of the frame (else `None`,
    /// nothing drawn). Returns the parts drawn.
    fn draw_now(
        &mut self,
        core: &Core,
        request: &FrameRequest<'_>,
        rects: &[PixelRect],
        budgeted: bool,
    ) -> Result<Option<Vec<PixelRect>>, RenderError> {
        let parts = parts(rects, self.size());
        if parts.is_empty() {
            return Ok(Some(parts));
        }
        let near: Vec<PixelRect> = parts.iter().map(|part| around_output(part, 1)).collect();
        if budgeted {
            let area: i64 = parts.iter().map(area).sum();
            if area > EDIT_PIXELS || core.raster_made().is_none() {
                return Ok(None);
            }
            if !self.residuals_ready(core, request, &near)? {
                return Ok(None);
            }
        }
        self.reach(core, request.artwork, &near)?;
        let held = self
            .held()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let atlas = self.raster_atlas(core)?;
        let plans = self.plans(&parts, &held, atlas)?;
        if budgeted {
            let cost = (plans.iter())
                .map(|(plan, _, _)| plan.counts.estimate(plan.size))
                .sum();
            if !core.pace().fits_raster(cost) {
                return Ok(None);
            }
        }
        let into = (self.raster.view.clone(), self.texture.format());
        self.draw_planned(core, &plans, held, (&into.0, into.1, paper(request)))?;
        Ok(Some(parts))
    }

    /// The plans of the canvas pixels `parts` of the scene the last draw drew, its smudges'
    /// residuals and filters' results as `held` holds them, with an atlas of `atlas` pixels.
    fn plans(
        &self,
        parts: &[PixelRect],
        held: &Held,
        atlas: [u32; 2],
    ) -> Result<Vec<Planned>, RenderError> {
        let missing = || RenderError::Invalid("no scene was built");
        let cache = self.scene.as_ref().ok_or_else(missing)?;
        let units = (&*cache.scene, &held.bases);
        (parts.iter())
            .map(|part| {
                let at = part.min.map(i32::unsigned_abs);
                let size = [0, 1].map(|axis| (part.max[axis] - part.min[axis]).unsigned_abs());
                let output = Output::at(1, at)?;
                Ok((RasterPlan::build(units, (output, size), atlas)?, output, at))
            })
            .collect()
    }

    /// The atlas of the frame loop's raster targets, made on first use.
    pub(crate) fn raster_atlas(&mut self, core: &Core) -> Result<[u32; 2], RenderError> {
        if self.raster.targets.is_none() {
            self.raster.targets = Some(RasterTargets::sized(core, self.size())?);
        }
        Ok((self.raster.targets.as_ref()).map_or([0; 2], RasterTargets::atlas))
    }

    /// Draws `plans` of the scene the last evaluation drew (S62: the canvas's own regions, or its
    /// view tiers' changes, S59), its residuals as `held` holds them, with the frame loop's
    /// raster targets, into `into` (its format) on `paper`; each waited for when the target's
    /// dispatches are a worker's (`pace_dispatches`).
    ///
    /// # Errors
    /// `Invalid` without a scene; as the raster core.
    pub(crate) fn draw_planned(
        &mut self,
        core: &Core,
        plans: &[Planned],
        held: Held,
        (into, format, paper): (&wgpu::TextureView, wgpu::TextureFormat, [f32; 4]),
    ) -> Result<(), RenderError> {
        self.raster_atlas(core)?;
        let missing = || RenderError::Invalid("no scene was built");
        let cache = self.scene.as_ref().ok_or_else(missing)?;
        let targets = self.raster.targets.as_mut().ok_or_else(missing)?;
        let buffers = SceneBuffers::of(&cache.ops, held);
        for (plan, output, at) in plans {
            let finish = Finish {
                view: into,
                format,
                at: *at,
                only: &[],
            };
            targets.draw(core, (&buffers, plan), (*output, paper), &finish, None)?;
            if self.paced {
                wait(&core.gpu, "a worker's raster region")?;
            }
        }
        core.evaluate.tally.rastered(plans.len());
        self.raster.drew = true;
        Ok(())
    }

    /// A new canvas shows the paper where its tiles wait (S49), the raster core's way: cleared.
    fn show_paper(&mut self, core: &Core, request: &FrameRequest<'_>) {
        self.blank = false;
        let [r, g, b, a] = paper(request).map(f64::from);
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("canvas paper"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.raster.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        drop(pass);
        core.gpu.submit([encoder.finish()]);
    }
}

/// The paper of `request`, premultiplied and opaque.
pub(crate) fn paper(request: &FrameRequest<'_>) -> [f32; 4] {
    let paper = request.background;
    [paper.r(), paper.g(), paper.b(), 1.0]
}

/// `rects` cut to a canvas of `size` pixels and into parts each inside one 256-px tile.
fn parts(rects: &[PixelRect], size: [u32; 2]) -> Vec<PixelRect> {
    let whole = PixelRect::full(size);
    let side = TIER_TILE as i32;
    let mut parts = Vec::new();
    for rect in rects {
        let rect = rect.intersect(&whole);
        if rect.is_empty() {
            continue;
        }
        let [x0, y0] = rect.min.map(|at| at.div_euclid(side));
        let [x1, y1] = rect.max.map(|at| (at + side - 1).div_euclid(side));
        for y in y0..y1 {
            for x in x0..x1 {
                let tile = PixelRect {
                    min: [x * side, y * side],
                    max: [(x + 1) * side, (y + 1) * side],
                };
                parts.push(rect.intersect(&tile));
            }
        }
    }
    parts
}

fn area(rect: &PixelRect) -> i64 {
    let [width, height] = [0, 1].map(|axis| i64::from(rect.max[axis] - rect.min[axis]));
    width * height
}

#[path = "canvas_raster_hand.rs"]
mod hand;
