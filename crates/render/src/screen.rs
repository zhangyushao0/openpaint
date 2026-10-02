//! Putting the canvas on screen: `CanvasBlit` draws a canvas target into a window or offscreen
//! frame at one screen pixel per canvas pixel, and `ScreenTarget` is the offscreen frame the
//! harness draws into instead of a window.

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::image::RgbaImage;
use crate::readback::{offscreen_texture, read_rgba};

const SHADER: &str = include_str!("shaders/blit.wgsl");

/// Where a canvas target shows on screen: its top-left corner in screen pixels. The canvas may
/// reach past the screen's edges; only the overlap shows.
#[derive(Clone, Copy)]
pub struct CanvasPlacement<'a> {
    pub target: &'a CanvasTarget,
    pub origin: [i32; 2],
}

/// The blit's one binding: the canvas texture. Shared by every frame format, so each canvas
/// target binds its texture once (`blit_bind_group`).
pub(crate) fn blit_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("blit"),
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
    })
}

/// Binds a canvas texture for the blit.
pub(crate) fn blit_bind_group(core: &Core, texture: &wgpu::Texture) -> wgpu::BindGroup {
    let view = texture.create_view(&Default::default());
    core.gpu
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit"),
            layout: &core.blit_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        })
}

/// The blit pipeline for one frame format.
pub(crate) struct CanvasBlit {
    pipeline: wgpu::RenderPipeline,
}

impl CanvasBlit {
    pub(crate) fn new(core: &Core, format: wgpu::TextureFormat) -> Self {
        let device = &core.gpu.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blit"),
            bind_group_layouts: &[Some(&core.blit_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&pipeline_layout),
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
        });
        Self { pipeline }
    }

    /// Draws the placed canvas into `pass`, whose target is the screen.
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, placement: &CanvasPlacement<'_>) {
        let [x, y] = placement.origin.map(|side| side as f32);
        let [width, height] = placement.target.size().map(|side| side as f32);
        pass.set_viewport(x, y, width, height, 0.0, 1.0);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, placement.target.shown(), &[]);
        pass.draw(0..3, 0..1);
    }
}

/// An offscreen frame in place of a window: drawn by `Core::render_screen`, read back on
/// demand.
pub struct ScreenTarget {
    texture: wgpu::Texture,
}

impl ScreenTarget {
    pub fn new(core: &Core, size: [u32; 2]) -> Result<Self, RenderError> {
        Ok(Self {
            texture: offscreen_texture(&core.gpu, size)?,
        })
    }

    /// `[width, height]` in pixels.
    #[must_use]
    pub fn size(&self) -> [u32; 2] {
        [self.texture.width(), self.texture.height()]
    }

    /// Waits for every submitted frame and copies the target into memory.
    pub fn read(&self, core: &Core) -> Result<RgbaImage, RenderError> {
        read_rgba(&core.gpu, &self.texture)
    }

    pub(crate) fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}
