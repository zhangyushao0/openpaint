//! What one render pass costs on this GPU (S57, the raster probe's batching): passes over the
//! probe's attachments (the layer, the atlas, the 8-bit result), each drawing one triangle of a
//! few pixels, back to back, timed by GPU timestamps, three ways: loading the attachment and
//! storing it, clearing and storing it, clearing and discarding it. On a tile-based GPU (the
//! tablet's Adreno) a pass rendered in tile memory loads and stores its whole attachments, so
//! the first two show what a batch's passes cost beyond their draws.

use crate::core::Core;
use crate::error::RenderError;
use crate::gpu_clock::GpuClock;
use crate::pace::Group;
use crate::readback::{FORMAT, target_texture};

/// Passes a measurement takes, and measurements a way (the median kept).
const PASSES: usize = 16;
const MEASURES: usize = 5;

const SHADER: &str = "
@vertex
fn vertex(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32(corner & 1u), f32(corner >> 1u)) * 0.001 - vec2<f32>(1.0);
    return vec4<f32>(p.x, -p.y, 0.0, 1.0);
}

@fragment
fn fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}
";

/// One attachment's passes: ns a pass loading and storing it, clearing and storing it,
/// clearing and discarding it (medians of `MEASURES` runs of `PASSES` passes).
#[derive(Debug, Clone, PartialEq)]
pub struct PassCost {
    pub label: &'static str,
    pub format: String,
    pub size: [u32; 2],
    pub bytes: u64,
    pub ns: [f64; 3],
}

impl Core {
    /// The cost of a render pass over each of the raster probe's attachments, with an atlas
    /// `atlas` pixels square. Empty without GPU timestamps.
    ///
    /// # Errors
    /// As the GPU.
    pub fn time_passes(&self, size: [u32; 2], atlas: u32) -> Result<Vec<PassCost>, RenderError> {
        let pipelines = self.raster_pipelines()?;
        let Some(mut clock) = GpuClock::new(&self.gpu) else {
            return Ok(Vec::new());
        };
        let attachments = [
            ("layer", pipelines.layer, size),
            ("atlas", pipelines.atlas, [atlas; 2]),
            ("result", FORMAT, size),
        ];
        let mut costs = Vec::new();
        for (label, format, size) in attachments {
            let texture = target_texture(
                &self.gpu,
                (size, format),
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            )?;
            let view = texture.create_view(&Default::default());
            let pipeline = self.tiny_pipeline(format);
            let ways = [
                (wgpu::LoadOp::Load, wgpu::StoreOp::Store),
                (
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    wgpu::StoreOp::Store,
                ),
                (
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    wgpu::StoreOp::Discard,
                ),
            ];
            let mut ns = [0.0; 3];
            for (slot, way) in ns.iter_mut().zip(ways) {
                let mut runs = Vec::new();
                for _ in 0..=MEASURES {
                    runs.push(self.passes_once((&view, &pipeline), way, &mut clock)?);
                }
                // The first run settles the attachment (wgpu clears it before its first load).
                runs.remove(0);
                runs.sort_by(f64::total_cmp);
                *slot = runs[runs.len() / 2] / PASSES as f64;
            }
            let texel = u64::from(format.block_copy_size(None).unwrap_or(4));
            costs.push(PassCost {
                label,
                format: format!("{format:?}"),
                size,
                bytes: u64::from(size[0]) * u64::from(size[1]) * texel,
                ns,
            });
        }
        Ok(costs)
    }

    /// `PASSES` passes over `view` the way `(load, store)`, timed: ns from the first's start to
    /// the last's end.
    fn passes_once(
        &self,
        (view, pipeline): (&wgpu::TextureView, &wgpu::RenderPipeline),
        (load, store): (wgpu::LoadOp<wgpu::Color>, wgpu::StoreOp),
        clock: &mut GpuClock,
    ) -> Result<f64, RenderError> {
        let timing = (clock.start(Group::default()))
            .ok_or(RenderError::Invalid("no free timestamp slot"))?;
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        for index in 0..PASSES {
            let ends = (index == 0, index + 1 == PASSES);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pass cost"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load, store },
                })],
                timestamp_writes: clock.render_writes(timing, ends),
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.draw(0..3, 0..1);
        }
        self.gpu.submit([encoder.finish()]);
        clock.finish(&self.gpu, timing);
        for _ in 0..100 {
            self.wait_idle()?;
            if let Some((_, ns)) = clock.readings(&self.gpu).into_iter().next() {
                return Ok(ns);
            }
        }
        Err(RenderError::Invalid("a pass timing never came back"))
    }

    /// A pipeline drawing one tiny triangle into `format`, unblended.
    fn tiny_pipeline(&self, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        let device = &self.gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pass cost"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pass cost"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }
}
