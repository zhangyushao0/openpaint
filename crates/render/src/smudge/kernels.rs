//! The smudge kernels (`shaders/smudge.wgsl`): resetting newly allocated tiles and applying
//! contacts to a residual being evaluated, two dispatches per contact (stamp, then commit), at
//! most `JOBS_PER_SUBMIT` jobs per submission, each job's values at its own 256-byte place in
//! one uniform buffer.

use super::RESIDUAL_TILE;
use super::dabs::{Dab, MAX_RADIUS};
use crate::gpu::Gpu;
use crate::protocol::{encode, prelude};
use crate::smudge::NODE_WORDS;

const SHADER: &str = include_str!("../shaders/smudge.wgsl");
/// Bytes between two jobs' values: the uniform offset alignment every device accepts.
const STRIDE: u64 = 256;
/// Bytes of one job's values (`Contact` in the shader).
const CONTACT_BYTES: u64 = 64;
/// Jobs per submission.
const JOBS_PER_SUBMIT: usize = 512;
/// Nodes one submission's jobs cover at most (S49): about 70 ms on the tablet, where a contact of
/// radius 300 (about 363,000 nodes, stamped and committed) took about 1.5 ms (S21).
const NODES_PER_SUBMIT: u64 = 1 << 24;
/// Nodes one contact can change at most: its rectangle's side is under `2 r + 3`.
const SCRATCH_NODES: u64 = (2 * MAX_RADIUS as u64 + 3) * (2 * MAX_RADIUS as u64 + 3);

/// One dispatch's worth of work.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Job {
    /// A newly allocated tile's nodes start as the identity.
    Reset([u32; 4]),
    /// A contact over the nodes of its rectangle.
    Contact(Dab, [u32; 4]),
}

pub(crate) struct SmudgePipeline {
    layout: wgpu::BindGroupLayout,
    reset: wgpu::ComputePipeline,
    stamp: wgpu::ComputePipeline,
    commit: wgpu::ComputePipeline,
}

/// What the kernels write to besides the residual: the jobs' values and the stamp's scratch.
pub(crate) struct KernelBuffers {
    params: wgpu::Buffer,
    scratch: wgpu::Buffer,
}

impl KernelBuffers {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        Self {
            params: gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("smudge contacts"),
                size: STRIDE * JOBS_PER_SUBMIT as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            scratch: gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("smudge scratch"),
                size: SCRATCH_NODES * u64::from(NODE_WORDS) * 4,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
        }
    }

    /// Bytes of its buffers.
    pub(crate) fn bytes(&self) -> u64 {
        self.params.size() + self.scratch.size()
    }
}

impl SmudgePipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("smudge"),
            source: wgpu::ShaderSource::Wgsl((prelude() + SHADER).into()),
        });
        let layout = layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("smudge"),
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
            reset: pipeline("reset"),
            stamp: pipeline("stamp"),
            commit: pipeline("commit"),
            layout,
        }
    }

    /// The kernels' bindings for the residual in `words`, reading the layer's colour before
    /// the smudge from `material`.
    pub(crate) fn group(
        &self,
        gpu: &Gpu,
        buffers: &KernelBuffers,
        words: &wgpu::Buffer,
        material: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        let params = wgpu::BufferBinding {
            buffer: &buffers.params,
            offset: 0,
            size: wgpu::BufferSize::new(CONTACT_BYTES),
        };
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("smudge"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(params),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: words.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffers.scratch.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(material),
                },
            ],
        })
    }

    /// Runs `jobs` in order on the residual `group` binds, on a canvas of `canvas` pixels.
    pub(crate) fn run(
        &self,
        gpu: &Gpu,
        (buffers, group): (&KernelBuffers, &wgpu::BindGroup),
        jobs: &[Job],
        canvas: [u32; 2],
    ) {
        for chunk in submissions(jobs) {
            let mut bytes = vec![0; chunk.len() * STRIDE as usize];
            let (slots, _) = bytes.as_chunks_mut::<{ STRIDE as usize }>();
            for (slot, job) in slots.iter_mut().zip(chunk) {
                let values = encode(&contact_words(job, canvas));
                slot[..values.len()].copy_from_slice(&values);
            }
            gpu.queue.write_buffer(&buffers.params, 0, &bytes);
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                for (index, job) in chunk.iter().enumerate() {
                    let offset = (index as u64 * STRIDE) as u32;
                    pass.set_bind_group(0, group, &[offset]);
                    let (Job::Reset(rect) | Job::Contact(_, rect)) = job;
                    let groups = [rect[2].div_ceil(8), rect[3].div_ceil(8)];
                    let pipelines: &[&wgpu::ComputePipeline] = match job {
                        Job::Reset(_) => &[&self.reset],
                        Job::Contact(..) => &[&self.stamp, &self.commit],
                    };
                    for pipeline in pipelines {
                        pass.set_pipeline(pipeline);
                        pass.dispatch_workgroups(groups[0], groups[1], 1);
                    }
                }
            }
            gpu.queue.submit([encoder.finish()]);
        }
    }
}

/// `jobs` cut in order into submissions of at most `JOBS_PER_SUBMIT` jobs over at most
/// `NODES_PER_SUBMIT` nodes (a job above it alone), so no submission runs past the GPU's time
/// limit (S49).
fn submissions(jobs: &[Job]) -> Vec<&[Job]> {
    let nodes = |job: &Job| {
        let (Job::Reset(rect) | Job::Contact(_, rect)) = job;
        u64::from(rect[2]) * u64::from(rect[3])
    };
    let mut out = Vec::new();
    let (mut start, mut covered) = (0, 0);
    for (index, job) in jobs.iter().enumerate() {
        let full = index - start == JOBS_PER_SUBMIT || covered + nodes(job) > NODES_PER_SUBMIT;
        if index > start && full {
            out.push(&jobs[start..index]);
            (start, covered) = (index, 0);
        }
        covered += nodes(job);
    }
    if start < jobs.len() {
        out.push(&jobs[start..]);
    }
    out
}

/// The kernels' bindings: the job's values (one `Contact` at a dynamic offset), the residual's
/// words, the scratch and the material.
fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let storage = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("smudge"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(CONTACT_BYTES),
                },
                count: None,
            },
            storage(1),
            storage(2),
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
        ],
    })
}

/// A job's values as the shader's `Contact` lays them out: 16 words.
fn contact_words(job: &Job, [width, height]: [u32; 2]) -> [u32; 16] {
    let (rect, dab) = match job {
        Job::Reset(rect) => (*rect, None),
        Job::Contact(dab, rect) => (*rect, Some(dab)),
    };
    let float = f32::to_bits;
    let mut words = [0; 16];
    words[..4].copy_from_slice(&rect);
    if let Some(dab) = dab {
        words[4..10].copy_from_slice(&[
            float(dab.centre[0]),
            float(dab.centre[1]),
            float(dab.step[0]),
            float(dab.step[1]),
            float(dab.radius),
            float(dab.strength),
        ]);
        words[10] = dab.kernel as u32;
    }
    words[12] = width;
    words[13] = height;
    words
}

/// The nodes of residual tile `tile` (row major over a canvas of `size` pixels) as a reset's
/// rectangle.
pub(crate) fn tile_job(tile: u32, size: [u32; 2]) -> Job {
    let columns = size[0].div_ceil(RESIDUAL_TILE);
    let origin = [
        tile % columns * RESIDUAL_TILE,
        tile / columns * RESIDUAL_TILE,
    ];
    let width = RESIDUAL_TILE.min(size[0] - origin[0]);
    let height = RESIDUAL_TILE.min(size[1] - origin[1]);
    Job::Reset([origin[0], origin[1], width, height])
}
