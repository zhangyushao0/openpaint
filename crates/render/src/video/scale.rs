//! The video's scale pass on the GPU (`shaders/video.wgsl`, the module's rule): a canvas target
//! scaled to a size by area, in integers, into a buffer read back as RGBA8.

use wgpu::util::DeviceExt;

use crate::canvas::CanvasTarget;
use crate::core::Core;
use crate::error::RenderError;
use crate::image::RgbaImage;
use crate::readback::read_buffer;

const SHADER: &str = include_str!("../shaders/video.wgsl");
/// The shader's workgroup side.
const GROUP: u32 = 8;

/// The scale pass's pipeline.
pub(super) struct ScalePass {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

/// One scale of a canvas target to `size`: its sizes, the pixels it writes and its bind group.
pub(super) struct Bindings {
    size: [u32; 2],
    pixels: wgpu::Buffer,
    group: wgpu::BindGroup,
}

impl ScalePass {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("video scale"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let buffer = |ty| wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        let texture = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let storage = wgpu::BufferBindingType::Storage { read_only: false };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("video scale"),
            entries: &[
                entry(0, texture),
                entry(1, buffer(wgpu::BufferBindingType::Uniform)),
                entry(2, buffer(storage)),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("video scale"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("video scale"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("scale"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layout }
    }

    /// The bindings scaling `target` to `size`.
    pub(super) fn bindings(
        &self,
        core: &Core,
        target: &CanvasTarget,
        size: [u32; 2],
    ) -> Result<Bindings, RenderError> {
        let device = &core.gpu.device;
        let limits = device.limits();
        let bytes = u64::from(size[0]) * u64::from(size[1]) * 4;
        if size.contains(&0) {
            return Err(RenderError::Invalid("a video frame has a zero side"));
        }
        if bytes
            > limits
                .max_storage_buffer_binding_size
                .min(limits.max_buffer_size)
        {
            return Err(RenderError::Invalid(
                "a video frame beyond the device's buffers",
            ));
        }
        let words: Vec<u8> = [target.size(), size]
            .concat()
            .iter()
            .flat_map(|side| side.to_le_bytes())
            .collect();
        let sizes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("video scale sizes"),
            contents: &words,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let pixels = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("video frame"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let view = target.texture().create_view(&Default::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("video scale"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sizes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: pixels.as_entire_binding(),
                },
            ],
        });
        Ok(Bindings {
            size,
            pixels,
            group,
        })
    }

    /// The canvas `bindings` scale, scaled and read back (waiting for the draws before).
    pub(super) fn run(&self, core: &Core, bindings: &Bindings) -> Result<RgbaImage, RenderError> {
        let [width, height] = bindings.size;
        let mut encoder = core.gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings.group, &[]);
            pass.dispatch_workgroups(width.div_ceil(GROUP), height.div_ceil(GROUP), 1);
        }
        core.gpu.submit([encoder.finish()]);
        let bytes = u64::from(width) * u64::from(height) * 4;
        let data = read_buffer(&core.gpu, &bindings.pixels, bytes)?;
        Ok(RgbaImage::new(bindings.size, data))
    }
}
