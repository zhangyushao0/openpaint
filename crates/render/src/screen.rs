//! Putting the canvas on screen: `CanvasBlit` draws a canvas target into a window or offscreen
//! frame through the view (scale, rotation, pan), and `ScreenTarget` is the offscreen frame the
//! harness draws into instead of a window. The target itself stays at canvas resolution; the
//! screen samples it bilinearly, without mipmaps (D35).

use crate::core::Core;
use crate::error::RenderError;
use crate::evaluator::CanvasTarget;
use crate::image::RgbaImage;
use crate::readback::{offscreen_texture, read_rgba};

const SHADER: &str = include_str!("shaders/blit.wgsl");
/// Bytes of the view uniform: four rows of four floats (the map's two rows, the cursor, the
/// surround).
pub(crate) const VIEW_BYTES: u64 = 64;

/// Where a canvas target shows on screen: the canvas area it is clipped to (top-left corner and
/// size in screen pixels), the view's map from screen pixels to canvas pixels, as the rows
/// `[a, b, c, d, e, f]` of `canvas = (a x + b y + c, d x + e y + f)`, and the pen's hover
/// outline, if it shows.
#[derive(Clone, Copy)]
pub struct CanvasPlacement<'a> {
    pub target: &'a CanvasTarget,
    pub origin: [i32; 2],
    pub size: [u32; 2],
    pub to_canvas: [f32; 6],
    pub cursor: Option<BrushCursor>,
}

/// The pen's hover outline: a circle around `centre` of `radius`, `width` wide, all in screen
/// pixels; each of its pixels black or white, whichever stands out from what lies under it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrushCursor {
    pub centre: [f32; 2],
    pub radius: f32,
    pub width: f32,
}

/// The blit's bindings: the canvas texture, the sampler and the view. Shared by every frame
/// format, so each canvas target binds them once (`blit_bind_group`).
pub(crate) fn blit_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty,
        count: None,
    };
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("blit"),
        entries: &[
            entry(
                0,
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            ),
            entry(
                1,
                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            ),
            entry(
                2,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(VIEW_BYTES),
                },
            ),
        ],
    })
}

/// The screen's sampler: bilinear, clamped at the canvas's edges.
pub(crate) fn blit_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("blit"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

/// Binds a canvas texture and its view uniform for the blit.
pub(crate) fn blit_bind_group(
    core: &Core,
    texture: &wgpu::Texture,
    view: &wgpu::Buffer,
) -> wgpu::BindGroup {
    let texture = texture.create_view(&Default::default());
    core.gpu
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit"),
            layout: &core.blit_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&core.blit_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: view.as_entire_binding(),
                },
            ],
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

    /// Draws the placed canvas into `pass`, whose target is the screen of `screen` pixels
    /// cleared to `surround`, clipped to the placement's area. Writes the view before the pass
    /// is submitted.
    pub(crate) fn draw(
        &self,
        core: &Core,
        pass: &mut wgpu::RenderPass<'_>,
        placement: &CanvasPlacement<'_>,
        (screen, surround): ([u32; 2], [u8; 4]),
    ) {
        let clip = |origin: i32, size: u32, limit: u32| {
            let start = origin.clamp(0, limit as i32) as u32;
            let end = (i64::from(origin) + i64::from(size)).clamp(0, i64::from(limit)) as u32;
            (start, end - start)
        };
        let (x, width) = clip(placement.origin[0], placement.size[0], screen[0]);
        let (y, height) = clip(placement.origin[1], placement.size[1], screen[1]);
        if width == 0 || height == 0 {
            return;
        }
        let [a, b, c, d, e, f] = placement.to_canvas;
        let cursor = placement.cursor.map_or([0.0; 4], |cursor| {
            let [x, y] = cursor.centre;
            [x, y, cursor.radius, cursor.width / 2.0]
        });
        let surround = surround.map(|channel| f32::from(channel) / 255.0);
        let rows = [[a, b, c, 0.0], [d, e, f, 0.0], cursor, surround];
        let bytes: Vec<u8> = rows
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        core.gpu
            .queue
            .write_buffer(placement.target.view_buffer(), 0, &bytes);
        pass.set_scissor_rect(x, y, width, height);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, placement.target.shown(), &[]);
        pass.draw(0..3, 0..1);
        pass.set_scissor_rect(0, 0, screen[0], screen[1]);
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
