//! The evaluator's compute pipeline and the bindings a plan is dispatched through
//! (`program_dispatch.rs`: one evaluation in bounded runs of tiles, S49). Since S65 the evaluator
//! evaluates only the reference images (`evaluator.rs` says what remains and why).

use std::sync::atomic::AtomicU64;

use wgpu::util::DeviceExt;

use crate::affine::Affine;
use crate::bounded::DISPATCH_WORK;
use crate::gpu::Gpu;
use crate::gpu_clock::{GpuClock, Timing};
use crate::output::Output;
use crate::paper::paper_levels;
use crate::plan::Plan;
use crate::protocol::{Op, Params, Record, encode};
use crate::shapes::shapes;
use crate::tally::Tally;

#[path = "program_scene.rs"]
mod scene_ops;
pub(crate) use scene_ops::{SceneOps, scene_ops};

#[path = "program_layout.rs"]
mod layout;
use layout::evaluate_layout;
pub(crate) use layout::shader_source;

#[path = "program_dispatch.rs"]
mod dispatch;

/// What one dispatch evaluates.
pub(crate) struct Dispatch<'a> {
    /// The scene's ops and its fill masks (`scene_ops`), uploaded once per scene.
    pub(crate) ops: &'a SceneOps,
    /// Ops of this evaluation behind the scene's (a material's layer end).
    pub(crate) fresh: &'a [Op],
    pub(crate) plan: &'a Plan,
    /// Where the target lies in the output (`Output::CANVAS` for the canvas itself).
    pub(crate) output: Output,
    pub(crate) background: [f32; 4],
    /// The smudge arena and the in-progress smudge's residual (`Smudges::residuals`).
    pub(crate) residuals: [&'a wgpu::Buffer; 2],
    /// Where the target's pixel centres lie on the canvas: the identity, but for a smudge's
    /// material seen from a transform record (S31).
    pub(crate) base: Affine,
    /// Its GPU time measured by these timestamps (the benchmark, `gpu_clock.rs`).
    pub(crate) timing: Option<(&'a GpuClock, Timing)>,
}

/// One target's evaluation resources, kept between dispatches: buffers are rewritten in place
/// and the bind group is rebuilt only when a buffer grows or the scene's ops change. A bind
/// group per frame would cycle Vulkan descriptor pools, a periodic multi-millisecond stall.
pub(crate) struct Bindings {
    size: [u32; 2],
    canvas: wgpu::TextureView,
    params: wgpu::Buffer,
    /// Tiles and op entries, in binding order 2, 3 (fresh ops follow the scene's, S55).
    lists: [wgpu::Buffer; 2],
    /// The plan whose entries the list holds (`Plan::serial`; 0: none), placed behind a scene
    /// of this many ops.
    written: (u64, u32),
    /// Built for the scene ops buffer and the two residual buffers it names.
    group: Option<([wgpu::Buffer; 3], wgpu::BindGroup)>,
}

impl Bindings {
    /// Bindings for the 8-bit texture `canvas`.
    pub(crate) fn new(gpu: &Gpu, canvas: &wgpu::Texture) -> Self {
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        Self {
            size: [canvas.width(), canvas.height()],
            canvas: canvas.create_view(&Default::default()),
            params: empty_buffer(gpu, uniform, size_of::<Params>() as u64),
            lists: [0; 2].map(|_| empty_buffer(gpu, storage, MIN_LIST_BYTES)),
            written: (0, 0),
            group: None,
        }
    }

    /// Bytes of its own buffers (S49).
    pub(crate) fn bytes(&self) -> u64 {
        let lists: u64 = self.lists.iter().map(wgpu::Buffer::size).sum();
        lists + self.params.size()
    }

    /// Writes `records` into list `slot`, growing it (and dropping the bind group) if needed.
    fn write<R: Record>(&mut self, gpu: &Gpu, slot: usize, records: &[R]) {
        let bytes = encode(records);
        let size = bytes.len() as u64;
        if size > self.lists[slot].size() {
            let usage = self.lists[slot].usage();
            self.lists[slot] = empty_buffer(gpu, usage, size.next_power_of_two());
            self.group = None;
        }
        if size > 0 {
            gpu.queue.write_buffer(&self.lists[slot], 0, &bytes);
        }
    }
}

/// Smallest list buffer: room for 128 ops.
const MIN_LIST_BYTES: u64 = 128 * size_of::<Op>() as u64;

fn empty_buffer(gpu: &Gpu, usage: wgpu::BufferUsages, size: u64) -> wgpu::Buffer {
    gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("evaluate"),
        size,
        usage,
        mapped_at_creation: false,
    })
}

pub(crate) struct EvaluatePipeline {
    pipeline: wgpu::ComputePipeline,
    /// Without the moved selections' stack (`Plan::deepest` 0).
    flat: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    /// The paper's mip chain (`paper.rs`), then the shape textures' (`shapes.rs`), uploaded once;
    /// the raster core binds it too.
    pub(crate) textures: wgpu::Buffer,
    /// The most work one dispatch takes (`bounded.rs`; tests change it).
    bound: AtomicU64,
    /// Dispatches submitted so far.
    runs: AtomicU64,
    /// What evaluations did, counted (S53).
    pub(crate) tally: Tally,
}

impl EvaluatePipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let textures = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("textures"),
            contents: &encode(&[paper_levels(), shapes()].concat()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("evaluate"),
            source: wgpu::ShaderSource::Wgsl(shader_source().into()),
        });
        let layout = evaluate_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("evaluate"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        // The same evaluator twice: with the stack of what moved selections set aside, and
        // (for plans that set nothing aside) without it, which keeps more pixels in flight.
        let [pipeline, flat] = [1.0, 0.0].map(|stacked| {
            let constants = [("STACKED", stacked)];
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("evaluate"),
                layout: Some(&pipeline_layout),
                module: &module,
                entry_point: Some("evaluate"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                cache: None,
            })
        });
        Self {
            pipeline,
            flat,
            layout,
            textures,
            bound: AtomicU64::new(DISPATCH_WORK),
            runs: AtomicU64::new(0),
            tally: Tally::default(),
        }
    }

    fn bind_group(
        &self,
        gpu: &Gpu,
        bindings: &Bindings,
        scene: &SceneOps,
        [ops, arena, active]: [&wgpu::Buffer; 3],
    ) -> wgpu::BindGroup {
        let [tiles, indices] = &bindings.lists;
        let buffers = [
            (0, &bindings.params),
            (1, ops),
            (2, tiles),
            (3, indices),
            (7, &scene.masks),
            (9, &self.textures),
            (10, &scene.patches),
            (11, arena),
            (12, active),
        ];
        let mut entries: Vec<_> = buffers
            .into_iter()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::TextureView(&bindings.canvas),
        });
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("evaluate"),
            layout: &self.layout,
            entries: &entries,
        })
    }
}
