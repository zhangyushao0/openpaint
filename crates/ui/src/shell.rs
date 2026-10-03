//! The screen: a top bar over the rest. On the canvas the bar holds, left to right, Gallery,
//! undo and redo, and at its right end the Layers button with a notice left of it when saving
//! or opening failed; the canvas frame fills the rest and the layer panel opens over it. In the
//! gallery the bar holds the screen's name and "+", and the cards fill the rest.

use egui::{Align, Layout};
use openpaint_app::{
    ContactPhase, EditorAction, GalleryIntent, HistoryIntent, InputBatch, Intent, ScreenProjection,
    SessionProjection, StorageIntent, StorageNotice,
};

use crate::components::{
    CanvasInput, ScreenArea, canvas_frame, icon_button, notice, text_button, top_bar,
};
use crate::design::{Icon, SPACE};
use crate::gallery::GalleryScreen;
use crate::layer_panel::LayerPanel;

/// The history buttons, left to right.
const HISTORY_BUTTONS: [(Icon, &str, EditorAction, HistoryIntent); 2] = [
    (Icon::Undo, "Undo", EditorAction::Undo, HistoryIntent::Undo),
    (Icon::Redo, "Redo", EditorAction::Redo, HistoryIntent::Redo),
];

/// Which screen the last pass showed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Showing {
    #[default]
    Starting,
    Canvas,
    Gallery,
}

/// The screen's UI-local state between frames.
#[derive(Debug, Default)]
pub struct Shell {
    /// The area under the top bar: where the canvas shows, or the gallery's cards.
    canvas: Option<ScreenArea>,
    showing: Showing,
    input: CanvasInput,
    layers: LayerPanel,
    gallery: GalleryScreen,
    /// Intents from taps on the chrome since the last `route`.
    taps: Vec<Intent>,
}

impl Shell {
    /// Builds one frame inside the root `ui` from `projection`, remembers where the canvas is
    /// and collects the buttons tapped. egui hands input to the first pass of a frame only, so
    /// a tap is collected once however many passes run.
    pub fn show(&mut self, ui: &mut egui::Ui, projection: &ScreenProjection) {
        let showing = match projection {
            ScreenProjection::Starting => Showing::Starting,
            ScreenProjection::Canvas(_) => Showing::Canvas,
            ScreenProjection::Gallery(_) => Showing::Gallery,
        };
        if showing != self.showing {
            // A new screen starts with its panels and dialogs closed.
            self.layers.close();
            self.gallery.back();
            self.input = CanvasInput::default();
            self.showing = showing;
        }
        let rect = match projection {
            ScreenProjection::Starting => {
                top_bar(ui, |ui| {
                    ui.label("OpenPaint");
                });
                canvas_frame(ui)
            }
            ScreenProjection::Canvas(session) => self.canvas_screen(ui, session),
            ScreenProjection::Gallery(gallery) => {
                top_bar(ui, |ui| self.gallery.bar(ui, gallery, &mut self.taps));
                egui::CentralPanel::no_frame()
                    .show(ui, |ui| {
                        self.gallery.body(ui, gallery, &mut self.taps);
                        ui.max_rect()
                    })
                    .inner
            }
        };
        self.canvas = Some(ScreenArea::from_points(rect, ui.ctx().pixels_per_point()));
    }

    fn canvas_screen(&mut self, ui: &mut egui::Ui, projection: &SessionProjection) -> egui::Rect {
        top_bar(ui, |ui| {
            let enabled = projection
                .availability(EditorAction::Gallery)
                .is_available();
            if text_button(ui, "Gallery", enabled).clicked() {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
            }
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
        self.layers.show(ui.ctx(), projection, &mut self.taps);
        rect
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

    /// The system's back button: from the canvas it goes to the gallery; in the gallery it
    /// closes an open dialog. Returns whether it was used; otherwise the system's back applies.
    pub fn back(&mut self, projection: &ScreenProjection) -> bool {
        match projection {
            ScreenProjection::Canvas(_) => {
                self.taps.push(Intent::Gallery(GalleryIntent::Show));
                true
            }
            ScreenProjection::Gallery(_) => self.gallery.back(),
            ScreenProjection::Starting => false,
        }
    }

    /// Where the canvas shows on screen (the area under the top bar), once a frame has been
    /// laid out.
    #[must_use]
    pub fn canvas_area(&self) -> Option<ScreenArea> {
        self.canvas
    }

    /// The intents this frame asks for: on the canvas its contacts first, then the taps on the
    /// chrome, so a stroke lifted just before undo is tapped is committed before it is undone.
    /// Call after `show`.
    pub fn route(&mut self, batches: &[InputBatch]) -> Vec<Intent> {
        let Some(area) = self.canvas.filter(|_| self.showing == Showing::Canvas) else {
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
