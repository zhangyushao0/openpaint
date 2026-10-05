//! A check of the screen's precision for the eye (S37, reached only from debug builds): a soft
//! grey ramp from sRGB 100 to 120 across the whole screen, over everything; its upper half
//! rounded to 8 bits, its lower half as the window holds it (ten bits on the tablet). Where ten
//! bits reach the panel the lower half shows four times as many, narrower bands.

const SHADER: &str = r"
struct Out {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> Out {
    // (0, 0), (2, 0), (0, 2) in 0..1 of the viewport: a triangle that holds all of it.
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return Out(vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0), uv);
}

@fragment
fn fragment(in: Out) -> @location(0) vec4<f32> {
    var value = (100.0 + 20.0 * clamp(in.uv.x, 0.0, 1.0)) / 255.0;
    if in.uv.y < 0.5 {
        value = round(value * 255.0) / 255.0;
    }
    return vec4<f32>(vec3<f32>(value), 1.0);
}
";

pub(crate) struct Gradient {
    pipeline: wgpu::RenderPipeline,
}

impl Gradient {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gradient"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gradient"),
            layout: None,
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

    /// Covers the whole of `pass`'s target.
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.draw(0..3, 0..1);
    }
}
