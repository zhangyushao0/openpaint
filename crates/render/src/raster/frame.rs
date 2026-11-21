//! A region's passes on the GPU: the plan's passes in order, each a render pass over the atlas or
//! a float target of the region (a copy for a backdrop or a group's; a copy, then a pass, for a
//! smudge, an adjustment or a filter), then the finish rounding the canvas into the destination; a
//! batch's regions one after another (`raster/batch.rs`), all in one command buffer.

use super::reads::{ATLAS, COPIED, ending};
use super::{Bound, Pages, RasterTargets, Region, timed};
use crate::affine::Affine;
use crate::error::RenderError;
use crate::gpu_clock::Stamps;
use crate::output::Output;
use crate::profile::Attached;
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{CoverageDraw, Draw, Instance, Pass, Scratch};
use crate::raster_states::INSTANCE_BYTES;
use crate::readback::{extent, place, texture_bytes};

impl RasterTargets {
    /// Each region's passes on its paper (its slot passes on `pages`), then its finish when it
    /// has one, its values and instances where `bounds` says; timed from the first pass's start
    /// to the last's end by `stamps`.
    pub(super) fn encode(
        &mut self,
        (pipelines, pages): (&RasterPipelines, Pages<'_>),
        (regions, bounds): (&[Region<'_>], &[Bound]),
        (stamps, encoder): (Option<&Stamps>, &mut wgpu::CommandEncoder),
    ) -> Result<(), RenderError> {
        let mut finishing = Vec::with_capacity(regions.len());
        for region in regions {
            finishing.push(match region.finish {
                Some(finish) => Some(pipelines.finish(finish.format).ok_or(
                    RenderError::Invalid("the raster core does not finish into this format"),
                )?),
                None => None,
            });
        }
        let writes = |ends| stamps.and_then(|stamps| stamps.render_writes(ends));
        let (first, last) = timed(regions);

        self.attached = Attached::NONE;
        let regions = regions.iter().zip(bounds).zip(finishing).enumerate();
        for (at, ((region, bound), finishing)) in regions {
            self.bound = *bound;
            let plan = region.plan;
            for (index, pass) in plan.passes.iter().enumerate() {
                let timestamps = writes((first == (at, index), last == (at, index)));
                let place = (plan.size, region.paper);
                self.pass(encoder, (pipelines, pages), (pass, place), timestamps);
                if let Some((bytes, cleared)) = self.weigh(pass) {
                    self.attached.pass(bytes, cleared);
                }
            }
            if let (Some(finishing), Some(finish)) = (finishing, region.finish) {
                let finished = plan.instances.len() as u32;
                let ends = (false, last == (at, plan.passes.len()));
                let size = plan.size;
                self.finish(encoder, (finishing, finish, finished), size, writes(ends))?;
            }
        }
        Ok(())
    }

    /// One pass of a region of `size` on paper `background`.
    fn pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        (pipelines, pages): (&RasterPipelines, Pages<'_>),
        (pass, (size, background)): (&Pass, ([u32; 2], [f32; 4])),
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        match pass {
            Pass::Restore(copies) => self.snapshot_copies(encoder, (copies, true), pages),
            Pass::Keep(copies) => self.snapshot_copies(encoder, (copies, false), pages),
            Pass::Kept { held, draws, keep } => {
                self.kept(encoder, pipelines, (held, draws, keep), pages, timestamps);
            }
            Pass::Zero(slots) => super::slots::zero(encoder, slots, pages),
            Pass::Copy { from, to } => self.copy(encoder, (*from, *to), [0, 0, size[0], size[1]]),
            Pass::Rewrite { target, rect, draw } => {
                self.copy(encoder, (*target, Scratch::Backdrop), *rect);
                let group = self.reads.get(COPIED);
                let place = (*target, wgpu::LoadOp::Load, group);
                let draws = std::slice::from_ref(draw);
                self.draws(encoder, pipelines, place, (draws, size), timestamps);
            }
            Pass::Coverage(draws) => {
                let mut pass = begin(encoder, &self.atlas.view, clear(None), timestamps);
                self.bind(&mut pass, None);
                coverage(&mut pass, pipelines, draws);
            }
            Pass::Clear(target) => {
                let color = (*target == Scratch::Canvas).then_some(background);
                drop(begin(encoder, self.view(*target), clear(color), timestamps));
            }
            Pass::Draw {
                target,
                clear: cleared,
                draws,
            } => {
                let load = if *cleared {
                    clear(None)
                } else {
                    wgpu::LoadOp::Load
                };
                let place = (*target, load, self.reads.get(ATLAS));
                self.draws(encoder, pipelines, place, (draws, size), timestamps);
            }
            Pass::End {
                target,
                reads,
                draw,
            } => {
                let group = self.reads.get(ending(*reads));
                let draws = std::slice::from_ref(draw);
                let place = (*target, wgpu::LoadOp::Load, group);
                self.draws(encoder, pipelines, place, (draws, size), timestamps);
            }
        }
    }

    /// The attachment `pass` draws into, its bytes, and whether the pass clears it first; `None`
    /// for a copy.
    fn weigh(&self, pass: &Pass) -> Option<(u64, bool)> {
        let scratch = |target: Scratch| {
            (self.scratch[target.index()].as_ref()).map_or(0, |at| texture_bytes(&at.texture))
        };
        match pass {
            Pass::Kept { .. } | Pass::Coverage(_) => {
                Some((texture_bytes(&self.atlas.texture), true))
            }
            Pass::Clear(target) => Some((scratch(*target), true)),
            Pass::Draw { target, clear, .. } => Some((scratch(*target), *clear)),
            Pass::Rewrite { target, .. } | Pass::End { target, .. } => {
                Some((scratch(*target), false))
            }
            Pass::Restore(_) | Pass::Keep(_) | Pass::Zero(_) | Pass::Copy { .. } => None,
        }
    }

    /// `draws` onto `target` (loaded by `load`), reading `group`.
    fn draws(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &RasterPipelines,
        (target, load, group): (Scratch, wgpu::LoadOp<wgpu::Color>, Option<&wgpu::BindGroup>),
        (draws, size): (&[Draw], [u32; 2]),
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let mut pass = begin(encoder, self.view(target), load, timestamps);
        region(&mut pass, [0, 0], size);
        self.bind(&mut pass, group);
        let mut set = None;
        for draw in draws.iter().filter(|draw| !draw.instances.is_empty()) {
            if set != Some(draw.kind) {
                pass.set_pipeline(pipelines.draw(draw.kind));
                set = Some(draw.kind);
            }
            pass.draw(0..4, draw.instances.clone());
        }
    }

    /// The scene's group with the region's values, the reads' `group` when given, the region's
    /// instances.
    pub(super) fn bind(&self, pass: &mut wgpu::RenderPass<'_>, group: Option<&wgpu::BindGroup>) {
        if let Some((_, scene)) = self.scene.last() {
            pass.set_bind_group(0, scene, &[self.bound.values]);
        }
        if let Some(group) = group {
            pass.set_bind_group(1, group, &[]);
        }
        pass.set_vertex_buffer(0, self.instances.slice(self.bound.instances..));
    }

    /// The pixels `rect` (x, y, width, height) of `from` copied into `to`, in place.
    fn copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        (from, to): (Scratch, Scratch),
        [x, y, width, height]: [u32; 4],
    ) {
        let (Some(source), Some(target)) = (
            self.scratch[from.index()].as_ref(),
            self.scratch[to.index()].as_ref(),
        ) else {
            return;
        };
        encoder.copy_texture_to_texture(
            place(&source.texture, [x, y]),
            place(&target.texture, [x, y]),
            extent([width, height]),
        );
    }
}

/// Strokes' pieces and stamps `draws` laid into the attachment, each into its region (its
/// scissor) by its kind's pipeline.
pub(super) fn coverage(
    pass: &mut wgpu::RenderPass<'_>,
    pipelines: &RasterPipelines,
    draws: &[CoverageDraw],
) {
    let mut set = None;
    for draw in draws.iter().filter(|draw| !draw.instances.is_empty()) {
        let [x, y, width, height] = draw.scissor;
        if set != Some(draw.coverage) {
            pass.set_pipeline(pipelines.coverage(draw.coverage));
            set = Some(draw.coverage);
        }
        pass.set_scissor_rect(x, y, width, height);
        pass.draw(0..4, draw.instances.clone());
    }
}

/// The pass draws only into the `size` pixels from `at`.
pub(super) fn region(pass: &mut wgpu::RenderPass<'_>, [x, y]: [u32; 2], [width, height]: [u32; 2]) {
    pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0.0, 1.0);
    pass.set_scissor_rect(x, y, width, height);
}

/// Cleared to the premultiplied `color` (the paper), or to nothing.
pub(super) fn clear(color: Option<[f32; 4]>) -> wgpu::LoadOp<wgpu::Color> {
    let [r, g, b, a] = color.unwrap_or([0.0; 4]).map(f64::from);
    wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a })
}

/// A render pass over `view`, starting by `load`, storing what it draws.
pub(super) fn begin<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
    timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'a>>,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("raster core"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        timestamp_writes,
        ..Default::default()
    })
}

/// The values raster.wgsl's `Raster` holds, as GPU bytes: for a region of `size` pixels at
/// `output` whose centres lie on the canvas by `base`, with an atlas of `atlas` pixels.
pub(super) fn values(
    output: Output,
    (size, base): ([u32; 2], Affine),
    atlas: [u32; 2],
    paper: [f32; 4],
) -> Vec<u8> {
    let (scale, inverse) = output.operands();
    let floats = [scale, inverse, scale.log2(), 0.0];
    let mut bytes: Vec<u8> = floats
        .iter()
        .chain(&paper)
        .flat_map(|v| v.to_le_bytes())
        .collect();
    for origin in output.origin {
        bytes.extend(i32::try_from(origin).unwrap_or(i32::MAX).to_le_bytes());
    }
    let sizes = [size[0], size[1], atlas[0], atlas[1]].map(|side| side as f32);
    bytes.extend(
        sizes
            .iter()
            .chain(&[0.0, 0.0])
            .flat_map(|v| v.to_le_bytes()),
    );
    // The base's rows, footprint and level (raster.wgsl's `base_x`, `base_y`), and the map back.
    let [a, b, c, d, e, f] = base.rows();
    let (foot, level) = if base.is_identity() {
        (1.0, 0.0)
    } else {
        let canvas = base.scale();
        ((1.0 / canvas) as f32, -canvas.log2() as f32)
    };
    let [ia, ib, ic, id, ie, iff] = base.inverse().rows();
    let rows = [
        a, b, c, foot, d, e, f, level, ia, ib, ic, 0.0, id, ie, iff, 0.0,
    ];
    bytes.extend(rows.iter().flat_map(|v| v.to_le_bytes()));
    bytes
}

/// The instances as GPU bytes, then the finish's, one a place: the destination pixel at the
/// region's top-left.
pub(super) fn instance_bytes(list: &[Instance], places: &[[i32; 2]]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((list.len() + places.len()) * INSTANCE_BYTES as usize);
    let finishes: Vec<Instance> = (places.iter())
        .map(|place| Instance {
            a: [0; 4],
            b: [place[0], place[1], 0, 0],
        })
        .collect();
    for instance in list.iter().chain(&finishes) {
        bytes.extend(instance.a.iter().flat_map(|v| v.to_le_bytes()));
        bytes.extend(instance.b.iter().flat_map(|v| v.to_le_bytes()));
    }
    bytes
}
