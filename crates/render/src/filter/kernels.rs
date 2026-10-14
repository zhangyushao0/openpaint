//! The filter kernels (`shaders/filter.wgsl`): a separable Gaussian over a filter's block, on
//! cells of `factor` x `factor` canvas pixels for a large deviation (S35, `Cells`), in bands of
//! `BAND_ROWS` cell rows. Per band `across` sums the rows the band's output reads (the kernel's
//! reach beyond it each way) into the band buffer, then `down` sums its columns and writes the
//! block's words, or at a factor above 1 the blurred cells, which `enlarge` then spreads onto the
//! block's pixels. Each band's values sit at their own 256-byte place in one uniform buffer. The
//! band and cell buffers grow to the largest asked for and are kept.

use crate::geom::PixelRect;
use crate::gpu::Gpu;
use crate::protocol::encode;

pub(crate) const SHADER: &str = include_str!("../shaders/filter.wgsl");
/// Output rows per band.
const BAND_ROWS: u32 = 128;
/// Bytes between two bands' values: the uniform offset alignment every device accepts.
const STRIDE: u64 = 256;
/// Bytes of one band's values (`Job` in the shader).
const JOB_BYTES: u64 = 80;
/// The longest reach in cells: a deviation is reduced below 8 cells before it reaches further.
pub(crate) const MAX_CELL_REACH: u32 = 24;
/// Bytes of the weights at the longest kernel.
const WEIGHT_BYTES: u64 = (2 * MAX_CELL_REACH as u64 + 1) * 4;

/// How a filter of deviation `sigma` (canvas pixels) is evaluated over a block: canvas pixels
/// per cell, the deviation in cells (lowered by what the cells' means and the enlarging add, so
/// the whole spreads as `sigma` does), the cells the block's pixels need and the canvas pixels
/// the kernel reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cells {
    pub(crate) factor: u32,
    pub(crate) sigma: f32,
    /// First column, first row, columns, rows.
    pub(crate) needed: [u32; 4],
    pub(crate) read: PixelRect,
}

/// Canvas pixels per cell at deviation `sigma`: 4 from 8 pixels, 2 from 4, else 1, so a cell
/// kernel keeps a deviation of at least 2 cells (within 2 levels of the exact Gaussian on edges
/// and lines, measured).
pub(crate) fn factor(sigma: f32) -> u32 {
    if sigma >= 8.0 {
        4
    } else if sigma >= 4.0 {
        2
    } else {
        1
    }
}

impl Cells {
    /// The cells for a block `block` (canvas pixels, not empty) on a canvas of `canvas` pixels.
    pub(crate) fn of(block: PixelRect, canvas: [u32; 2], sigma: f32) -> Self {
        let factor = factor(sigma);
        let f = factor as f32;
        // A cell's mean adds (f^2 - 1) / 12 to the variance, the enlarging about f^2 / 6.
        let added = if factor > 1 {
            (f * f - 1.0) / 12.0 + f * f / 6.0
        } else {
            0.0
        };
        let cell_sigma = (sigma * sigma - added).max(0.25 * f * f).sqrt() / f;
        let cell_sigma = if factor > 1 { cell_sigma } else { sigma };
        let low = canvas.map(|side| side.div_ceil(factor));
        let reach = super::reach(cell_sigma) as i32;
        let (needed, read) = if factor == 1 {
            (block, block)
        } else {
            let at = |v: i32| ((v as f32 + 0.5) / f - 0.5).floor() as i32;
            let first = [at(block.min[0]).max(0), at(block.min[1]).max(0)];
            let last = [0, 1].map(|axis| (at(block.max[axis] - 1) + 2).min(low[axis] as i32));
            let needed = PixelRect {
                min: first,
                max: last,
            };
            let scaled = |rect: PixelRect| PixelRect {
                min: rect.min.map(|v| v * factor as i32),
                max: rect.max.map(|v| v * factor as i32),
            };
            (needed, scaled(needed))
        };
        let widened = PixelRect {
            min: read.min.map(|v| v - reach * factor as i32),
            max: read.max.map(|v| v + reach * factor as i32),
        };
        let size = needed.max.map(|v| v as u32);
        Self {
            factor,
            sigma: cell_sigma,
            needed: [
                needed.min[0] as u32,
                needed.min[1] as u32,
                size[0] - needed.min[0] as u32,
                size[1] - needed.min[1] as u32,
            ],
            read: widened.intersect(&PixelRect::full(canvas)),
        }
    }
}

/// One filter's work: its block (`[x, y, width, height]` in canvas pixels), its cells, the
/// canvas's size, the word where the block's pixels start in the arena, the kernel's weights in
/// cells (an odd count) and the sharpening strength (0: a blur).
pub(crate) struct FilterJob {
    pub(crate) block: [u32; 4],
    pub(crate) cells: Cells,
    pub(crate) canvas: [u32; 2],
    pub(crate) base: u32,
    pub(crate) weights: Vec<f32>,
    pub(crate) strength: f32,
}

pub(crate) struct FilterPipeline {
    layout: wgpu::BindGroupLayout,
    kernels: [Kernels; 2],
}

/// The bands' values, the weights, the band of row sums, the blurred cells, and the bind group
/// last built (for the arena buffer, the material view and the buffers it names).
pub(crate) struct FilterBuffers {
    params: wgpu::Buffer,
    weights: wgpu::Buffer,
    band: wgpu::Buffer,
    cells: wgpu::Buffer,
    group: Option<([wgpu::Buffer; 4], wgpu::TextureView, wgpu::BindGroup)>,
}

impl FilterBuffers {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        Self {
            params: buffer(gpu, "filter bands", STRIDE, uniform),
            weights: buffer(gpu, "filter weights", WEIGHT_BYTES, storage()),
            band: buffer(gpu, "filter band", 16, storage()),
            cells: buffer(gpu, "filter cells", 16, storage()),
            group: None,
        }
    }

    /// Bytes of its buffers.
    pub(crate) fn bytes(&self) -> u64 {
        [&self.params, &self.weights, &self.band, &self.cells]
            .iter()
            .map(|buffer| buffer.size())
            .sum()
    }

    /// Grows `slot` (the band or the cells) to `bytes`.
    fn hold(gpu: &Gpu, slot: &mut wgpu::Buffer, bytes: u64, label: &str) {
        if bytes > slot.size() {
            *slot = buffer(gpu, label, bytes, storage());
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

/// The three kernels compiled for reaches up to `reach` cells: workgroup memory for that reach
/// only, so short kernels keep more workgroups in flight.
struct Kernels {
    reach: u32,
    across: wgpu::ComputePipeline,
    down: wgpu::ComputePipeline,
    enlarge: wgpu::ComputePipeline,
}

/// The reaches the kernels are compiled for, shortest first; the last is `MAX_CELL_REACH`.
const REACHES: [u32; 2] = [8, MAX_CELL_REACH];

impl FilterPipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let layout = layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("filter"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let kernels = REACHES.map(|reach| {
            let source = format!("const MAX_CELL_REACH: u32 = {reach}u;\n{SHADER}");
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("filter"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
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
            Kernels {
                reach,
                across: pipeline("across"),
                down: pipeline("down"),
                enlarge: pipeline("enlarge"),
            }
        });
        Self { layout, kernels }
    }

    /// Runs `job`, reading the layer's pixels from `material` and writing into `arena`.
    pub(crate) fn run(
        &self,
        gpu: &Gpu,
        buffers: &mut FilterBuffers,
        job: &FilterJob,
        (material, arena): (&wgpu::TextureView, &wgpu::Buffer),
    ) {
        let [_, _, columns, rows] = job.cells.needed;
        let reach = (job.weights.len() as u32) / 2;
        let bands = bands(job, reach);
        let band_rows = u64::from(BAND_ROWS + 2 * reach);
        FilterBuffers::hold(
            gpu,
            &mut buffers.band,
            band_rows * u64::from(columns) * 16,
            "filter band",
        );
        if job.cells.factor > 1 {
            let bytes = u64::from(columns) * u64::from(rows) * 16;
            FilterBuffers::hold(gpu, &mut buffers.cells, bytes, "filter cells");
        }
        let params_bytes = STRIDE * bands.len() as u64;
        if params_bytes > buffers.params.size() {
            let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
            buffers.params = buffer(gpu, "filter bands", params_bytes, uniform);
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
        let kernels = self
            .kernels
            .iter()
            .find(|kernels| reach <= kernels.reach)
            .unwrap_or(&self.kernels[1]);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            for (index, band) in bands.iter().enumerate() {
                pass.set_bind_group(0, &group, &[(index as u64 * STRIDE) as u32]);
                pass.set_pipeline(&kernels.across);
                pass.dispatch_workgroups(columns.div_ceil(16), band[1].div_ceil(16), 1);
                pass.set_pipeline(&kernels.down);
                pass.dispatch_workgroups(columns.div_ceil(16), band[3].div_ceil(16), 1);
            }
            if job.cells.factor > 1 {
                pass.set_bind_group(0, &group, &[0]);
                pass.set_pipeline(&kernels.enlarge);
                let [_, _, width, height] = job.block;
                pass.dispatch_workgroups(width.div_ceil(16), height.div_ceil(16), 1);
            }
        }
        gpu.submit([encoder.finish()]);
    }

    /// The kernels' bindings, built again only when a buffer or the material changed.
    fn group(
        &self,
        gpu: &Gpu,
        buffers: &mut FilterBuffers,
        (material, arena): (&wgpu::TextureView, &wgpu::Buffer),
    ) -> wgpu::BindGroup {
        let named = [&buffers.params, &buffers.band, &buffers.cells, arena];
        if let Some((built, view, group)) = &buffers.group
            && built.iter().zip(named).all(|(a, b)| a == b)
            && view == material
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
                entry(5, buffers.cells.as_entire_binding()),
            ],
        });
        buffers.group = Some((named.map(Clone::clone), material.clone(), group.clone()));
        group
    }
}

/// The bands of `job`'s cell rows, `BAND_ROWS` written each: the first row the band reads, the
/// rows it reads (the kernel's `reach` beyond each way, inside the canvas), the first row it
/// writes, the rows it writes.
fn bands(job: &FilterJob, reach: u32) -> Vec<[u32; 4]> {
    let [_, first, _, rows] = job.cells.needed;
    let low_height = job.canvas[1].div_ceil(job.cells.factor);
    (0..rows.div_ceil(BAND_ROWS))
        .map(|band| {
            let from_row = first + band * BAND_ROWS;
            let last = (from_row + BAND_ROWS).min(first + rows);
            let from = from_row.saturating_sub(reach);
            let to = (last + reach).min(low_height);
            [from, to - from, from_row, last - from_row]
        })
        .collect()
}

fn entry(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
}

/// A band's values as the shader's `Job` lays them out: 20 words.
fn job_words(job: &FilterJob, band: [u32; 4], reach: u32) -> [u32; 20] {
    let [x, y, width, height] = job.block;
    let [cell_x, cell_y, columns, rows] = job.cells.needed;
    let [from, read, first, written] = band;
    let factor = job.cells.factor;
    let [canvas_width, canvas_height] = job.canvas;
    [
        x,
        y,
        width,
        height,
        cell_x,
        cell_y,
        columns,
        rows,
        from,
        read,
        first,
        written,
        canvas_width,
        canvas_height,
        canvas_width.div_ceil(factor),
        canvas_height.div_ceil(factor),
        reach,
        job.base,
        job.strength.to_bits(),
        factor,
    ]
}

/// The kernels' bindings: a band's values (at a dynamic offset), the weights, the material, the
/// band of row sums, the arena and the blurred cells.
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
            buffer(5, storage(false), false),
        ],
    })
}
