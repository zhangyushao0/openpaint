//! The copy menu (Procreate's Copy & Paste): a pill in the middle of the top band holding
//! the commands Cut, Copy, Copy All, Duplicate, Cut & Paste and Paste as bare words, each
//! pencil grey when there is nothing for it. Three fingers swiped down on the canvas open it,
//! and so does Actions > Copy & Paste (the visible entry the gesture duplicates). A command
//! closes it, and so does any contact landing on the canvas outside it. While the selection
//! or the transform bar holds the band's middle it lies just under the islands. It grows out
//! of its own middle and goes back into it.

use openpaint_app::{ClipboardAction, EditorAction, Intent, SessionProjection};

use crate::components::{Place, ScreenArea, Source, pill, text_button};
use crate::taps::push_once;
use crate::text as t;

#[derive(Debug, Default)]
pub(crate) struct CopyBar {
    open: bool,
    /// Where the bar lies on screen, in pixels, from the last pass.
    area: Option<ScreenArea>,
}

impl CopyBar {
    pub(crate) fn open(&mut self) {
        self.open = true;
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
        self.area = None;
    }

    /// The bar while it is open, its taps pushed to `taps`.
    pub(crate) fn show(
        &mut self,
        ctx: &egui::Context,
        projection: &SessionProjection,
        taps: &mut Vec<Intent>,
    ) {
        self.area = None;
        if !self.open {
            return;
        }
        let place =
            Place::middle_unless(projection.selection().tool || projection.transform().tool);
        let source = Source::Within(egui::Align::Center);
        let (tapped, rect) = pill(ctx, "copy bar", place, source, |ui| {
            let mut tapped = None;
            for action in ClipboardAction::ALL {
                let offered = projection.availability(EditorAction::Clipboard(action));
                if text_button(ui, t::clipboard(action), offered.is_offered()).clicked() {
                    tapped = Some(action);
                }
            }
            tapped
        });
        if let Some(action) = tapped {
            push_once(taps, Intent::Clipboard(action));
            self.close();
        } else {
            self.area = Some(ScreenArea::from_points(rect, ctx.pixels_per_point()));
        }
    }

    /// Where the bar lies, which the canvas must not take contacts from.
    pub(crate) const fn area(&self) -> Option<ScreenArea> {
        self.area
    }
}
