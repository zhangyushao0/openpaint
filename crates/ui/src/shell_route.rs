//! Routing a frame's contacts (`Shell::route`): the reference window, a colour drop and the
//! canvas take theirs in turn, controls over the canvas never yield theirs to it, a contact on
//! the canvas outside closes the panels and the copy menu, and three fingers swiped down open
//! the copy menu (S40).

use openpaint_app::{InputBatch, Intent, ViewIntent, ViewMapping};

use super::Shell;
use crate::canvas_input::{CanvasFrame, touched_outside};
use crate::color_drop::DropFrame;
use crate::components::ScreenArea;
use crate::design::motion::SETTLE;
use crate::sheet::Showing;

impl Shell {
    /// The intents this frame asks for: on the canvas its contacts first, then the taps on the
    /// chrome, so a stroke lifted just before undo is tapped is committed before it is undone.
    /// Call after `show`.
    pub fn route(&mut self, batches: &[InputBatch]) -> Vec<Intent> {
        let now_ms = self.now_ms;
        let area = self.canvas.filter(|_| self.showing == Showing::Canvas);
        let (Some(area), Some(shown)) = (area, self.shown) else {
            return std::mem::take(&mut self.taps);
        };
        let panel = self.panels.area();
        let covers: Vec<ScreenArea> = [self.side.area(), self.reference.area(), panel]
            .into_iter()
            .chain([self.drop_bar, self.selection.area(), self.transform.area()])
            .chain([self.adjust.area(), self.copy.area()])
            .flatten()
            .chain(self.islands.iter().copied())
            .collect();
        let frame = CanvasFrame {
            area,
            covers: &covers,
            canvas: shown.canvas,
            view: shown.view,
            pixels_per_point: self.pixels_per_point,
            now_ms,
            picking: shown.picking,
            selecting: shown.selecting,
            adjusting: shown.adjusting,
            transform: shown.transform,
            drawing: shown.drawing,
            buttons: shown.buttons,
            control: !self.taps.is_empty(),
        };
        let drop = DropFrame {
            swatch: self.panels.colors.button_area(),
            color: shown.color,
            threshold: shown.threshold,
            bar: self.drop_bar,
        };
        let mut intents = self.reference.route(batches, now_ms, shown.picking);
        let (dropped, rest) = self.drop.route(&frame, &drop, batches);
        intents.extend(dropped);
        if self.drop.carried().is_some() {
            self.panels.close();
        }
        intents.extend(self.input.route(&frame, &rest));
        if touched_outside(&area.screen, &covers, batches) {
            self.panels.close();
            self.adjust.close_menu();
            self.copy.close();
        }
        if self.input.take_copy_menu() {
            self.panels.close();
            self.copy.open();
        }
        self.settle_view(&intents, frame.mapping());
        // Choosing a tool ends continuing to fill.
        if self.taps.iter().any(|tap| matches!(tap, Intent::Tool(_))) {
            self.drop.set_continuing(false);
        }
        intents.append(&mut self.taps);
        intents
    }

    /// A quick pinch settles the view at fit from where it showed; a contact on the canvas or a
    /// navigation ends any settling at once, so the hand always acts on what it sees.
    fn settle_view(&mut self, intents: &[Intent], mapping: ViewMapping) {
        for intent in intents {
            match intent {
                Intent::View(ViewIntent::Fit) => self.view.start(self.view.shown(mapping), SETTLE),
                Intent::View(_)
                | Intent::Stroke(_)
                | Intent::Selection(_)
                | Intent::Transform(_) => self.view.stop(),
                _ => {}
            }
        }
    }
}
