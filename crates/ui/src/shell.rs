//! The screen: the top bar over the canvas frame, which fills the rest of the screen. The bar
//! holds the title (the gallery's place, S10), undo and redo, at its right end the Layers
//! button and left of it a notice when saving or opening failed. The layer panel opens over the
//! canvas.

use egui::{Align, Layout};
use openpaint_app::{
    ContactPhase, EditorAction, HistoryIntent, InputBatch, Intent, SessionProjection,
    StorageIntent, StorageNotice,
};

use crate::components::{CanvasInput, ScreenArea, canvas_frame, icon_button, notice, top_bar};
use crate::design::{Icon, SPACE};
use crate::layer_panel::LayerPanel;

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
    layers: LayerPanel,
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
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.layers.button(ui);
                self.storage_notice(ui, projection);
            });
        });
        let rect = canvas_frame(ui);
        self.canvas = Some(ScreenArea::from_points(rect, ui.ctx().pixels_per_point()));
        self.layers.show(ui.ctx(), projection, &mut self.taps);
    }

    /// The notice for a failed save (with Retry) or a failed open (with OK), if any.
    fn storage_notice(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) {
        let (message, action, editor_action, intent) = match projection.storage() {
            StorageNotice::None => return,
            StorageNotice::SaveFailed { retrying: false } => (
                "Couldn't save. Your drawing is still here.",
                "Retry",
                EditorAction::RetrySave,
                StorageIntent::RetrySave,
            ),
            StorageNotice::SaveFailed { retrying: true } => (
                "Saving…",
                "Retry",
                EditorAction::RetrySave,
                StorageIntent::RetrySave,
            ),
            StorageNotice::OpenFailed => (
                "Couldn't open the last drawing. This is a new one.",
                "OK",
                EditorAction::DismissNotice,
                StorageIntent::Dismiss,
            ),
        };
        let enabled = projection.availability(editor_action).is_available();
        if notice(ui, message, action, enabled).clicked() {
            self.taps.push(Intent::Storage(intent));
        }
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
        let Some(area) = self.canvas else {
            return std::mem::take(&mut self.taps);
        };
        let cover = self.layers.area();
        let mut intents = self.input.route(&area, cover.as_ref(), batches);
        if let Some(cover) = cover
            && touched_outside(&area, &cover, batches)
        {
            self.layers.close();
        }
        intents.append(&mut self.taps);
        intents
    }
}

/// True when a contact of `batches` touches down on the canvas `area` outside `cover`.
fn touched_outside(area: &ScreenArea, cover: &ScreenArea, batches: &[InputBatch]) -> bool {
    batches
        .iter()
        .flat_map(|batch| &batch.contacts)
        .filter(|contact| contact.phase == ContactPhase::Down)
        .filter_map(|contact| contact.samples.first())
        .any(|first| area.contains(first.position) && !cover.contains(first.position))
}
