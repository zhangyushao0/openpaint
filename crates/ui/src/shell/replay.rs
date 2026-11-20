//! The replay's place on the screen (S163): while the projection holds an open replay the canvas
//! shows its frame at fit between the top band and the replay's bar (`canvas_area`'s fit and
//! `displayed` say so to the renderer), with the replay's chrome instead of the studio's
//! (`replay/`), the pen's tip a thin ring where a stroke is being drawn (`cursor`). Pen and
//! fingers on the canvas do nothing; a contact outside the export panel closes it; the system's
//! back is Back.

use openpaint_app::{InputBatch, Intent, ReplayIntent, ReplayProjection, SessionProjection};

use super::{Shell, Shown};
use crate::canvas_input::{CanvasInput, touched_outside};
use crate::components::canvas_frame;
use crate::replay::{fit_view, replay_area};
use crate::sheet;

impl Shell {
    /// The replay's screen for `projection`, whose replay is `replay`; the rectangle the canvas
    /// shows in, in points.
    pub(super) fn replay_screen(
        &mut self,
        ui: &mut egui::Ui,
        projection: &SessionProjection,
        replay: &ReplayProjection,
    ) -> egui::Rect {
        if !self.replay.was_showing() {
            self.leave_studio();
        }
        self.shown = Shown::of(projection, false);
        let rect = canvas_frame(ui);
        let ctx = ui.ctx().clone();
        let ppp = self.pixels_per_point;
        let area = replay_area(rect, ppp);
        let view = (self.shown).map(|shown| fit_view(shown.canvas, &area, ppp));
        if let Some(view) = view {
            let whole = sheet::show_sheet(&ctx, rect, self.shown, &view, ppp);
            self.shown_at = sheet::left_from(whole, (rect, self.shown), &view, ppp);
        }
        let laid = (replay, projection.canvas(), self.home.utc_offset);
        self.replay.show(&ctx, laid, view, &mut self.taps);
        rect
    }

    /// The replay opened: the studio's panels, menus, bars and gestures end as they are.
    fn leave_studio(&mut self) {
        self.panels.close();
        self.side.hide();
        self.copy.close();
        self.adjust.close_menu();
        self.input = CanvasInput::default();
        self.drop.reset();
        self.drop_bar = None;
        self.view.stop();
    }

    /// This frame's intents while the replay shows: its taps only; a contact on the canvas
    /// outside its controls closes the export panel.
    pub(super) fn replay_route(&mut self, batches: &[InputBatch]) -> Vec<Intent> {
        if let Some(area) = self.canvas
            && touched_outside(&area.screen, self.replay.areas(), batches)
        {
            self.replay.close_panel();
        }
        std::mem::take(&mut self.taps)
    }

    /// The system's back while the replay shows: Back, unless a video is being made, when it
    /// waits. Always used, so the system's own back never leaves the app meanwhile.
    pub(super) fn replay_back(&mut self, projection: &SessionProjection) -> bool {
        let open = projection.replay();
        if open.is_some_and(|replay| replay.controls.is_available()) {
            self.taps.push(Intent::Replay(ReplayIntent::Close));
        }
        true
    }
}
