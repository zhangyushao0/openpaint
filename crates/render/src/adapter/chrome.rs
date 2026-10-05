//! The chrome's paper drawn on the GPU (D44): one full-screen pass of `chrome.wgsl` inside the
//! UI's frame, through an egui-wgpu paint callback, so the islands and the panels growing out of
//! them are one signed distance field with one crisp ink outline. The UI says what to draw
//! (`ChromeShapes`); the pipeline lives with the screen painter, one per frame format.

use egui::{Color32, Pos2, Rect};

const SHADER: &str = include_str!("../shaders/chrome.wgsl");
/// At most this many islands carry chrome at once.
pub const CHROME_ISLANDS: usize = 4;
/// At most this many panels are drawn growing or shrinking at once; more keep the newest.
pub const CHROME_PANELS: usize = 3;
/// Smooth-union radius between an island and its panel, in points: a small fillet where the
/// panel crosses the island's edge, below the gap the panel's top steps over, so the two never
/// pull towards each other across a thin gap.
const JOIN: f32 = 4.0;
/// Vec4 rows of the uniform: screen, widths, three colours, islands and their corners, panels
/// and their shapes.
const ROWS: usize = 5 + 2 * CHROME_ISLANDS + 2 * CHROME_PANELS;

/// One panel growing out of an island, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromePanel {
    pub rect: Rect,
    pub radius: f32,
    /// 0 while its outline is the ink ring it grows from, 1 once it is the panel's rule.
    pub ring_to_rule: f32,
    /// Index of its island in `ChromeShapes::islands`.
    pub island: usize,
}

/// This frame's chrome: islands (rectangle and corner radius, in points), the panels growing
/// out of them (bottom first), the paper and the inks, and the line widths in points.
#[derive(Debug, Clone, PartialEq)]
pub struct ChromeShapes {
    pub islands: Vec<(Rect, f32)>,
    pub panels: Vec<ChromePanel>,
    pub paper: Color32,
    pub rule: Color32,
    pub active: Color32,
    pub rule_width: f32,
    pub ring_width: f32,
}

impl ChromeShapes {
    /// The uniform's rows; the density and the framebuffer's encoding are the painter's.
    fn rows(&self, pixels_per_point: f32, linear: bool, screen: Pos2) -> [[f32; 4]; ROWS] {
        let mut rows = [[0.0; 4]; ROWS];
        let colour = |c: Color32| [c.r(), c.g(), c.b(), c.a()].map(|v| f32::from(v) / 255.0);
        let rect = |r: Rect| {
            [
                r.center().x,
                r.center().y,
                r.width() / 2.0,
                r.height() / 2.0,
            ]
        };
        let islands = self.islands.len().min(CHROME_ISLANDS);
        let skip = self.panels.len().saturating_sub(CHROME_PANELS);
        let panels = &self.panels[skip..];
        rows[0] = [
            screen.x,
            screen.y,
            pixels_per_point,
            f32::from(u8::from(linear)),
        ];
        rows[1] = [
            self.rule_width,
            self.ring_width,
            islands as f32,
            panels.len() as f32,
        ];
        rows[2] = colour(self.paper);
        rows[3] = colour(self.rule);
        rows[4] = colour(self.active);
        for (k, (island, corner)) in self.islands.iter().take(islands).enumerate() {
            rows[5 + k] = rect(*island);
            rows[5 + CHROME_ISLANDS + k] = [*corner, 0.0, 0.0, 0.0];
        }
        let first = 5 + 2 * CHROME_ISLANDS;
        for (i, panel) in panels.iter().enumerate() {
            rows[first + i] = rect(panel.rect);
            let island = panel.island.min(CHROME_ISLANDS - 1) as f32;
            rows[first + CHROME_PANELS + i] = [panel.radius, JOIN, panel.ring_to_rule, island];
        }
        rows
    }
}

/// The pipeline and its uniform buffer, kept in egui-wgpu's callback resources.
struct Resources {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    linear: bool,
}

/// Creates the chrome pipeline for frames of `format` and hands it to `renderer`.
pub(crate) fn install(
    device: &wgpu::Device,
    renderer: &mut egui_wgpu::Renderer,
    format: wgpu::TextureFormat,
) {
    let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chrome"),
        size: (ROWS * 16) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("chrome"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new((ROWS * 16) as u64),
            },
            count: None,
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("chrome"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniforms.as_entire_binding(),
        }],
    });
    renderer.callback_resources.insert(Resources {
        pipeline: pipeline(device, &layout, format),
        uniforms,
        bind_group,
        linear: format.is_srgb(),
    });
}

/// The full-screen pass of `chrome.wgsl` into frames of `format`, blending as egui does.
fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chrome"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("chrome"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("chrome"),
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
            // egui's own blending: premultiplied alpha.
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// This frame's chrome as a paint callback.
struct Chrome {
    shapes: ChromeShapes,
    screen: Rect,
}

impl egui_wgpu::CallbackTrait for Chrome {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(res) = resources.get::<Resources>() {
            let size = self.screen.size().to_pos2();
            let rows = self.shapes.rows(screen.pixels_per_point, res.linear, size);
            let bytes: Vec<u8> = rows
                .iter()
                .flatten()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            queue.write_buffer(&res.uniforms, 0, &bytes);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(res) = resources.get::<Resources>() {
            pass.set_pipeline(&res.pipeline);
            pass.set_bind_group(0, &res.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}

/// The shape that draws `shapes` where the UI paints it; the pass runs only over `reach`
/// (points). A frame may hold several, each over its own reach, but all with the same shapes:
/// they share one uniform buffer.
pub fn chrome_shape(reach: Rect, shapes: ChromeShapes) -> egui::Shape {
    egui::Shape::Callback(egui_wgpu::Callback::new_paint_callback(
        reach,
        Chrome {
            shapes,
            screen: reach,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uniform_keeps_the_newest_panels_and_names_their_islands() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(100.0, 40.0));
        let panel = |island| ChromePanel {
            rect,
            radius: 8.0,
            ring_to_rule: 0.5,
            island,
        };
        let shapes = ChromeShapes {
            islands: vec![(rect, 10.0); 2],
            panels: vec![panel(0), panel(0), panel(1), panel(1)],
            paper: Color32::WHITE,
            rule: Color32::BLACK,
            active: Color32::BLACK,
            rule_width: 1.25,
            ring_width: 1.5,
        };
        let rows = shapes.rows(1.75, false, Pos2::new(800.0, 600.0));
        assert_eq!(rows[0], [800.0, 600.0, 1.75, 0.0]);
        assert_eq!(rows[1], [1.25, 1.5, 2.0, 3.0]);
        assert_eq!(rows[5], [60.0, 40.0, 50.0, 20.0]);
        let shape = 5 + 2 * CHROME_ISLANDS + CHROME_PANELS;
        assert_eq!(rows[shape][3], 0.0, "the oldest panel left out");
        assert_eq!(rows[shape + 2], [8.0, JOIN, 0.5, 1.0]);
    }

    /// The chrome pass alone over a surround of `SURROUND`, on a 200 x 160 offscreen screen.
    fn drawn(shapes: ChromeShapes) -> crate::RgbaImage {
        let core = crate::core::test_core();
        let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(200.0, 160.0));
        let ctx = egui::Context::default();
        // One pass loads the fonts tessellation needs.
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        let clipped = egui::epaint::ClippedShape {
            clip_rect: screen,
            shape: chrome_shape(screen, shapes),
        };
        let frame = crate::ScreenFrame {
            surround: SURROUND,
            canvas: None,
            ui: crate::UiFrame {
                primitives: ctx.tessellate(vec![clipped], 1.0),
                textures: Default::default(),
                pixels_per_point: 1.0,
            },
        };
        let target = crate::ScreenTarget::new(core, [200, 160]).expect("screen");
        let mut painter = crate::ScreenPainter::offscreen(core);
        core.render_screen(&mut painter, &target, frame)
            .expect("draw");
        target.read(core).expect("readback")
    }

    const SURROUND: [u8; 4] = [233, 230, 224, 255];

    fn at(image: &crate::RgbaImage, [x, y]: [u32; 2]) -> [u8; 4] {
        let i = 4 * (y * image.size()[0] + x) as usize;
        let p = image.pixels();
        [p[i], p[i + 1], p[i + 2], p[i + 3]]
    }

    #[test]
    fn an_island_and_its_panel_are_one_paper_with_one_outline() {
        let island = Rect::from_min_max(Pos2::new(100.0, 10.0), Pos2::new(190.0, 50.0));
        let panel = Rect::from_min_max(Pos2::new(110.0, 40.0), Pos2::new(190.0, 150.0));
        let paper = Color32::from_rgb(250, 248, 244);
        let rule = Color32::from_rgb(70, 67, 62);
        let image = drawn(ChromeShapes {
            islands: vec![(island, 10.0)],
            panels: vec![ChromePanel {
                rect: panel,
                radius: 10.0,
                ring_to_rule: 1.0,
                island: 0,
            }],
            paper,
            rule,
            active: Color32::from_rgb(22, 28, 44),
            rule_width: 2.0,
            ring_width: 2.0,
        });
        assert_eq!(at(&image, [20, 20]), SURROUND, "nothing off the chrome");
        assert_eq!(
            at(&image, [150, 30]),
            [250, 248, 244, 255],
            "the island's paper"
        );
        assert_eq!(
            at(&image, [150, 100]),
            [250, 248, 244, 255],
            "the panel's paper"
        );
        assert_eq!(
            at(&image, [150, 10]),
            [70, 67, 62, 255],
            "the island's outline"
        );
        assert_eq!(
            at(&image, [110, 100]),
            [70, 67, 62, 255],
            "the panel's outline"
        );
        // Where the panel leaves the island the two are one: no line across the join.
        assert_eq!(
            at(&image, [150, 50]),
            [250, 248, 244, 255],
            "no outline at the join"
        );
    }
}
