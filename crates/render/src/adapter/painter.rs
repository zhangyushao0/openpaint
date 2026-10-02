//! `UiPainter`: draws one frame of egui output over a cleared canvas.

use crate::core::{Core, clear_attachment};
use crate::error::RenderError;
use crate::image::RgbaImage;
use crate::readback::{FORMAT, offscreen_texture, read_rgba};
use crate::surface::Surface;

/// One frame of UI output, already tessellated.
pub struct UiFrame {
    pub primitives: Vec<egui::ClippedPrimitive>,
    pub textures: egui::TexturesDelta,
    pub pixels_per_point: f32,
}

/// Paints UI frames. It holds the UI's textures (the font atlas among them), which egui sends
/// only once, so one painter lives as long as the UI context that feeds it.
pub struct UiPainter {
    renderer: egui_wgpu::Renderer,
    format: wgpu::TextureFormat,
}

impl UiPainter {
    /// A painter for `surface` and every later surface of the same format.
    pub fn for_surface(core: &Core, surface: &Surface) -> Self {
        Self::new(core, surface.format())
    }

    /// A painter for offscreen targets read back by `Core::render_ui`.
    pub fn offscreen(core: &Core) -> Self {
        Self::new(core, FORMAT)
    }

    fn new(core: &Core, format: wgpu::TextureFormat) -> Self {
        let options = egui_wgpu::RendererOptions::default();
        Self {
            renderer: egui_wgpu::Renderer::new(&core.gpu.device, format, options),
            format,
        }
    }

    fn check_format(&self, format: wgpu::TextureFormat) -> Result<(), RenderError> {
        if self.format == format {
            Ok(())
        } else {
            Err(RenderError::Invalid(
                "the UI painter was made for another target format",
            ))
        }
    }

    fn upload(&mut self, core: &Core, textures: &egui::TexturesDelta) {
        for (id, deltas) in &textures.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&core.gpu.device, &core.gpu.queue, *id, delta);
            }
        }
    }

    /// Frees what egui let go of and marks the delta as handled (egui checks on drop).
    fn release(&mut self, textures: &mut egui::TexturesDelta) {
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
        textures.clear();
    }

    /// Clears `view` to `canvas` and draws `ui` over it in one pass.
    fn paint(
        &mut self,
        core: &Core,
        view: &wgpu::TextureView,
        size: [u32; 2],
        canvas: [u8; 4],
        ui: &UiFrame,
    ) {
        let (device, queue) = (&core.gpu.device, &core.gpu.queue);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: size,
            pixels_per_point: ui.pixels_per_point,
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        let mut commands =
            self.renderer
                .update_buffers(device, queue, &mut encoder, &ui.primitives, &screen);
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui"),
                color_attachments: &[Some(clear_attachment(view, canvas))],
                ..Default::default()
            })
            .forget_lifetime();
        self.renderer.render(&mut pass, &ui.primitives, &screen);
        drop(pass);
        commands.push(encoder.finish());
        queue.submit(commands);
    }
}

impl Core {
    /// Clears `surface` to `canvas`, draws `ui` over it and presents. Returns `false` when the
    /// window had no frame to give (the frame is skipped; the UI's texture changes still apply).
    pub fn present(
        &self,
        surface: &mut Surface,
        painter: &mut UiPainter,
        canvas: [u8; 4],
        mut ui: UiFrame,
    ) -> Result<bool, RenderError> {
        let presented = self.present_frame(surface, painter, canvas, &ui);
        painter.release(&mut ui.textures);
        presented
    }

    /// The same frame drawn into an offscreen target of `size` pixels and read back.
    pub fn render_ui(
        &self,
        painter: &mut UiPainter,
        size: [u32; 2],
        canvas: [u8; 4],
        mut ui: UiFrame,
    ) -> Result<RgbaImage, RenderError> {
        let image = self.render_offscreen(painter, size, canvas, &ui);
        painter.release(&mut ui.textures);
        image
    }

    fn present_frame(
        &self,
        surface: &mut Surface,
        painter: &mut UiPainter,
        canvas: [u8; 4],
        ui: &UiFrame,
    ) -> Result<bool, RenderError> {
        painter.check_format(surface.format())?;
        painter.upload(self, &ui.textures);
        let Some(frame) = surface.acquire(self)? else {
            return Ok(false);
        };
        let view = frame.texture.create_view(&Default::default());
        painter.paint(self, &view, surface.size(), canvas, ui);
        self.gpu.queue.present(frame);
        Ok(true)
    }

    fn render_offscreen(
        &self,
        painter: &mut UiPainter,
        size: [u32; 2],
        canvas: [u8; 4],
        ui: &UiFrame,
    ) -> Result<RgbaImage, RenderError> {
        painter.check_format(FORMAT)?;
        painter.upload(self, &ui.textures);
        let texture = offscreen_texture(&self.gpu, size)?;
        let view = texture.create_view(&Default::default());
        painter.paint(self, &view, size, canvas, ui);
        read_rgba(&self.gpu, &texture)
    }
}
