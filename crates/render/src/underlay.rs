//! The gallery picture enlarged into a canvas target (S54, DECISIONS D81): what a canvas opening
//! without a display cache shows under the tiles still waiting, in place of the paper. One
//! dispatch; the pipeline is made on first use, for the device's display format.

use wgpu::util::DeviceExt;

use crate::core::Core;
use crate::error::RenderError;
use crate::gpu::TEN_BITS;
use crate::image::RgbaImage;

const SHADER: &str = include_str!("shaders/underlay.wgsl");
/// The shader's workgroup side.
const GROUP: u32 = 8;

pub(crate) struct UnderlayPipeline {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

impl UnderlayPipeline {
    /// For targets in `format` (8-bit, or the ten-bit display format).
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let texel = if format == TEN_BITS {
            "rgb10a2unorm"
        } else {
            "rgba8unorm"
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("underlay"),
            source: wgpu::ShaderSource::Wgsl(SHADER.replace("TEXEL_FORMAT", texel).into()),
        });
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let picture = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let sampler = wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering);
        let canvas = wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("underlay"),
            entries: &[entry(0, picture), entry(1, sampler), entry(2, canvas)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("underlay"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("underlay"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("underlay"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layout }
    }
}

impl Core {
    /// Writes `picture` (8-bit RGBA, opaque) enlarged bilinearly over the whole of `texture`, a
    /// screen canvas target in the display format. Submitted, not waited for.
    pub(crate) fn enlarge_into(
        &self,
        picture: &RgbaImage,
        texture: &wgpu::Texture,
    ) -> Result<(), RenderError> {
        if texture.format() != self.gpu.display || picture.size().contains(&0) {
            return Err(RenderError::Invalid(
                "only a screen canvas target takes the picture",
            ));
        }
        let underlay = self
            .underlay
            .get_or_init(|| UnderlayPipeline::new(&self.gpu.device, self.gpu.display));
        let device = &self.gpu.device;
        let [width, height] = picture.size();
        let source = device.create_texture_with_data(
            &self.gpu.queue,
            &wgpu::TextureDescriptor {
                label: Some("underlay picture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            picture.pixels(),
        );
        let views = [&source, texture].map(|texture| texture.create_view(&Default::default()));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("underlay"),
            layout: &underlay.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&views[1]),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&underlay.pipeline);
            pass.set_bind_group(0, &group, &[]);
            let [x, y] = [texture.width(), texture.height()].map(|side| side.div_ceil(GROUP));
            pass.dispatch_workgroups(x, y, 1);
        }
        self.gpu.submit([encoder.finish()]);
        Ok(())
    }
}
