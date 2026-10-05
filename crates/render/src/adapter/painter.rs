//! `ScreenPainter`: draws one screen frame, the placed canvas with egui output over it.

use crate::core::{Core, clear_attachment};
use crate::error::RenderError;
use crate::gradient::Gradient;
use crate::screen::{CanvasBlit, CanvasPlacement, ScreenTarget};
use crate::surface::Surface;

/// One frame of UI output, already tessellated.
pub struct UiFrame {
    pub primitives: Vec<egui::ClippedPrimitive>,
    pub textures: egui::TexturesDelta,
    pub pixels_per_point: f32,
}

/// Everything one screen frame shows, bottom to top.
pub struct ScreenFrame<'a> {
    /// Fills the screen where the canvas does not reach.
    pub surround: [u8; 4],
    pub canvas: Option<CanvasPlacement<'a>>,
    pub ui: UiFrame,
}

/// Paints screen frames. It holds the UI's textures (the font atlas among them), which egui
/// sends only once, so one painter lives as long as the UI context that feeds it.
pub struct ScreenPainter {
    renderer: egui_wgpu::Renderer,
    blit: CanvasBlit,
    format: wgpu::TextureFormat,
    /// The precision check drawn over everything while it is on (`show_gradient`).
    gradient: Option<Gradient>,
}

impl ScreenPainter {
    /// A painter for `surface` and every later surface of the same format.
    pub fn for_surface(core: &Core, surface: &Surface) -> Self {
        Self::new(core, surface.format())
    }

    /// A painter for `ScreenTarget`s.
    pub fn offscreen(core: &Core) -> Self {
        Self::new(core, core.gpu.display)
    }

    fn new(core: &Core, format: wgpu::TextureFormat) -> Self {
        let options = egui_wgpu::RendererOptions::default();
        let mut renderer = egui_wgpu::Renderer::new(&core.gpu.device, format, options);
        super::chrome::install(&core.gpu.device, &mut renderer, format);
        Self {
            renderer,
            blit: CanvasBlit::new(core, format),
            format,
            gradient: None,
        }
    }

    /// Shows the precision check over the whole screen from the next frame on, or stops (S37,
    /// for debug builds: a soft grey ramp, its upper half at 8 bits, its lower at the window's
    /// own precision, `gradient.rs`).
    pub fn show_gradient(&mut self, core: &Core, on: bool) {
        self.gradient = on.then(|| Gradient::new(&core.gpu.device, self.format));
    }

    fn check_format(&self, format: wgpu::TextureFormat) -> Result<(), RenderError> {
        if self.format == format {
            Ok(())
        } else {
            Err(RenderError::Invalid(
                "the screen painter was made for another target format",
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

    /// Clears `view` to the surround, draws the canvas and then the UI, in one pass.
    fn paint(
        &mut self,
        core: &Core,
        view: &wgpu::TextureView,
        size: [u32; 2],
        frame: &ScreenFrame<'_>,
    ) {
        let (device, queue) = (&core.gpu.device, &core.gpu.queue);
        let ui = &frame.ui;
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
                label: Some("screen"),
                color_attachments: &[Some(clear_attachment(view, frame.surround))],
                ..Default::default()
            })
            .forget_lifetime();
        if let Some(canvas) = &frame.canvas {
            self.blit
                .draw(core, &mut pass, canvas, (size, frame.surround));
        }
        self.renderer.render(&mut pass, &ui.primitives, &screen);
        if let Some(gradient) = &self.gradient {
            pass.set_scissor_rect(0, 0, size[0], size[1]);
            gradient.draw(&mut pass);
        }
        drop(pass);
        commands.push(encoder.finish());
        queue.submit(commands);
    }
}

impl Core {
    /// Draws `frame` into `surface` and presents. Returns `false` when the window had no frame
    /// to give (the frame is skipped; the UI's texture changes still apply).
    pub fn present(
        &self,
        surface: &mut Surface,
        painter: &mut ScreenPainter,
        mut frame: ScreenFrame<'_>,
    ) -> Result<bool, RenderError> {
        let presented = self.present_frame(surface, painter, &frame);
        painter.release(&mut frame.ui.textures);
        presented
    }

    /// Draws `frame` into an offscreen `target`; read it back with `ScreenTarget::read`.
    pub fn render_screen(
        &self,
        painter: &mut ScreenPainter,
        target: &ScreenTarget,
        mut frame: ScreenFrame<'_>,
    ) -> Result<(), RenderError> {
        let drawn = painter.check_format(target.texture().format()).map(|()| {
            painter.upload(self, &frame.ui.textures);
            let view = target.texture().create_view(&Default::default());
            painter.paint(self, &view, target.size(), &frame);
        });
        painter.release(&mut frame.ui.textures);
        drawn
    }

    fn present_frame(
        &self,
        surface: &mut Surface,
        painter: &mut ScreenPainter,
        frame: &ScreenFrame<'_>,
    ) -> Result<bool, RenderError> {
        painter.check_format(surface.format())?;
        painter.upload(self, &frame.ui.textures);
        let Some(texture) = surface.acquire(self)? else {
            return Ok(false);
        };
        let view = texture.texture.create_view(&Default::default());
        painter.paint(self, &view, surface.size(), frame);
        self.gpu.queue.present(texture);
        Ok(true)
    }
}
