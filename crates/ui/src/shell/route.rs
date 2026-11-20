//! Routing a frame's contacts (`Shell::route`): the reference window, a colour drop and the
//! canvas take theirs in turn, controls over the canvas never yield theirs to it, a contact on
//! the canvas outside closes the panels and the copy menu, and three fingers swiped down open
//! the copy menu. The view they move the canvas to, foreseen before the pass lays what goes over
//! it (`Shell::foreseen`, S161).

use openpaint_app::{ContactPhase, InputBatch, Intent, StrokeIntent, ViewIntent, ViewMapping};

use super::{Shell, Shown};
use crate::canvas_input::{CanvasFrame, touched_outside};
use crate::color_drop::DropFrame;
use crate::components::{CanvasArea, ScreenArea};
use crate::design::motion::SETTLE;
use crate::hooks::Hooks;
use crate::sheet::Showing;

impl Shell {
    /// The intents this frame asks for: on the canvas its contacts first, then the taps on the
    /// chrome, so a stroke lifted just before undo is tapped is committed before it is undone.
    /// Call after `show`, with the same `hooks`: a mode's taps stay with it.
    pub fn route(&mut self, batches: &[InputBatch], hooks: Hooks<'_>) -> Vec<Intent> {
        if self.replay.showing() {
            return self.replay_route(batches);
        }
        let now_ms = self.now_ms;
        let area = self.canvas.filter(|_| self.showing == Showing::Canvas);
        let (Some(area), Some(shown)) = (area, self.shown) else {
            return std::mem::take(&mut self.taps);
        };
        let covers = self.covers();
        let canvas = hooks.canvas.as_deref();
        let (tapped, glides) = canvas.map_or((false, false), |hook| (hook.tapped(), hook.glides()));
        let control = !self.taps.is_empty() || tapped;
        let handles = self.handles.clone();
        let frame = self.canvas_frame((area, shown), (&covers, &handles), control);
        let drop = DropFrame {
            swatch: self.panels.colors.button_area(),
            color: shown.color,
            threshold: shown.threshold,
            bar: self.drop_bar,
        };
        // The pen on the brush editor's strip draws there.
        self.panels.brushes.route_strip(batches);
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
        self.settle_view(&intents, (frame.mapping(), glides));
        self.ended_by(&intents);
        intents.append(&mut self.taps);
        intents
    }

    /// The view's map in `area` as the canvas shows it this frame: this frame's contacts
    /// (`set_contacts`) routed ahead on a copy of the canvas input, the view moved by the pan or
    /// pinch they make, so what is laid over the canvas moves with it, not a frame behind (S161).
    /// A colour carried takes the contacts: the view as it is.
    pub(crate) fn foreseen(&self, area: CanvasArea) -> Option<ViewMapping> {
        let shown = self.shown?;
        let viewport = area.viewport(self.pixels_per_point);
        let mut view = shown.view;
        if !self.contacts.is_empty() && self.drop.carried().is_none() {
            let covers = self.covers();
            let frame = self.canvas_frame((area, shown), (&covers, &self.handles), false);
            for intent in self.input.clone().route(&frame, &self.contacts) {
                if let Intent::View(ViewIntent::Gesture { gesture, viewport }) = intent {
                    view = view.after(&gesture, shown.canvas, &viewport);
                }
            }
        }
        Some(view.mapping(shown.canvas, &viewport))
    }

    /// Where the controls over the canvas lie, from the last pass: the canvas takes no contact
    /// that lands on them.
    fn covers(&self) -> Vec<ScreenArea> {
        [self.side.area(), self.reference.area(), self.panels.area()]
            .into_iter()
            .chain([self.drop_bar, self.selection.area(), self.transform.area()])
            .chain([self.adjust.area(), self.copy.area(), self.shape.area()])
            .flatten()
            .chain(self.mode_areas.iter().copied())
            .chain(self.islands.iter().copied())
            .collect()
    }

    /// The canvas's frame for routing its contacts: `area`, what `shown` shows, `covers`, the
    /// handles held, and whether a control took a contact this frame.
    fn canvas_frame<'a>(
        &self,
        (area, shown): (CanvasArea, Shown),
        (covers, handles): (&'a [ScreenArea], &'a [[f32; 2]]),
        control: bool,
    ) -> CanvasFrame<'a> {
        CanvasFrame {
            area,
            covers,
            canvas: shown.canvas,
            view: shown.view,
            pixels_per_point: self.pixels_per_point,
            now_ms: self.now_ms,
            picking: shown.picking,
            selecting: shown.selecting,
            adjusting: shown.adjusting,
            transform: shown.transform,
            drawing: shown.drawing,
            buttons: shown.buttons,
            control,
            handles,
            guide_centre: self.guide_centre,
        }
    }

    /// What this frame's contacts and taps end: a new stroke lets the notice that the display
    /// was reset go; choosing a tool ends continuing to fill.
    fn ended_by(&mut self, intents: &[Intent]) {
        if intents.iter().any(starts_stroke) {
            self.reset_notice = false;
        }
        if self.taps.iter().any(|tap| matches!(tap, Intent::Tool(_))) {
            self.drop.set_continuing(false);
        }
    }
}

/// A contact touching down to draw.
fn starts_stroke(intent: &Intent) -> bool {
    matches!(intent, Intent::Stroke(StrokeIntent::Contact(contact))
        if contact.phase == ContactPhase::Down)
}

impl Shell {
    /// A quick pinch settles the view at fit from where it showed, and a mode's tap that moved
    /// the view on (`glides`) where it moved it; a contact on the canvas or a navigation ends any
    /// settling at once, so the hand always acts on what it sees.
    fn settle_view(&mut self, intents: &[Intent], (mapping, glides): (ViewMapping, bool)) {
        if glides {
            self.view.start(self.view.shown(mapping), SETTLE);
        }
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

#[cfg(test)]
mod tests {
    //! The view a pass lays what goes over the canvas with (`Shell::foreseen`, S161): where this
    //! frame's contacts move the canvas, the view the routing then gives it, so the marks over the
    //! canvas (a mode's sheet's frames and givens, the selection, the guide) move with it, never a
    //! frame behind.

    use openpaint_app::{
        ButtonMap, CanvasView, Contact, ContactPhase, Device, InputBatch, InputSample, Intent,
        ViewIntent, ViewMapping,
    };
    use openpaint_core::CanvasSize;

    use super::super::{Shell, Shown};
    use crate::components::{CanvasArea, ScreenArea};
    use crate::hooks::Hooks;
    use crate::sheet::Showing;

    const AREA: ScreenArea = ScreenArea {
        origin: [0, 84],
        size: [400, 300],
    };

    fn area() -> CanvasArea {
        CanvasArea {
            screen: AREA,
            fit: AREA,
        }
    }

    fn fingers(phase: ContactPhase, points: &[[f32; 2]]) -> InputBatch {
        let contacts = (0..)
            .zip(points)
            .map(|(id, &position)| Contact {
                id: 10 + id,
                device: Device::Finger,
                phase,
                samples: vec![InputSample {
                    position,
                    pressure: 1.0,
                    tilt: 0.0,
                    azimuth: 0.0,
                    time_ms: 0,
                }],
            })
            .collect();
        InputBatch {
            contacts,
            pen: None,
        }
    }

    /// A shell showing a canvas as large as its area, at fit.
    fn shell() -> Shell {
        let mut shell = Shell {
            pixels_per_point: 1.0,
            showing: Showing::Canvas,
            canvas: Some(area()),
            ..Shell::default()
        };
        shell.shown = Some(Shown {
            canvas: CanvasSize::new(400, 300).expect("size"),
            view: CanvasView::FIT,
            picking: false,
            selecting: false,
            transforming: false,
            adjusting: false,
            transform: None,
            drawing: false,
            color: None,
            threshold: 0.0,
            brush_size: None,
            buttons: ButtonMap::default(),
            mode: false,
        });
        shell
    }

    /// One frame as the frame loop runs it: its contacts set, the view foreseen for the pass, then
    /// the contacts routed and their view gestures applied, as the next pass shows the canvas. The
    /// view foreseen and the view the canvas shows.
    fn frame(shell: &mut Shell, batch: InputBatch) -> (ViewMapping, ViewMapping) {
        let batches = [batch];
        shell.set_contacts(&batches);
        let foreseen = shell.foreseen(area()).expect("a canvas shows");
        let mut shown = shell.shown.expect("a canvas shows");
        for intent in shell.route(&batches, Hooks::default()) {
            if let Intent::View(ViewIntent::Gesture { gesture, viewport }) = intent {
                shown.view = shown.view.after(&gesture, shown.canvas, &viewport);
            }
        }
        shell.shown = Some(shown);
        let viewport = area().viewport(1.0);
        (foreseen, shown.view.mapping(shown.canvas, &viewport))
    }

    #[test]
    fn what_is_laid_over_the_canvas_lies_where_this_frame_s_fingers_move_it() {
        let mut shell = shell();
        let start = [[180.0, 200.0], [220.0, 200.0]];
        let (_, before) = frame(&mut shell, fingers(ContactPhase::Down, &start));
        let (mut at, mut last, mut moved) = (start, before, 0);
        for step in 1..=8 {
            // Two fingers dragging and spreading.
            at = [
                [at[0][0] + 9.0, at[0][1] + 4.0],
                [at[1][0] + 13.0, at[1][1] + 4.0],
            ];
            let (foreseen, shown) = frame(&mut shell, fingers(ContactPhase::Move, &at));
            assert_eq!(foreseen, shown, "step {step}: laid where the canvas shows");
            // Laid with the view before the frame, the marks would trail the canvas here.
            moved += usize::from(shown != last);
            last = shown;
        }
        assert!(moved >= 6, "the view moved on {moved} of 8 frames");
        let (foreseen, shown) = frame(&mut shell, fingers(ContactPhase::Up, &at));
        assert_eq!(foreseen, shown, "the lift");
        assert_ne!(shown, before, "the fingers moved the view");
    }

    #[test]
    fn without_contacts_the_view_is_the_one_shown() {
        let mut shell = shell();
        shell.set_contacts(&[]);
        let viewport = area().viewport(1.0);
        let shown = shell.shown.expect("a canvas shows");
        assert_eq!(
            shell.foreseen(area()),
            Some(shown.view.mapping(shown.canvas, &viewport))
        );
    }
}
