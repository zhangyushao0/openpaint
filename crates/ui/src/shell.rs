//! The screen: the top bar over the canvas frame, which fills the rest of the screen.

use openpaint_app::{InputBatch, Intent};

use crate::components::{CanvasInput, ScreenArea, canvas_frame, top_bar};

/// The screen's UI-local state between frames.
#[derive(Debug, Default)]
pub struct Shell {
    canvas: Option<ScreenArea>,
    input: CanvasInput,
}

impl Shell {
    /// Builds one frame inside the root `ui` and remembers where the canvas is.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        top_bar(ui, |ui| {
            ui.label("OpenPaint");
        });
        let rect = canvas_frame(ui);
        self.canvas = Some(ScreenArea::from_points(rect, ui.ctx().pixels_per_point()));
    }

    /// Where the canvas shows on screen, once a frame has been laid out.
    #[must_use]
    pub fn canvas_area(&self) -> Option<ScreenArea> {
        self.canvas
    }

    /// The intents this frame's input asks for. Call after `show`.
    pub fn route(&mut self, batches: &[InputBatch]) -> Vec<Intent> {
        match &self.canvas {
            Some(area) => self.input.route(area, batches),
            None => Vec::new(),
        }
    }
}
