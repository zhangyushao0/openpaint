//! The view settling (D44): after a quick pinch the canvas glides to fit, and an artwork opened
//! from its gallery card grows out of the card into its view. Only what the screen shows moves;
//! input always maps through the view the session holds, so the hand acts on the final state,
//! and any contact on the canvas ends the motion at once (canvas navigation is never animated).

use egui::Id;
use openpaint_app::ViewMapping;

use crate::components::animate_from;
use crate::design::motion::Feel;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct ViewSettle {
    /// Where the canvas showed when the motion began, and how it moves.
    from: Option<(ViewMapping, Feel)>,
    /// Share of the way gone, this pass.
    progress: f32,
    /// One spring per motion.
    runs: u64,
}

impl ViewSettle {
    /// Starts moving from `from` towards whatever the view is, with `feel`.
    pub(crate) fn start(&mut self, from: ViewMapping, feel: Feel) {
        self.from = Some((from, feel));
        self.progress = 0.0;
        self.runs += 1;
    }

    /// Ends the motion where the view is.
    pub(crate) fn stop(&mut self) {
        self.from = None;
    }

    /// Moves on by the time since the last pass.
    pub(crate) fn advance(&mut self, ctx: &egui::Context) {
        let Some((_, feel)) = self.from else {
            return;
        };
        let id = Id::new(("view settle", self.runs));
        self.progress = animate_from(ctx, id, 0.0, 1.0, feel);
        if self.progress >= 1.0 {
            self.from = None;
        }
    }

    /// What the screen shows for the view `to`.
    #[must_use]
    pub(crate) fn shown(&self, to: ViewMapping) -> ViewMapping {
        self.from
            .map_or(to, |(from, _)| from.toward(&to, self.progress))
    }
}
