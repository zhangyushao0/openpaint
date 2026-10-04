//! A group of choices in one container, one of them chosen (the selection's Freehand and
//! Rectangle, a layer's four blend modes, the pen's Brush, Smudge and Eraser). The group owns
//! the one active mark: it remembers where the mark is in egui's memory and, when another item
//! is chosen, the mark slides and resizes there with the settle spring (D44's settle pattern),
//! retargeting from where it is if the choice changes again mid-slide. With nothing chosen the
//! mark fades where it is; a choice after that, or a group shown for the first time (a panel
//! opening), puts the mark in place at once. Items only say whether they are the chosen one.

use egui::{Id, Painter, Rect};

use crate::components::animate::{keep_moving, now};
use crate::design::motion::{ACKNOWLEDGE, SETTLE, Spring};
use crate::design::{Outline, paint_mark};

/// The group while its items are laid out: where the chosen one's mark goes.
pub struct Choices {
    id: Id,
    painter: Painter,
    chosen: Option<Outline>,
}

impl Choices {
    /// The item whose mark lies in `outline` is the chosen one.
    pub(crate) fn chosen(&mut self, outline: Outline) {
        self.chosen = Some(outline);
    }
}

/// Lays out the group `salt` through `add`, then draws its mark over the items.
pub fn choices<R>(
    ui: &mut egui::Ui,
    salt: &str,
    add: impl FnOnce(&mut egui::Ui, &mut Choices) -> R,
) -> R {
    let mut group = Choices {
        id: ui.id().with(salt).with("choices"),
        painter: ui.painter().clone(),
        chosen: None,
    };
    // Measured from the group's own corner, so a group that moves (its panel growing, rows
    // added above it) carries its mark along at once; only a change of choice slides.
    let origin = ui.cursor().min.to_vec2();
    let out = add(ui, &mut group);
    let target = group.chosen.map(|outline| outline.rect.translate(-origin));
    let (rect, shown) = step(ui.ctx(), group.id, target);
    let rect = rect.translate(origin);
    if ui.is_rect_visible(rect) {
        paint_mark(&group.painter, Outline::round(rect), shown);
    }
    out
}

/// The mark kept between passes: the place it heads for (the last chosen outline), how far
/// each edge (left, top, right, bottom) still shows from it, and how much it shows. Springs
/// run on the offsets, near zero, where a point's fraction is never lost to rounding.
#[derive(Debug, Clone, Copy)]
struct Mark {
    place: Rect,
    offsets: [Spring; 4],
    shown: Spring,
    /// Whether something was chosen at its last step.
    chosen: bool,
    /// The input time and the pass of its last step.
    time: f64,
    pass: u64,
}

impl Mark {
    fn at(target: Option<Rect>, time: f64, pass: u64) -> Self {
        Self {
            place: target.unwrap_or(Rect::ZERO),
            offsets: [Spring::at(0.0); 4],
            shown: Spring::at(f32::from(u8::from(target.is_some()))),
            chosen: target.is_some(),
            time,
            pass,
        }
    }

    fn rect(&self) -> Rect {
        let [left, top, right, bottom] = self.offsets.map(|offset| offset.x);
        Rect::from_min_max(
            self.place.min + egui::vec2(left, top),
            self.place.max + egui::vec2(right, bottom),
        )
    }

    fn at_rest(&self, chosen: bool) -> bool {
        let still = self.offsets.iter().all(|offset| offset.at_rest(0.0));
        self.shown.at_rest(f32::from(u8::from(chosen))) && (still || !chosen)
    }
}

fn edges(rect: Rect) -> [f32; 4] {
    [rect.left(), rect.top(), rect.right(), rect.bottom()]
}

/// Moves the group `id`'s mark towards `target` (the chosen item's outline, `None` for no
/// choice); where it shows now and how much.
fn step(ctx: &egui::Context, id: Id, target: Option<Rect>) -> (Rect, f32) {
    let (time, pass) = (now(ctx), ctx.cumulative_pass_nr());
    let (mark, resting) = ctx.data_mut(|data| {
        let kept = data.get_temp::<Mark>(id);
        // Not drawn in the pass before: the group shows anew, its mark already in place.
        let mut mark = match kept {
            Some(mark) if mark.pass + 1 >= pass => mark,
            _ => Mark::at(target, time, pass),
        };
        // At rest until now: it starts moving from this frame on, so idle time never counts.
        let dt = if mark.at_rest(mark.chosen) {
            0.0
        } else {
            (time - mark.time) as f32
        };
        if let Some(rect) = target {
            if mark.shown.at_rest(0.0) {
                // Faded out: the new choice's mark draws in where it is, without sliding there.
                mark.offsets = [Spring::at(0.0); 4];
            } else {
                // Retargeted: the edges keep where they show and their speed.
                let moved = edges(mark.place).into_iter().zip(edges(rect));
                for (offset, (from, to)) in mark.offsets.iter_mut().zip(moved) {
                    offset.x += from - to;
                }
            }
            mark.place = rect;
            for offset in &mut mark.offsets {
                offset.step(0.0, SETTLE, 1.0, dt);
            }
        }
        let shown = f32::from(u8::from(target.is_some()));
        mark.shown.step(shown, ACKNOWLEDGE, 1.0, dt);
        mark.chosen = target.is_some();
        mark.time = time;
        mark.pass = pass;
        data.insert_temp(id, mark);
        (mark, mark.at_rest(mark.chosen))
    });
    if !resting {
        keep_moving(ctx);
    }
    (mark.rect(), mark.shown.x)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass(ctx: &egui::Context, time: f64, target: Option<Rect>) -> (Rect, f32, bool) {
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut seen = (Rect::NOTHING, 0.0, false);
        let mut output = ctx.run_ui(input, |ui| {
            let (rect, shown) = step(ui.ctx(), Id::new("group"), target);
            seen = (rect, shown, crate::components::moving(ui.ctx()));
        });
        output.textures_delta.clear();
        seen
    }

    fn word(left: f32, width: f32) -> Option<Rect> {
        Some(Rect::from_min_size(
            egui::pos2(left, 10.0),
            egui::vec2(width, 38.0),
        ))
    }

    #[test]
    fn the_mark_slides_and_resizes_to_the_new_choice_and_retargets_mid_slide() {
        let ctx = egui::Context::default();
        let (first, _, moving) = pass(&ctx, 0.0, word(0.0, 100.0));
        assert_eq!(first, word(0.0, 100.0).unwrap(), "shown anew: in place");
        assert!(!moving);
        pass(&ctx, 1.0, word(200.0, 160.0));
        let (mid, shown, moving) = pass(&ctx, 1.06, word(200.0, 160.0));
        assert!(moving && shown == 1.0, "it slides, never fades");
        assert!(mid.left() > 0.0 && mid.left() < 200.0);
        assert!(
            mid.width() > 100.0 && mid.width() < 160.0,
            "its width follows"
        );
        // A new choice mid-slide: from where it is, never a jump.
        let (turned, ..) = pass(&ctx, 1.076, word(0.0, 100.0));
        assert!((turned.left() - mid.left()).abs() < 40.0);
        let mut last = (Rect::NOTHING, 0.0, true);
        for frame in 1..120 {
            last = pass(&ctx, 1.076 + f64::from(frame) / 60.0, word(0.0, 100.0));
        }
        assert_eq!(
            last,
            (word(0.0, 100.0).unwrap(), 1.0, false),
            "at rest, exactly"
        );
    }

    #[test]
    fn with_nothing_chosen_it_fades_and_a_later_choice_draws_in_in_place() {
        let ctx = egui::Context::default();
        pass(&ctx, 0.0, word(0.0, 100.0));
        pass(&ctx, 1.0, None);
        let mut last = (Rect::NOTHING, 1.0, true);
        for frame in 1..60 {
            last = pass(&ctx, 1.0 + f64::from(frame) / 60.0, None);
        }
        assert_eq!(
            (last.0, last.1, last.2),
            (word(0.0, 100.0).unwrap(), 0.0, false)
        );
        pass(&ctx, 3.0, word(200.0, 160.0));
        let (rect, shown, _) = pass(&ctx, 3.05, word(200.0, 160.0));
        assert_eq!(
            rect,
            word(200.0, 160.0).unwrap(),
            "no slide from the faded place"
        );
        assert!(shown > 0.0 && shown < 1.0, "it draws in");
    }

    #[test]
    fn a_group_not_drawn_in_the_pass_before_shows_its_mark_in_place() {
        let ctx = egui::Context::default();
        pass(&ctx, 0.0, word(0.0, 100.0));
        // The panel closed for a pass (the group not drawn), then opened on another choice.
        let input = egui::RawInput {
            time: Some(0.5),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |_| {});
        output.textures_delta.clear();
        let (rect, shown, moving) = pass(&ctx, 1.0, word(200.0, 160.0));
        assert_eq!(
            (rect, shown, moving),
            (word(200.0, 160.0).unwrap(), 1.0, false)
        );
    }
}
