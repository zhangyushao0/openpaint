//! The filter kernels (`shaders/filter.wgsl`): a separable Gaussian over a filter's block in
//! bands of `BAND_ROWS` rows. Per band `across` sums the rows the band's output reads (the
//! kernel's reach beyond it each way) into the band buffer, then `down` sums its columns and
//! writes the block's words; each band's values sit at their own 256-byte place in one uniform
//! buffer. The band buffer grows to the widest band asked for and is kept.

use super::MAX_REACH;
use crate::gpu::Gpu;
use crate::protocol::encode;

const SHADER: &str = include_str!("../shaders/filter.wgsl");
/// Output rows per band.
const BAND_ROWS: u32 = 128;
/// Bytes between two bands' values: the uniform offset alignment every device accepts.
const STRIDE: u64 = 256;
/// Bytes of one band's values (`Job` in the shader).
const JOB_BYTES: u64 = 64;
/// Bytes of the weights at the longest kernel.
const WEIGHT_BYTES: u64 = (2 * MAX_REACH as u64 + 1) * 4;

/// One filter's work: its block (`[x, y, width, height]` in canvas pixels), the canvas's size,
/// the word where the block's pixels start in the arena, the kernel's weights (an odd count) and
/// the sharpening strength (0: a blur).
pub(crate) struct FilterJob {
    pub(crate) block: [u32; 4],
    pub(crate) canvas: [u32; 2],
    pub(crate) base: u32,
    pub(crate) weights: Vec<f32>,
    pub(crate) strength: f32,
}

pub(crate) struct FilterPipeline {
    layout: wgpu::BindGroupLayout,
    across: wgpu::ComputePipeline,
    down: wgpu::ComputePipeline,
}

/// The bands' values, the weights, the band of row sums, and the bind group last built (for the
/// arena buffer, the material view and the band buffer it names).
pub(crate) struct FilterBuffers {
    params: wgpu::Buffer,
    weights: wgpu::Buffer,
    band: wgpu::Buffer,
    group: Option<(
        wgpu::Buffer,
        wgpu::TextureView,
        wgpu::Buffer,
        wgpu::BindGroup,
    )>,
}

impl FilterBuffers {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        Self {
            params: buffer(gpu, "filter bands", STRIDE, uniform),
            weights: buffer(gpu, "filter weights", WEIGHT_BYTES, storage()),
            band: buffer(gpu, "filter band", 16, storage()),
            group: None,
        }
    }
}

fn storage() -> wgpu::BufferUsages {
    wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST
}

fn buffer(gpu: &Gpu, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

impl FilterPipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("filter"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("filter"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Self {
            across: pipeline("across"),
            down: pipeline("down"),
            layout,
        }
    }

    /// Runs `job`, reading the layer's pixels from `material` and writing into `arena`.
    pub(crate) fn run(
        &self,
        gpu: &Gpu,
        buffers: &mut FilterBuffers,
        job: &FilterJob,
        (material, arena): (&wgpu::TextureView, &wgpu::Buffer),
    ) {
        let [_, y, width, height] = job.block;
        let reach = (job.weights.len() as u32) / 2;
        let bands: Vec<[u32; 4]> = (0..height.div_ceil(BAND_ROWS))
            .map(|band| {
                let first = y + band * BAND_ROWS;
                let last = (first + BAND_ROWS).min(y + height);
                let from = first.saturating_sub(reach);
                let to = (last + reach).min(job.canvas[1]);
                [from, to - from, first, last - first]
            })
            .collect();
        let rows = u64::from(BAND_ROWS + 2 * reach);
        let band_bytes = rows * u64::from(width) * 16;
        if band_bytes > buffers.band.size() {
            buffers.band = buffer(gpu, "filter band", band_bytes, storage());
        }
        let params_bytes = STRIDE * bands.len() as u64;
        if params_bytes > buffers.params.size() {
            let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
            buffers.params = buffer(gpu, "filter bands", params_bytes, uniform);
            buffers.group = None;
        }
        let mut bytes = vec![0; bands.len() * STRIDE as usize];
        let (slots, _) = bytes.as_chunks_mut::<{ STRIDE as usize }>();
        for (slot, band) in slots.iter_mut().zip(&bands) {
            let values = encode(&job_words(job, *band, reach));
            slot[..values.len()].copy_from_slice(&values);
        }
        gpu.queue.write_buffer(&buffers.params, 0, &bytes);
        let weight_words: Vec<u32> = job.weights.iter().map(|weight| weight.to_bits()).collect();
        gpu.queue
            .write_buffer(&buffers.weights, 0, &encode(&weight_words));
        let group = self.group(gpu, buffers, (material, arena));
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            for (index, band) in bands.iter().enumerate() {
                pass.set_bind_group(0, &group, &[(index as u64 * STRIDE) as u32]);
                pass.set_pipeline(&self.across);
                pass.dispatch_workgroups(width.div_ceil(16), band[1].div_ceil(16), 1);
                pass.set_pipeline(&self.down);
                pass.dispatch_workgroups(width.div_ceil(16), band[3].div_ceil(16), 1);
            }
        }
        gpu.queue.submit([encoder.finish()]);
    }

    /// The kernels' bindings, built again only when a buffer or the material changed.
    fn group(
        &self,
        gpu: &Gpu,
        buffers: &mut FilterBuffers,
        (material, arena): (&wgpu::TextureView, &wgpu::Buffer),
    ) -> wgpu::BindGroup {
        if let Some((built, view, band, group)) = &buffers.group
            && built == arena
            && view == material
            && *band == buffers.band
        {
            return group.clone();
        }
        let params = wgpu::BufferBinding {
            buffer: &buffers.params,
            offset: 0,
            size: wgpu::BufferSize::new(JOB_BYTES),
        };
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("filter"),
            layout: &self.layout,
            entries: &[
                entry(0, wgpu::BindingResource::Buffer(params)),
                entry(1, buffers.weights.as_entire_binding()),
                entry(2, wgpu::BindingResource::TextureView(material)),
                entry(3, buffers.band.as_entire_binding()),
                entry(4, arena.as_entire_binding()),
            ],
        });
        let kept = (arena.clone(), material.clone(), buffers.band.clone());
        buffers.group = Some((kept.0, kept.1, kept.2, group.clone()));
        group
    }
}

fn entry(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
}

/// A band's values as the shader's `Job` lays them out: 16 words.
fn job_words(job: &FilterJob, band: [u32; 4], reach: u32) -> [u32; 16] {
    let [x, y, width, height] = job.block;
    let [from, rows, first, written] = band;
    [
        x,
        y,
        width,
        height,
        from,
        rows,
        first,
        written,
        job.canvas[0],
        job.canvas[1],
        reach,
        job.base,
        job.strength.to_bits(),
        0,
        0,
        0,
    ]
}

/// The kernels' bindings: a band's values (at a dynamic offset), the weights, the material, the
/// band of row sums and the arena.
fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let buffer = |binding, ty, dynamic| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: dynamic,
            min_binding_size: None,
        },
        count: None,
    };
    let storage = |read_only| wgpu::BufferBindingType::Storage { read_only };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("filter"),
        entries: &[
            buffer(0, wgpu::BufferBindingType::Uniform, true),
            buffer(1, storage(true), false),
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            buffer(3, storage(false), false),
            buffer(4, storage(false), false),
        ],
    })
}
