//! The raster probe's pipelines (S57, `raster.rs`): its shader (raster.wgsl, raster_stamp.wgsl,
//! raster_composite.wgsl, then the evaluator's own helpers, spans, stamps, textures and deposits),
//! one pipeline per way a stroke lays into the atlas (the blend unit keeping the largest or
//! summing), one per paint blend onto the layer, and the finish onto the paper. Each pipeline's
//! entry points reach only their own kind's code, so none pays for the others' registers (D82).

use crate::error::RenderError;
use crate::gpu::Gpu;
use crate::protocol::{Blend, prelude};
use crate::raster_plan::Coverage;
use crate::readback::FORMAT;

const RASTER: &str = include_str!("shaders/raster.wgsl");
const RASTER_STAMP: &str = include_str!("shaders/raster_stamp.wgsl");
const RASTER_COMPOSITE: &str = include_str!("shaders/raster_composite.wgsl");
const EVALUATE: &str = include_str!("shaders/evaluate.wgsl");
const SEGMENT: &str = include_str!("shaders/segment.wgsl");
const STAMP: &str = include_str!("shaders/stamp.wgsl");
const STAMP_TEXTURE: &str = include_str!("shaders/stamp_texture.wgsl");
const DEPOSIT: &str = include_str!("shaders/deposit.wgsl");

/// The helpers of evaluate.wgsl the probe's stages call, taken from it whole, so they stay its.
const HELPERS: [&str; 11] = [
    "fn word(",
    "fn stroke_paint(",
    "fn clip(",
    "fn paint_color(",
    "fn mask_bit(",
    "fn over(",
    "fn grain(",
    "const GRAIN_STEP",
    "fn hash(",
    "fn pcg(",
    "fn quantized(",
];

/// The probe's WGSL: the protocol prelude, its own stages, then the evaluator's functions.
pub(crate) fn raster_source() -> Result<String, RenderError> {
    let mut source = prelude() + RASTER + RASTER_STAMP + RASTER_COMPOSITE;
    for head in HELPERS {
        source.push_str(item(EVALUATE, head).ok_or(RenderError::Invalid(
            "evaluate.wgsl lacks a helper the raster probe takes",
        ))?);
    }
    Ok(source + SEGMENT + STAMP + STAMP_TEXTURE + DEPOSIT)
}

/// The declaration of `source` starting a line with `head`: a function through its closing
/// brace at the start of a line, a constant through its line.
fn item<'a>(source: &'a str, head: &str) -> Option<&'a str> {
    let start = source.find(&format!("\n{head}"))? + 1;
    let rest = &source[start..];
    let end = if head.starts_with("fn ") {
        rest.find("\n}\n")? + 3
    } else {
        rest.find('\n')? + 1
    };
    Some(&rest[..end])
}

/// Bytes of one instance: four whole numbers, four signed ones (`raster_plan::Instance`).
pub(crate) const INSTANCE_BYTES: u64 = 32;

pub(crate) struct RasterPipelines {
    /// The scene's buffers and the values (group 0); a texture read (group 1).
    pub(crate) layouts: [wgpu::BindGroupLayout; 2],
    /// By `Coverage`, in its order.
    coverage: [wgpu::RenderPipeline; 5],
    /// By `Blend`: over, atop, erase.
    composite: [wgpu::RenderPipeline; 3],
    pub(crate) finish: wgpu::RenderPipeline,
    /// The atlas's and the layer's formats: 32-bit floats where the device blends them.
    pub(crate) atlas: wgpu::TextureFormat,
    pub(crate) layer: wgpu::TextureFormat,
    /// How long making them took, ns (`Core::raster_pipelines`).
    pub(crate) compile_ns: f64,
}

impl RasterPipelines {
    pub(crate) fn new(gpu: &Gpu) -> Result<Self, RenderError> {
        let device = &gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raster probe"),
            source: wgpu::ShaderSource::Wgsl(raster_source()?.into()),
        });
        let layouts = layouts(device);
        let scene = pipeline_layout(device, &layouts[..1]);
        let both = pipeline_layout(device, &layouts);
        let atlas = floats(
            gpu,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::R16Float,
        );
        let layer = floats(
            gpu,
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureFormat::Rgba16Float,
        );
        let make = |(vertex, fragment): (&str, &str), layout, target: wgpu::ColorTargetState| {
            let stage = Stage {
                module: &module,
                layout,
                entries: [vertex, fragment],
            };
            stage.pipeline(device, target)
        };
        let coverage = Coverage::ALL.map(|coverage| {
            let (entries, operation) = coverage.stage();
            make(entries, &scene, blended(atlas, summing(operation)))
        });
        let composite = [Blend::Over, Blend::Atop, Blend::Erase].map(|blend| {
            let entries = ("composite_vertex", "composite_fragment");
            make(entries, &both, blended(layer, painted(blend)))
        });
        let finish = make(
            ("finish_vertex", "finish_fragment"),
            &both,
            wgpu::ColorTargetState::from(FORMAT),
        );
        Ok(Self {
            layouts,
            coverage,
            composite,
            finish,
            atlas,
            layer,
            compile_ns: 0.0,
        })
    }

    pub(crate) fn coverage(&self, coverage: Coverage) -> &wgpu::RenderPipeline {
        &self.coverage[coverage as usize]
    }

    pub(crate) fn composite(&self, blend: Blend) -> &wgpu::RenderPipeline {
        &self.composite[blend as usize]
    }
}

impl Coverage {
    const ALL: [Self; 5] = [
        Self::Cover,
        Self::Density,
        Self::Contact,
        Self::StampCover,
        Self::StampDensity,
    ];

    /// Its vertex and fragment entries, and how the blend unit combines what they lay.
    fn stage(self) -> ((&'static str, &'static str), wgpu::BlendOperation) {
        let (max, add) = (wgpu::BlendOperation::Max, wgpu::BlendOperation::Add);
        match self {
            Self::Cover => (("piece_vertex", "cover_fragment"), max),
            Self::Density => (("piece_vertex", "density_fragment"), add),
            Self::Contact => (("piece_vertex", "contact_fragment"), max),
            Self::StampCover => (("stamp_vertex", "stamp_fragment"), max),
            Self::StampDensity => (("stamp_vertex", "stamp_fragment"), add),
        }
    }
}

/// `wide` where the device blends into it, else `narrow` (16-bit floats blend everywhere).
fn floats(
    gpu: &Gpu,
    wide: wgpu::TextureFormat,
    narrow: wgpu::TextureFormat,
) -> wgpu::TextureFormat {
    let specific =
        (gpu.device.features()).contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
    let flags = gpu.adapter.get_texture_format_features(wide).flags;
    if specific && flags.contains(wgpu::TextureFormatFeatureFlags::BLENDABLE) {
        wide
    } else {
        narrow
    }
}

fn blended(format: wgpu::TextureFormat, blend: wgpu::BlendState) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend: Some(blend),
        write_mask: wgpu::ColorWrites::ALL,
    }
}

/// The largest (max) or the sum (add) of what is there and what is laid.
fn summing(operation: wgpu::BlendOperation) -> wgpu::BlendState {
    let component = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation,
    };
    wgpu::BlendState {
        color: component,
        alpha: component,
    }
}

/// A paint op's blend with the layer (`blend` in evaluate.wgsl), premultiplied: over; atop
/// (`paint x layer alpha + layer x (1 - paint alpha)`, the alpha kept); erase.
fn painted(blend: Blend) -> wgpu::BlendState {
    use wgpu::BlendFactor::{DstAlpha, One, OneMinusSrcAlpha, Zero};
    let component = |src_factor, dst_factor| wgpu::BlendComponent {
        src_factor,
        dst_factor,
        operation: wgpu::BlendOperation::Add,
    };
    match blend {
        Blend::Over => wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        Blend::Atop => wgpu::BlendState {
            color: component(DstAlpha, OneMinusSrcAlpha),
            alpha: component(Zero, One),
        },
        Blend::Erase => wgpu::BlendState {
            color: component(Zero, OneMinusSrcAlpha),
            alpha: component(Zero, OneMinusSrcAlpha),
        },
    }
}

/// Group 0: the values, the ops, the masks, the textures; group 1: one float texture.
fn layouts(device: &wgpu::Device) -> [wgpu::BindGroupLayout; 2] {
    let stages = wgpu::ShaderStages::VERTEX_FRAGMENT;
    let buffer = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: stages,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    };
    let read = wgpu::BufferBindingType::Storage { read_only: true };
    let scene = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("raster probe scene"),
        entries: &[
            buffer(0, wgpu::BufferBindingType::Uniform),
            buffer(1, read),
            buffer(2, read),
            buffer(3, read),
        ],
    });
    let source = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("raster probe source"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    });
    [scene, source]
}

fn pipeline_layout(
    device: &wgpu::Device,
    groups: &[wgpu::BindGroupLayout],
) -> wgpu::PipelineLayout {
    let groups: Vec<Option<&wgpu::BindGroupLayout>> = groups.iter().map(Some).collect();
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("raster probe"),
        bind_group_layouts: &groups,
        immediate_size: 0,
    })
}

/// The instances: four whole numbers at location 0, four signed ones at 1.
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Uint32x4, 1 => Sint32x4];

/// A pipeline's module, layout and entries.
struct Stage<'a> {
    module: &'a wgpu::ShaderModule,
    layout: &'a wgpu::PipelineLayout,
    entries: [&'a str; 2],
}

impl Stage<'_> {
    fn pipeline(
        &self,
        device: &wgpu::Device,
        target: wgpu::ColorTargetState,
    ) -> wgpu::RenderPipeline {
        let [vertex, fragment] = self.entries;
        let instanced = [Some(wgpu::VertexBufferLayout {
            array_stride: INSTANCE_BYTES,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES,
        })];
        let (buffers, topology): (&[Option<wgpu::VertexBufferLayout>], _) =
            if vertex == "finish_vertex" {
                (&[], wgpu::PrimitiveTopology::TriangleList)
            } else {
                (&instanced, wgpu::PrimitiveTopology::TriangleStrip)
            };
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(fragment),
            layout: Some(self.layout),
            vertex: wgpu::VertexState {
                module: self.module,
                entry_point: Some(vertex),
                compilation_options: Default::default(),
                buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: self.module,
                entry_point: Some(fragment),
                compilation_options: Default::default(),
                targets: &[Some(target)],
            }),
            primitive: wgpu::PrimitiveState {
                topology,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_probes_shader_validates_with_the_evaluators_helpers() {
        let source = raster_source().expect("every helper is found");
        let module = wgpu::naga::front::wgsl::parse_str(&source).expect("the probe's WGSL parses");
        let mut validator = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .expect("the probe's WGSL validates");
        assert_eq!(
            item("\nfn a(x) {\n  y\n}\nfn b", "fn a("),
            Some("fn a(x) {\n  y\n}\n")
        );
        assert_eq!(
            item("x\nconst K: u32 = 1u;\ny", "const K"),
            Some("const K: u32 = 1u;\n")
        );
    }
}
