//! A region's passes on the GPU (S57's probe, S59): the plan's passes in order, each a render
//! pass over the atlas or a float target of the region (a copy for a backdrop; a copy, then a
//! pass, for a smudge, S61), then the finish rounding the canvas into the destination; a batch's
//! regions one after another (S63, `raster_batch.rs`), all in one command buffer.

use super::{Bound, Finish, Pages, RasterTargets, Region, timed};
use crate::core::Core;
use crate::error::RenderError;
use crate::geom::PixelRect;
use crate::gpu_clock::Stamps;
use crate::output::Output;
use crate::raster_gpu::RasterPipelines;
use crate::raster_plan::{CoverageDraw, Draw, Instance, Pass, Scratch};
use crate::raster_states::INSTANCE_BYTES;

/// Group 1 for each way a draw reads: the atlas (a layer's draws), a layer's paint with its mask
/// and the backdrop (its end), the chain with them (its end), the backdrop alone (a smudge, which
/// draws onto the layer or its mask), the canvas (the finish).
pub(super) struct Reads {
    atlas: wgpu::BindGroup,
    layer: wgpu::BindGroup,
    chain: wgpu::BindGroup,
    copied: wgpu::BindGroup,
    canvas: wgpu::BindGroup,
}

impl RasterTargets {
    fn view(&self, at: Scratch) -> &wgpu::TextureView {
        &self.scratch[at as usize]
            .as_ref()
            .unwrap_or(&self.unread)
            .view
    }

    /// The reads' bind groups, made when a scratch target was made since.
    fn reads(&mut self, core: &Core, pipelines: &RasterPipelines) -> &Reads {
        if self.reads.is_none() {
            let (atlas, unread) = (&self.atlas.view, &self.unread.view);
            let [canvas, chain, layer, mask, backdrop] = [
                Scratch::Canvas,
                Scratch::Chain,
                Scratch::Layer,
                Scratch::Mask,
                Scratch::Backdrop,
            ]
            .map(|at| self.view(at));
            let group = |views: [&wgpu::TextureView; 3]| {
                core.gpu
                    .device
                    .create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("raster reads"),
                        layout: &pipelines.layouts[1],
                        entries: &(0..)
                            .zip(views)
                            .map(|(binding, view)| wgpu::BindGroupEntry {
                                binding,
                                resource: wgpu::BindingResource::TextureView(view),
                            })
                            .collect::<Vec<_>>(),
                    })
            };
            let reads = Reads {
                atlas: group([atlas, unread, unread]),
                layer: group([layer, mask, backdrop]),
                chain: group([chain, mask, backdrop]),
                copied: group([backdrop, unread, unread]),
                canvas: group([canvas, unread, unread]),
            };
            self.reads = Some(reads);
        }
        self.reads
            .as_ref()
            .unwrap_or_else(|| unreachable!("made above"))
    }

    /// Each region's passes on its paper (its slot passes on `pages`), then its finish when it
    /// has one, its values and instances where `bounds` says; timed from the first pass's start
    /// to the last's end by `stamps`.
    pub(super) fn encode(
        &mut self,
        core: &Core,
        (pipelines, pages): (&RasterPipelines, Pages<'_>),
        (regions, bounds): (&[Region<'_>], &[Bound]),
        stamps: Option<&Stamps>,
    ) -> Result<wgpu::CommandBuffer, RenderError> {
        let mut finishing = Vec::with_capacity(regions.len());
        for region in regions {
            finishing.push(match region.finish {
                Some(finish) => Some(pipelines.finish(finish.format).ok_or(
                    RenderError::Invalid("the raster core does not finish into this format"),
                )?),
                None => None,
            });
        }
        self.reads(core, pipelines);
        let writes = |ends| stamps.and_then(|stamps| stamps.render_writes(ends));
        let (first, last) = timed(regions);
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        let regions = regions.iter().zip(bounds).zip(finishing).enumerate();
        for (at, ((region, bound), finishing)) in regions {
            self.bound = *bound;
            let plan = region.plan;
            for (index, pass) in plan.passes.iter().enumerate() {
                let timestamps = writes((first == (at, index), last == (at, index)));
                let place = (plan.size, region.paper);
                self.pass(&mut encoder, (pipelines, pages), (pass, place), timestamps);
            }
            if let (Some(finishing), Some(finish)) = (finishing, region.finish) {
                let finished = plan.instances.len() as u32;
                let ends = (false, last == (at, plan.passes.len()));
                let size = plan.size;
                self.finish(
                    &mut encoder,
                    (finishing, finish, finished),
                    size,
                    writes(ends),
                );
            }
        }
        Ok(encoder.finish())
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
            Pass::Restore(copies) => self.restore(encoder, copies, pages),
            Pass::Keep(copies) => self.keep(encoder, copies, pages),
            Pass::Kept { copies, draws } => {
                self.kept(encoder, pipelines, (copies, draws), pages, timestamps);
            }
            Pass::Cover { page, clear, draws } => {
                self.cover(
                    encoder,
                    pipelines,
                    (*page, *clear, draws),
                    pages,
                    timestamps,
                );
            }
            Pass::Copy(from) => self.copy(encoder, *from, [0, 0, size[0], size[1]]),
            Pass::Smudge { target, rect, draw } => {
                self.copy(encoder, *target, *rect);
                let group = self.reads.as_ref().map(|reads| &reads.copied);
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
                let reads = self.reads.as_ref().map(|reads| &reads.atlas);
                let place = (*target, load, reads);
                self.draws(encoder, pipelines, place, (draws, size), timestamps);
            }
            Pass::End {
                target,
                reads,
                draw,
            } => {
                let group = self.reads.as_ref().map(|all| match reads {
                    Scratch::Chain => &all.chain,
                    _ => &all.layer,
                });
                let draws = std::slice::from_ref(draw);
                let place = (*target, wgpu::LoadOp::Load, group);
                self.draws(encoder, pipelines, place, (draws, size), timestamps);
            }
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
        if let Some((_, scene)) = &self.scene {
            pass.set_bind_group(0, scene, &[self.bound.values]);
        }
        if let Some(group) = group {
            pass.set_bind_group(1, group, &[]);
        }
        pass.set_vertex_buffer(0, self.instances.slice(self.bound.instances..));
    }

    /// The pixels `rect` (x, y, width, height) of `from` copied into the backdrop, in place.
    fn copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        from: Scratch,
        [x, y, width, height]: [u32; 4],
    ) {
        let (Some(source), Some(backdrop)) = (
            self.scratch[from as usize].as_ref(),
            self.scratch[Scratch::Backdrop as usize].as_ref(),
        ) else {
            return;
        };
        let at = |texture| wgpu::TexelCopyTextureInfo {
            origin: wgpu::Origin3d { x, y, z: 0 },
            ..wgpu::Texture::as_image_copy(texture)
        };
        encoder.copy_texture_to_texture(
            at(&source.texture),
            at(&backdrop.texture),
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// The canvas rounded into the destination's pixels from `finish.at` (only those of
    /// `finish.only` when it names any), by `pipeline`, its instance `finished` naming that place.
    fn finish(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        (pipeline, finish, finished): (&wgpu::RenderPipeline, &Finish<'_>, u32),
        size: [u32; 2],
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let mut pass = begin(encoder, finish.view, wgpu::LoadOp::Load, timestamps);
        region(&mut pass, finish.at, size);
        self.bind(&mut pass, self.reads.as_ref().map(|reads| &reads.canvas));
        pass.set_pipeline(pipeline);
        if finish.only.is_empty() {
            pass.draw(0..3, finished..finished + 1);
            return;
        }
        let [x, y] = finish.at.map(|at| i32::try_from(at).unwrap_or(i32::MAX));
        let whole = PixelRect {
            min: [x, y],
            max: [x + size[0] as i32, y + size[1] as i32],
        };
        for rect in finish.only {
            let part = rect.intersect(&whole);
            if part.is_empty() {
                continue;
            }
            let [x0, y0] = part.min.map(i32::unsigned_abs);
            let [x1, y1] = part.max.map(i32::unsigned_abs);
            pass.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
            pass.draw(0..3, finished..finished + 1);
        }
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
fn region(pass: &mut wgpu::RenderPass<'_>, [x, y]: [u32; 2], [width, height]: [u32; 2]) {
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

/// The values raster.wgsl's `Raster` holds, as GPU bytes.
pub(super) fn values(output: Output, size: [u32; 2], atlas: [u32; 2], paper: [f32; 4]) -> Vec<u8> {
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
    bytes
}

/// The instances as GPU bytes, then the finish's: the destination pixel at the region's top-left.
pub(super) fn instance_bytes(list: &[Instance], at: [u32; 2]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity((list.len() + 1) * INSTANCE_BYTES as usize);
    let place = at.map(|v| i32::try_from(v).unwrap_or(i32::MAX));
    let finish = Instance {
        a: [0; 4],
        b: [place[0], place[1], 0, 0],
    };
    for instance in list.iter().chain([&finish]) {
        bytes.extend(instance.a.iter().flat_map(|v| v.to_le_bytes()));
        bytes.extend(instance.b.iter().flat_map(|v| v.to_le_bytes()));
    }
    bytes
}
