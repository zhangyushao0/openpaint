//! The evaluator's compute pipeline and one dispatch of a plan into a target.

use wgpu::util::DeviceExt;

use crate::affine::Affine;
use crate::edits::patch_bytes;
use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::output::Output;
use crate::paper::paper_levels;
use crate::plan::Plan;
use crate::protocol::{MOVE_DEPTH, Op, Params, Record, encode, prelude};
use crate::readback::FORMAT;
use crate::scene::Scene;

#[path = "program_layout.rs"]
mod layout;
use layout::{buffer_entry, texture_entry, workgroups};

const SHADER: &str = include_str!("shaders/evaluate.wgsl");
const SEGMENT: &str = include_str!("shaders/segment.wgsl");
const DEPOSIT: &str = include_str!("shaders/deposit.wgsl");
const SMUDGE: &str = include_str!("shaders/smudge_composite.wgsl");
const ADJUST: &str = include_str!("shaders/adjust.wgsl");
const FILL_EDGE: &str = include_str!("shaders/fill_edge.wgsl");
const FILTER: &str = include_str!("shaders/filter_composite.wgsl");
/// The in-progress stroke's coverage (or density) and paper contact textures: one float per
/// canvas pixel each.
pub(crate) const COVERAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

/// The protocol prelude followed by the evaluator, its curve spans, its accumulating deposits,
/// its smudges, its adjustments, its fill edges and its filters.
pub(crate) fn shader_source() -> String {
    prelude() + SHADER + SEGMENT + DEPOSIT + SMUDGE + ADJUST + FILL_EDGE + FILTER
}

/// What one dispatch evaluates.
pub(crate) struct Dispatch<'a> {
    /// The scene's ops and its fill masks (`scene_ops`), uploaded once per scene.
    pub(crate) ops: &'a SceneOps,
    /// The in-progress stroke's ops for this evaluation.
    pub(crate) fresh: &'a [Op],
    pub(crate) plan: &'a Plan,
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
    /// The in-progress stroke's coverage or density, then its paper contact.
    kept: [(wgpu::Texture, wgpu::TextureView); 2],
    params: wgpu::Buffer,
    /// Tiles, indices and fresh ops, in binding order 2, 3, 5.
    lists: [wgpu::Buffer; 3],
    /// Built for the scene ops buffer and the two residual buffers it names.
    group: Option<([wgpu::Buffer; 3], wgpu::BindGroup)>,
}

/// One scene on the GPU: its ops (item index = op index), its fill masks and its transforms'
/// patches.
pub(crate) struct SceneOps {
    pub(crate) ops: wgpu::Buffer,
    pub(crate) masks: wgpu::Buffer,
    pub(crate) patches: wgpu::Buffer,
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
            kept: ["active coverage", "active contact"].map(|label| {
                let texture = kept_texture(gpu, if keeps { size } else { [1, 1] }, label);
                let view = texture.create_view(&Default::default());
                (texture, view)
            }),
            params: empty_buffer(gpu, uniform, size_of::<Params>() as u64),
            lists: [0; 3].map(|_| empty_buffer(gpu, storage, MIN_LIST_BYTES)),
            group: None,
        }
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
    layout: wgpu::BindGroupLayout,
    /// The paper's mip chain (`paper.rs`), uploaded once.
    paper: wgpu::Buffer,
}

impl EvaluatePipeline {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let paper = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("paper"),
            contents: &encode(&paper_levels()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("evaluate"),
            source: wgpu::ShaderSource::Wgsl(shader_source().into()),
        });
        let read_only = wgpu::BufferBindingType::Storage { read_only: true };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("evaluate"),
            entries: &[
                buffer_entry(0, wgpu::BufferBindingType::Uniform),
                buffer_entry(1, read_only),
                buffer_entry(2, read_only),
                buffer_entry(3, read_only),
                texture_entry(4, wgpu::StorageTextureAccess::WriteOnly, FORMAT),
                buffer_entry(5, read_only),
                texture_entry(6, wgpu::StorageTextureAccess::ReadWrite, COVERAGE_FORMAT),
                buffer_entry(7, read_only),
                texture_entry(8, wgpu::StorageTextureAccess::ReadWrite, COVERAGE_FORMAT),
                buffer_entry(9, read_only),
                buffer_entry(10, read_only),
                buffer_entry(11, read_only),
                buffer_entry(12, read_only),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("evaluate"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("evaluate"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("evaluate"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            pipeline,
            layout,
            paper,
        }
    }

    /// Writes the fresh ops and the plan, then evaluates every plan tile into the canvas.
    pub(crate) fn dispatch(
        &self,
        gpu: &Gpu,
        bindings: &mut Bindings,
        work: &Dispatch<'_>,
    ) -> Result<(), RenderError> {
        if work.plan.deepest > MOVE_DEPTH {
            return Err(RenderError::Invalid(
                "moved selections lie over one another deeper than the evaluator holds",
            ));
        }
        let groups = workgroups(gpu, work.plan.tiles.len())?;
        let (scale, inverse) = work.output.operands();
        let [a, b, c, d, e, f] = work.base.rows();
        let (foot, level) = if work.base.is_identity() {
            (1.0, 0.0)
        } else {
            let canvas = work.base.scale();
            ((1.0 / canvas) as f32, -canvas.log2() as f32)
        };
        let params = Params {
            background: work.background,
            tiles: groups.tiles,
            row: groups.row,
            size: bindings.size,
            kept: u32::from(bindings.keeps),
            level: scale.log2(),
            scale,
            inverse,
            base_x: [a, b, c, foot],
            base_y: [d, e, f, level],
        };
        gpu.queue
            .write_buffer(&bindings.params, 0, &encode(&[params]));
        bindings.write(gpu, 0, &work.plan.tiles);
        bindings.write(gpu, 1, &work.plan.indices);
        bindings.write(gpu, 2, work.fresh);
        let [arena, active] = work.residuals;
        let named = [&work.ops.ops, arena, active];
        if bindings
            .group
            .as_ref()
            .is_none_or(|(built, _)| built.iter().zip(named).any(|(a, b)| a != b))
        {
            let group = self.bind_group(gpu, bindings, work.ops, work.residuals);
            bindings.group = Some((named.map(Clone::clone), group));
        }
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        if work.reset {
            for (_, view) in &bindings.kept {
                clear_kept(&mut encoder, view);
            }
        }
        if let Some((_, group)) = &bindings.group {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(groups.row, groups.rows, 1);
        }
        gpu.queue.submit([encoder.finish()]);
        Ok(())
    }

    fn bind_group(
        &self,
        gpu: &Gpu,
        bindings: &Bindings,
        scene: &SceneOps,
        [arena, active]: [&wgpu::Buffer; 2],
    ) -> wgpu::BindGroup {
        let [tiles, indices, fresh] = &bindings.lists;
        let buffers = [
            (0, &bindings.params),
            (1, &scene.ops),
            (2, tiles),
            (3, indices),
            (5, fresh),
            (7, &scene.masks),
            (9, &self.paper),
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
        let [(_, coverage), (_, contact)] = &bindings.kept;
        for (binding, view) in [(4, &bindings.canvas), (6, coverage), (8, contact)] {
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

/// The scene's ops (item index = op index), fill masks and patches as GPU buffers; `patches`
/// is a buffer that already holds this scene's patch words (the scene before had the same
/// patches), so a transform being dragged uploads its pixels once.
pub(crate) fn scene_ops(gpu: &Gpu, scene: &Scene, patches: Option<wgpu::Buffer>) -> SceneOps {
    let ops: Vec<Op> = scene.items.iter().map(|item| item.op()).collect();
    let patches = patches.unwrap_or_else(|| {
        let mut bytes = patch_bytes(scene);
        if bytes.is_empty() {
            bytes.resize(4, 0);
        }
        gpu.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("patches"),
                contents: &bytes,
                usage: wgpu::BufferUsages::STORAGE,
            })
    });
    // The smudges' base words are written after their residuals are placed.
    let masks = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
    SceneOps {
        ops: buffer(gpu, wgpu::BufferUsages::STORAGE, &ops),
        masks: buffer(gpu, masks, &scene.masks),
        patches,
    }
}

/// Zero everywhere, through a render pass: clearing a texture directly needs an optional
/// device feature.
fn clear_kept(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear kept"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    drop(pass);
}

/// A buffer holding `records`. An empty list still gets one zeroed record, because a binding
/// must hold at least one element; no tile refers to it.
fn buffer<R: Record>(gpu: &Gpu, usage: wgpu::BufferUsages, records: &[R]) -> wgpu::Buffer {
    let mut contents = encode(records);
    if contents.is_empty() {
        contents.resize(size_of::<R>(), 0);
    }
    gpu.device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("evaluate"),
            contents: &contents,
            usage,
        })
}
