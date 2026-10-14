//! Thumbnails: a canvas target reduced on the GPU to a small image for the gallery and read
//! back. Each thumbnail pixel is the rounded mean of the canvas pixels in its box, computed in
//! 8-bit integers, so `thumbnail_reference` reproduces it exactly on the CPU (the test that the
//! thumbnail matches the artwork). The work on the calling thread is fixed: one dispatch and a
//! readback of at most `max` pixels.

use wgpu::util::DeviceExt;

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::image::RgbaImage;
use crate::readback::read_buffer;

const SHADER: &str = include_str!("shaders/thumbnail.wgsl");
/// The shader's workgroup side.
const GROUP: u32 = 8;

/// The size of the thumbnail of a `canvas` that fits in `max`: the canvas scaled down (never
/// up) to touch `max` on one side, rounded to whole pixels, at least one pixel per side.
#[must_use]
pub fn thumbnail_size(canvas: [u32; 2], max: [u32; 2]) -> [u32; 2] {
    let scale = |side: u32, limit: u32| f64::from(limit) / f64::from(side.max(1));
    let factor = scale(canvas[0], max[0])
        .min(scale(canvas[1], max[1]))
        .min(1.0);
    [0, 1].map(|axis| {
        let side = (f64::from(canvas[axis]) * factor).round() as u32;
        side.clamp(1, canvas[axis].max(1))
    })
}

/// The thumbnail of `canvas` (RGBA pixels, `size`) at `thumbnail` size, on the CPU: what the GPU
/// path must produce bit for bit.
#[must_use]
pub fn thumbnail_reference(canvas: &RgbaImage, thumbnail: [u32; 2]) -> Vec<u8> {
    let [width, height] = canvas.size().map(u64::from);
    let [w, h] = thumbnail.map(u64::from);
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        let rows = (y * height / h)..((y + 1) * height / h);
        for x in 0..w {
            let columns = (x * width / w)..((x + 1) * width / w);
            let mut sum = [0u64; 4];
            for row in rows.clone() {
                for column in columns.clone() {
                    let at = ((row * width + column) * 4) as usize;
                    for (channel, total) in sum.iter_mut().enumerate() {
                        *total += u64::from(canvas.pixels()[at + channel]);
                    }
                }
            }
            let count = (rows.end - rows.start) * (columns.end - columns.start);
            out.extend(sum.map(|total| ((total + count / 2) / count) as u8));
        }
    }
    out
}

pub(crate) struct ThumbnailPipeline {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

impl ThumbnailPipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("thumbnail"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let buffer = |ty| wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        let texture = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let storage = wgpu::BufferBindingType::Storage { read_only: false };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("thumbnail"),
            entries: &[
                entry(0, texture),
                entry(1, buffer(wgpu::BufferBindingType::Uniform)),
                entry(2, buffer(storage)),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("thumbnail"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("thumbnail"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("thumbnail"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layout }
    }

    fn bind_group(
        &self,
        device: &wgpu::Device,
        target: &CanvasTarget,
        sizes: &wgpu::Buffer,
        pixels: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let view = target.texture().create_view(&Default::default());
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("thumbnail"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sizes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: pixels.as_entire_binding(),
                },
            ],
        })
    }
}

impl Core {
    /// The thumbnail of what `target` shows now, at most `max` pixels (`thumbnail_size`).
    /// Waits for the GPU, including evaluations still running.
    pub fn thumbnail(
        &self,
        target: &CanvasTarget,
        max: [u32; 2],
    ) -> Result<RgbaImage, RenderError> {
        let canvas = target.size();
        let size = thumbnail_size(canvas, max);
        let device = &self.gpu.device;
        let words: Vec<u8> = [canvas, size]
            .concat()
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let sizes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("thumbnail sizes"),
            contents: &words,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bytes = u64::from(size[0]) * u64::from(size[1]) * 4;
        let pixels = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("thumbnail pixels"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let group = self.thumbnail.bind_group(device, target, &sizes, &pixels);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.thumbnail.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(size[0].div_ceil(GROUP), size[1].div_ceil(GROUP), 1);
        }
        self.gpu.submit([encoder.finish()]);
        let data = read_buffer(&self.gpu, &pixels, bytes)?;
        Ok(RgbaImage::new(size, data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnails_keep_the_aspect_and_never_grow() {
        assert_eq!(thumbnail_size([2880, 1716], [448, 336]), [448, 267]);
        assert_eq!(thumbnail_size([1000, 2000], [448, 336]), [168, 336]);
        assert_eq!(thumbnail_size([64, 48], [448, 336]), [64, 48]);
        assert_eq!(thumbnail_size([5000, 1], [448, 336]), [448, 1]);
    }
}
