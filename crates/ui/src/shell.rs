//! The screen: the top bar over the canvas frame, which fills the rest of the screen. The bar
//! holds the title (the gallery's place, S10) and undo and redo.

use openpaint_app::{EditorAction, HistoryIntent, InputBatch, Intent, SessionProjection};

use crate::components::{CanvasInput, ScreenArea, canvas_frame, icon_button, top_bar};
use crate::design::{Icon, SPACE};

/// The history buttons, left to right.
const HISTORY_BUTTONS: [(Icon, &str, EditorAction, HistoryIntent); 2] = [
    (Icon::Undo, "Undo", EditorAction::Undo, HistoryIntent::Undo),
    (Icon::Redo, "Redo", EditorAction::Redo, HistoryIntent::Redo),
];

/// The screen's UI-local state between frames.
#[derive(Debug, Default)]
pub struct Shell {
    canvas: Option<ScreenArea>,
    input: CanvasInput,
    /// Intents from taps on the chrome since the last `route`.
    taps: Vec<Intent>,
}

impl Shell {
    /// Builds one frame inside the root `ui` from `projection`, remembers where the canvas is
    /// and collects the buttons tapped. egui hands input to the first pass of a frame only, so
    /// a tap is collected once however many passes run.
    pub fn show(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) {
        top_bar(ui, |ui| {
            ui.label("OpenPaint");
            ui.add_space(SPACE);
            for (icon, label, action, intent) in HISTORY_BUTTONS {
                let enabled = projection.availability(action).is_available();
                if icon_button(ui, icon, label, enabled).clicked() {
                    self.taps.push(Intent::History(intent));
                }
            }
        });
        let rect = canvas_frame(ui);
        self.canvas = Some(ScreenArea::from_points(rect, ui.ctx().pixels_per_point()));
    }

    /// Where the canvas shows on screen, once a frame has been laid out.
    #[must_use]
    pub fn canvas_area(&self) -> Option<ScreenArea> {
        self.canvas
    }

    /// The intents this frame asks for: the canvas contacts first, then the taps on the chrome,
    /// so a stroke lifted just before undo is tapped is committed before it is undone. Call
    /// after `show`.
    pub fn route(&mut self, batches: &[InputBatch]) -> Vec<Intent> {
        let mut intents = match &self.canvas {
            Some(area) => self.input.route(area, batches),
            None => Vec::new(),
        };
        intents.append(&mut self.taps);
        intents
    }
}
