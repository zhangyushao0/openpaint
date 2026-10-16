//! The raster core's fixed-function states (S59, `raster_gpu.rs`): the formats it draws in, how
//! the blend unit combines what each draw lays (the evaluator's equations, premultiplied), its
//! bind group layouts and how a pipeline is made from two entry points.

use crate::gpu::Gpu;
use crate::protocol::Blend;
use crate::raster_plan::EndBlend;

/// Bytes of one instance: four whole numbers, four signed ones (`raster_plan::Instance`).
pub(crate) const INSTANCE_BYTES: u64 = 32;

/// The atlas's and the float targets' formats: 32-bit floats where the device blends them, else
/// 16-bit ones.
pub(crate) fn floats(gpu: &Gpu) -> [wgpu::TextureFormat; 2] {
    use wgpu::TextureFormat::{R16Float, R32Float, Rgba16Float, Rgba32Float};
    [(R32Float, R16Float), (Rgba32Float, Rgba16Float)]
        .map(|(wide, narrow)| blends(gpu, wide, narrow))
}

/// `wide` where the device blends into it, else `narrow` (16-bit floats blend everywhere).
fn blends(
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

pub(crate) fn blended(
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
) -> wgpu::ColorTargetState {
    wgpu::ColorTargetState {
        format,
        blend,
        write_mask: wgpu::ColorWrites::ALL,
    }
}

/// The largest (max) or the sum (add) of what is there and what is laid.
pub(crate) fn summing(operation: wgpu::BlendOperation) -> wgpu::BlendState {
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

fn component(src_factor: wgpu::BlendFactor, dst_factor: wgpu::BlendFactor) -> wgpu::BlendComponent {
    wgpu::BlendComponent {
        src_factor,
        dst_factor,
        operation: wgpu::BlendOperation::Add,
    }
}

/// The alpha of what lies under kept: a clipping chain's, and the opaque canvas's.
fn kept_alpha(color: wgpu::BlendComponent) -> wgpu::BlendState {
    use wgpu::BlendFactor::{One, Zero};
    wgpu::BlendState {
        color,
        alpha: component(Zero, One),
    }
}

/// A paint op's blend with the layer (`blend` in evaluate.wgsl), premultiplied: over; atop
/// (`paint x layer alpha + layer x (1 - paint alpha)`, the alpha kept); erase.
pub(crate) fn painted(blend: Blend) -> wgpu::BlendState {
    use wgpu::BlendFactor::{DstAlpha, OneMinusSrcAlpha, Zero};
    match blend {
        Blend::Over => wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        Blend::Atop => kept_alpha(component(DstAlpha, OneMinusSrcAlpha)),
        Blend::Erase => wgpu::BlendState {
            color: component(Zero, OneMinusSrcAlpha),
            alpha: component(Zero, OneMinusSrcAlpha),
        },
    }
}

/// A layer end's fragment entry and blend (`composite` and `clipped` in evaluate.wgsl with
/// `s` the layer as shown at its opacity, `d` what lies under it): onto the opaque canvas, over
/// is `s + d (1 - sa)`, Multiply `d (1 - sa + s)`, Screen `s + d (1 - s)`; into a chain (its
/// alpha kept), Normal is atop, `s da + d (1 - sa)`, Multiply the same `d (1 - sa + s)`, Screen
/// `s da + d (1 - s)`; a base and Overlay (from a copy of what lies under it) are written whole.
pub(crate) fn ended(end: EndBlend) -> (&'static str, Option<wgpu::BlendState>) {
    use wgpu::BlendFactor::{DstAlpha, One, OneMinusSrc, OneMinusSrcAlpha, Src, Zero};
    match end {
        EndBlend::Over => (
            "end_fragment",
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        ),
        EndBlend::Multiply => ("multiply_fragment", Some(kept_alpha(component(Zero, Src)))),
        EndBlend::ScreenCanvas => (
            "end_fragment",
            Some(kept_alpha(component(One, OneMinusSrc))),
        ),
        EndBlend::Atop => (
            "end_fragment",
            Some(kept_alpha(component(DstAlpha, OneMinusSrcAlpha))),
        ),
        EndBlend::ScreenChain => (
            "end_fragment",
            Some(kept_alpha(component(DstAlpha, OneMinusSrc))),
        ),
        EndBlend::Base => ("base_fragment", None),
        EndBlend::Overlay => ("overlay_fragment", None),
    }
}

/// Group 0: the values, the ops, the masks, the textures, the patches; group 1: three float
/// textures read (the atlas, or a layer, its mask and a backdrop, or the canvas).
pub(crate) fn layouts(device: &wgpu::Device) -> [wgpu::BindGroupLayout; 2] {
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
        label: Some("raster scene"),
        entries: &[
            buffer(0, wgpu::BufferBindingType::Uniform),
            buffer(1, read),
            buffer(2, read),
            buffer(3, read),
            buffer(4, read),
        ],
    });
    let texture = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    };
    let reads = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("raster reads"),
        entries: &[texture(0), texture(1), texture(2)],
    });
    [scene, reads]
}

pub(crate) fn pipeline_layout(
    device: &wgpu::Device,
    groups: &[wgpu::BindGroupLayout],
) -> wgpu::PipelineLayout {
    let groups: Vec<Option<&wgpu::BindGroupLayout>> = groups.iter().map(Some).collect();
    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("raster core"),
        bind_group_layouts: &groups,
        immediate_size: 0,
    })
}

/// The instances: four whole numbers at location 0, four signed ones at 1.
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Uint32x4, 1 => Sint32x4];

/// A pipeline's module, layout and entries.
pub(crate) struct Stage<'a> {
    pub(crate) module: &'a wgpu::ShaderModule,
    pub(crate) layout: &'a wgpu::PipelineLayout,
    pub(crate) entries: [&'a str; 2],
}

impl Stage<'_> {
    /// Instanced quads as triangle strips (the finish draws one triangle).
    pub(crate) fn pipeline(
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
        let topology = if vertex == "finish_vertex" {
            wgpu::PrimitiveTopology::TriangleList
        } else {
            wgpu::PrimitiveTopology::TriangleStrip
        };
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(fragment),
            layout: Some(self.layout),
            vertex: wgpu::VertexState {
                module: self.module,
                entry_point: Some(vertex),
                compilation_options: Default::default(),
                buffers: &instanced,
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
