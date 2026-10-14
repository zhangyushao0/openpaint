//! The evaluator's compute pipeline and the bindings a plan is dispatched through
//! (`program_dispatch.rs`: one evaluation in bounded runs of tiles, S49).

use std::sync::atomic::AtomicU64;

use wgpu::util::DeviceExt;

use crate::affine::Affine;
use crate::bounded::DISPATCH_WORK;
use crate::gpu::{Gpu, TEN_BITS};
use crate::gpu_clock::{GpuClock, Timing};
use crate::output::Output;
use crate::paper::paper_levels;
use crate::plan::Plan;
use crate::protocol::{Op, Params, Record, TileRecord, encode};
use crate::readback::FORMAT;
use crate::shapes::shapes;
use crate::tally::Tally;

#[path = "program_scene.rs"]
mod scene_ops;
pub(crate) use scene_ops::{SceneOps, scene_ops};

#[path = "program_layout.rs"]
mod layout;
pub(crate) use layout::shader_source;
use layout::{evaluate_layout, unwritten};

#[path = "program_dispatch.rs"]
mod dispatch;

/// The in-progress stroke's coverage (or density) and paper contact textures: one float per
/// canvas pixel each.
pub(crate) const COVERAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

/// What one dispatch evaluates.
pub(crate) struct Dispatch<'a> {
    /// The scene's ops and its fill masks (`scene_ops`), uploaded once per scene.
    pub(crate) ops: &'a SceneOps,
    /// The in-progress stroke's ops for this evaluation.
    pub(crate) fresh: &'a [Op],
    pub(crate) plan: &'a Plan,
    /// Only these of its tiles (a preview's, near the layer's content, S35); `None`: all.
    pub(crate) only: Option<&'a [TileRecord]>,
    /// Where the target lies in the output (`Output::CANVAS` for the canvas itself).
    pub(crate) output: Output,
    pub(crate) background: [f32; 4],
    /// Clears the coverage texture first: the in-progress stroke starts over.
    pub(crate) reset: bool,
    /// The smudge arena and the in-progress smudge's residual (`Smudges::residuals`).
    pub(crate) residuals: [&'a wgpu::Buffer; 2],
    /// Where the target's pixel centres lie on the canvas: the identity, but for a smudge's
    /// material seen from a transform record (S31).
    pub(crate) base: Affine,
    /// A previewed layer's snapshot (S35): read by `OP_RESUME`, or written instead of the
    /// target.
    pub(crate) snapshot: Option<SnapshotUse<'a>>,
    /// Background work's GPU time measured by these timestamps (S53, `gpu_clock.rs`).
    pub(crate) timing: Option<(&'a GpuClock, Timing)>,
}

/// A snapshot's buffer (`snapshot.rs`), the canvas's tiles per row and the four-float units its
/// table takes, and whether the dispatch writes it.
#[derive(Clone, Copy)]
pub(crate) struct SnapshotUse<'a> {
    pub(crate) buffer: &'a wgpu::Buffer,
    pub(crate) layout: [u32; 2],
    pub(crate) writes: bool,
}

/// One canvas target's evaluation resources, kept between dispatches: buffers are rewritten
/// in place and the bind group is rebuilt only when a buffer grows or the scene's ops change.
/// A bind group per frame would cycle Vulkan descriptor pools, a periodic multi-millisecond
/// stall.
pub(crate) struct Bindings {
    size: [u32; 2],
    /// It keeps the in-progress stroke's coverage, in kept textures of its size.
    pub(crate) keeps: bool,
    canvas: wgpu::TextureView,
    /// The target is a ten-bit display texture (S37), written at binding 14 instead of 4.
    display: bool,
    /// The in-progress stroke's coverage or density, then its paper contact.
    kept: [(wgpu::Texture, wgpu::TextureView); 2],
    params: wgpu::Buffer,
    /// Tiles and op entries, in binding order 2, 3 (fresh ops follow the scene's, S55).
    lists: [wgpu::Buffer; 2],
    /// The plan whose entries the list holds (`Plan::serial`; 0: none), placed behind a scene
    /// of this many ops.
    written: (u64, u32),
    /// Built for the scene ops buffer, the two residual buffers and the snapshot it names.
    group: Option<([wgpu::Buffer; 4], wgpu::BindGroup)>,
    /// Bytes a snapshot of this target may grow to (S49: what the GPU memory budget leaves).
    pub(crate) snapshot_room: u64,
    /// Its dispatches are a worker's: small runs, each waited for (S53, `pace_dispatches`).
    pub(crate) paced: bool,
}

impl Bindings {
    /// Bindings for `canvas`, with the in-progress stroke's two kept textures of its size.
    pub(crate) fn new(gpu: &Gpu, canvas: &wgpu::Texture) -> Self {
        Self::with_kept(gpu, canvas, true)
    }

    /// Bindings for a target that keeps no in-progress stroke (an export piece, a view tier).
    pub(crate) fn without_kept(gpu: &Gpu, canvas: &wgpu::Texture) -> Self {
        Self::with_kept(gpu, canvas, false)
    }

    fn with_kept(gpu: &Gpu, canvas: &wgpu::Texture, keeps: bool) -> Self {
        let storage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let uniform = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        let size = [canvas.width(), canvas.height()];
        Self {
            size,
            keeps,
            canvas: canvas.create_view(&Default::default()),
            display: canvas.format() == TEN_BITS,
            kept: ["active coverage", "active contact"].map(|label| {
                let texture = kept_texture(gpu, if keeps { size } else { [1, 1] }, label);
                let view = texture.create_view(&Default::default());
                (texture, view)
            }),
            params: empty_buffer(gpu, uniform, size_of::<Params>() as u64),
            lists: [0; 2].map(|_| empty_buffer(gpu, storage, MIN_LIST_BYTES)),
            written: (0, 0),
            group: None,
            snapshot_room: u64::MAX,
            paced: false,
        }
    }

    /// Bytes of its own buffers and kept textures (S49).
    pub(crate) fn bytes(&self) -> u64 {
        let kept: u64 = self
            .kept
            .iter()
            .map(|(texture, _)| u64::from(texture.width()) * u64::from(texture.height()) * 4)
            .sum();
        let lists: u64 = self.lists.iter().map(wgpu::Buffer::size).sum();
        kept + lists + self.params.size()
    }

    /// The target's size in pixels.
    pub(crate) const fn size(&self) -> [u32; 2] {
        self.size
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

/// One float per canvas pixel that the evaluator reads and writes.
fn kept_texture(gpu: &Gpu, [width, height]: [u32; 2], label: &str) -> wgpu::Texture {
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COVERAGE_FORMAT,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

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
    /// The paper's mip chain (`paper.rs`), then the shape textures' (`shapes.rs`), uploaded once.
    pub(crate) textures: wgpu::Buffer,
    /// Bound as the snapshot when a dispatch has none.
    no_snapshot: wgpu::Buffer,
    /// One-texel stand-ins bound where a target does not write: at binding 4 (8 bits) and 14
    /// (the display format).
    unwritten: [wgpu::TextureView; 2],
    /// The most work one dispatch takes (`bounded.rs`; tests change it).
    bound: AtomicU64,
    /// Dispatches submitted so far.
    runs: AtomicU64,
    /// What evaluations did, counted (S53).
    pub(crate) tally: Tally,
}

impl EvaluatePipeline {
    pub(crate) fn new(device: &wgpu::Device, display: wgpu::TextureFormat) -> Self {
        let textures = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("textures"),
            contents: &encode(&[paper_levels(), shapes()].concat()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("evaluate"),
            source: wgpu::ShaderSource::Wgsl(shader_source(display).into()),
        });
        let layout = evaluate_layout(device, display);
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
        let no_snapshot = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("no snapshot"),
            size: 16,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            flat,
            layout,
            textures,
            no_snapshot,
            unwritten: [FORMAT, display].map(|format| unwritten(device, format)),
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
        [ops, arena, active, snapshot]: [&wgpu::Buffer; 4],
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
            (13, snapshot),
        ];
        let mut entries: Vec<_> = buffers
            .into_iter()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        let [(_, coverage), (_, contact)] = &bindings.kept;
        let [eight, display] = &self.unwritten;
        let (eight, display) = if bindings.display {
            (eight, &bindings.canvas)
        } else {
            (&bindings.canvas, display)
        };
        let views = [(4, eight), (6, coverage), (8, contact), (14, display)];
        for (binding, view) in views {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("evaluate"),
            layout: &self.layout,
            entries: &entries,
        })
    }
}
