//! The Pomodoro in the bar (S124), before the lesson's place: the ring (an ink arc for the time
//! gone on a pale ring) and the minutes left, one control named by its minutes ("18 min"); a tap
//! opens its menu under it, Pause (Continue while paused) and Stop timer. Held, the ring pale and
//! Paused where the minutes were. At the time the ring whole in the seal's red and Time in its red,
//! no control; Next reads End, ringed (`next_or_end`).

use egui::{Id, Order, Pos2, Rect, Sense, TextStyle, Vec2, WidgetInfo, WidgetType};
use openpaint_practice_app::{BlockShown, TimerIntent};
use openpaint_ui::components::chrome::Source;
use openpaint_ui::components::{controls, paper, pill_button, text_button};
use openpaint_ui::design::{
    CORNER, INK, INK_SECONDARY, Outline, SPACE, TOUCH_TARGET, paint_pressed, surface_frame,
};

use crate::components::{Ring, paint_ring};
use crate::palette::{VERMILION, VERMILION_INK};
use crate::text::{self as t, Language, Text, Words};

/// The bar's ring's radius, in points.
const RING: f32 = 10.0;

/// What the bar's last word asks: the next exercise (or the lesson's end), or at the time the
/// block's end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Command {
    Next,
    End,
}

/// The ring and its word for `block` (under way or up; nothing for a block ended), one touch
/// target high; where it lies and whether it was tapped (only while it counts or is held).
pub(crate) fn ring_word(ui: &mut egui::Ui, block: &BlockShown) -> Option<(Rect, bool)> {
    let language = t::language(ui.ctx());
    let (ring, word, ink, minutes) = match *block {
        BlockShown::Running {
            left,
            share,
            paused: false,
        } => {
            let gone = 1.0 - share;
            let word = t::minutes_left(language, left);
            (Ring::Counting { gone }, word, INK, Some(left))
        }
        BlockShown::Running {
            left,
            share,
            paused: true,
        } => {
            let word = t::PAUSED.get(language).to_owned();
            let gone = 1.0 - share;
            (Ring::Held { gone }, word, INK_SECONDARY, Some(left))
        }
        BlockShown::Up => {
            let word = t::TIME_UP.get(language).to_owned();
            (Ring::Whole(VERMILION), word, VERMILION_INK, None)
        }
        BlockShown::Ended { .. } => return None,
    };
    let font = TextStyle::Button.resolve(ui.style());
    let galley = ui.painter().layout_no_wrap(word.clone(), font, ink);
    let width = 2.0 * RING + SPACE + galley.size().x + 3.0 * SPACE;
    let sense = if minutes.is_some() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, TOUCH_TARGET), sense);
    if let Some(minutes) = minutes {
        // Named by its minutes whichever language shows, held or not.
        let name = t::minutes_left(Language::English, minutes);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &word));
        controls::register(ui, &name, rect);
        if response.is_pointer_button_down_on() {
            paint_pressed(ui.painter(), Outline::pill(rect));
        }
    }
    let centre = Pos2::new(rect.left() + 1.5 * SPACE + RING, rect.center().y);
    paint_ring(ui.painter(), (centre, RING), ring);
    let at = Pos2::new(
        centre.x + RING + SPACE,
        rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(at, galley, ink);
    Some((rect, response.clicked()))
}

/// The ring's menu under `ring` (points) while it is open: Pause, or Continue while `paused`, and
/// Stop timer; what was tapped and where the menu lies.
pub(crate) fn ring_menu(
    ctx: &egui::Context,
    ring: Rect,
    paused: bool,
) -> (Option<TimerIntent>, Rect) {
    let id = Id::new("pomodoro menu");
    let source = Source::Rect {
        rect: ring,
        radius: (ring.height() / 2.0, CORNER),
        veil: false,
    };
    let area = egui::Area::new(id)
        .order(Order::Foreground)
        .movable(false)
        .constrain(false)
        .fade_in(false)
        .fixed_pos(ring.left_bottom() + Vec2::new(0.0, SPACE));
    paper(ctx, id, source, area, |ui| {
        surface_frame(egui::Margin::same(SPACE as i8))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let (word, intent): (Text, TimerIntent) = if paused {
                    (t::CONTINUE, TimerIntent::Resume)
                } else {
                    (t::PAUSE, TimerIntent::Pause)
                };
                let held = text_button(ui, word, true).clicked().then_some(intent);
                let stop = text_button(ui, t::STOP_TIMER, true).clicked();
                stop.then_some(TimerIntent::Stop).or(held)
            })
            .inner
    })
}

/// The bar's last word: Next or Finish, or at the time (`up`) End in its ink ring; what it asks
/// when tapped and where it lies.
pub(crate) fn next_or_end(
    ui: &mut egui::Ui,
    (last, up): (bool, bool),
    enabled: bool,
) -> Option<(Command, Rect)> {
    let (word, command) = match (up, last) {
        (true, _) => (t::END, Command::End),
        (false, true) => (t::FINISH, Command::Next),
        (false, false) => (t::NEXT, Command::Next),
    };
    let tapped = if up {
        pill_button(ui, Words::from(word), enabled)
    } else {
        text_button(ui, word, enabled)
    };
    tapped.clicked().then_some((command, tapped.rect))
}
