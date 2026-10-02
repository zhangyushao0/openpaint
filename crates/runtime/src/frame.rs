//! `Runtime`: the frame loop a platform host drives. The host attaches, resizes and detaches
//! its window, hands over input batches and asks for frames; all calls come from one thread.

use openpaint_render::{Core, RgbaImage, Surface, SurfaceWindow, UiFrame, UiPainter};

use crate::error::RuntimeError;
use crate::input::MotionBatch;

/// The canvas colour until artworks render (S3).
pub const CANVAS: [u8; 4] = [255, 255, 255, 255];

pub struct Runtime {
    core: Core,
    ui: egui::Context,
    /// Created with the first target and kept for the process: it holds the UI's textures.
    painter: Option<UiPainter>,
    surface: Option<Surface>,
    pixels_per_point: f32,
}

impl Runtime {
    /// Opens the GPU device. `pixels_per_point` is the display density (Android's
    /// `DisplayMetrics.density`): UI points per physical pixel.
    pub fn start(pixels_per_point: f32) -> Result<Self, RuntimeError> {
        let core = Core::headless()?;
        log::info!("GPU adapter: {}", core.adapter());
        Ok(Self {
            core,
            ui: egui::Context::default(),
            painter: None,
            surface: None,
            pixels_per_point,
        })
    }

    /// The host's window exists, `size` pixels large. Replaces any previous window.
    pub fn attach(
        &mut self,
        window: impl SurfaceWindow,
        size: [u32; 2],
    ) -> Result<(), RuntimeError> {
        self.surface = None;
        let surface = self.core.create_surface(window, size)?;
        if self.painter.is_none() {
            self.painter = Some(UiPainter::for_surface(&self.core, &surface));
        }
        self.surface = Some(surface);
        Ok(())
    }

    /// The window changed size or density (rotation, split screen, display change).
    pub fn resize(&mut self, size: [u32; 2], pixels_per_point: f32) {
        self.pixels_per_point = pixels_per_point;
        if let Some(surface) = &mut self.surface {
            surface.resize(&self.core, size);
        }
    }

    /// The window is about to be destroyed; call before the host lets it go.
    pub fn detach(&mut self) {
        self.surface = None;
    }

    /// Takes one platform input batch. Until the normaliser arrives (S4) it is only logged.
    pub fn motion(&mut self, batch: &MotionBatch) {
        log::info!(target: "openpaint::input", "{batch}");
    }

    /// Runs the UI and presents one frame. Returns whether another frame is wanted at once.
    pub fn frame(&mut self) -> Result<bool, RuntimeError> {
        let Some(surface) = self.surface.as_mut() else {
            return Ok(false);
        };
        let (ui, repaint) = run_ui(&self.ui, surface.size(), self.pixels_per_point);
        let painter = self
            .painter
            .get_or_insert_with(|| UiPainter::for_surface(&self.core, surface));
        let presented = self.core.present(surface, painter, CANVAS, ui)?;
        Ok(repaint || !presented)
    }

    /// The same frame drawn offscreen at `size` pixels and read back, for the harness. Only for
    /// a runtime that never attached a window.
    pub fn screenshot(&mut self, size: [u32; 2]) -> Result<RgbaImage, RuntimeError> {
        let (ui, _) = run_ui(&self.ui, size, self.pixels_per_point);
        let painter = self
            .painter
            .get_or_insert_with(|| UiPainter::offscreen(&self.core));
        Ok(self.core.render_ui(painter, size, CANVAS, ui)?)
    }
}

/// One UI pass over a screen of `size` pixels. Returns the tessellated frame and whether egui
/// wants the next frame immediately (animations, first-frame layout).
fn run_ui(ctx: &egui::Context, size: [u32; 2], pixels_per_point: f32) -> (UiFrame, bool) {
    let points = egui::vec2(size[0] as f32, size[1] as f32) / pixels_per_point;
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, points)),
        ..Default::default()
    };
    input
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(pixels_per_point);
    let output = ctx.run_ui(input, openpaint_ui::shell);
    let repaint = output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .is_some_and(|viewport| viewport.repaint_delay.is_zero());
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    let frame = UiFrame {
        primitives,
        textures: output.textures_delta,
        pixels_per_point: output.pixels_per_point,
    };
    (frame, repaint)
}
