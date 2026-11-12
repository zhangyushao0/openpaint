use egui::{Context, Pos2, Rect, Ui};
use openpaint_app::{Gallery, Intent};

use super::*;
use crate::components::carry::Opening;
use crate::components::controls;
use crate::text::{self as t, Text};

/// A mode's hook that keeps where the home gave it its column.
#[derive(Default)]
struct Column {
    rect: Option<Rect>,
}

impl HomeHook for Column {
    fn page(&mut self, _: &Context, _: &mut Vec<Intent>) -> Page {
        Page::None
    }

    fn column(&mut self, _: &mut Ui, rect: Rect, _: bool, _: &mut Vec<Intent>) -> Rect {
        self.rect = Some(rect);
        rect
    }

    fn dialogs(&mut self, _: &Context, _: &mut Vec<Intent>) {}

    fn notice(&self) -> Option<Text> {
        None
    }

    fn dismiss(&mut self) {}

    fn opening(&mut self) -> Option<Opening> {
        None
    }

    fn back(&mut self) -> bool {
        false
    }
}

#[test]
fn the_mode_s_column_lies_beside_the_studio_on_the_screen() {
    let ctx = Context::default();
    crate::design::install(&ctx);
    let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(1440.0, 900.0));
    let mut home = HomeScreen::default();
    let mut hook = Column::default();
    let projection = Gallery::new().projection();
    let mut map = None;
    for _ in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            controls::begin(ui.ctx());
            let mut taps = Vec::new();
            home.show(ui, &projection, (Some(&mut hook), &mut taps));
            map = Some(controls::take(ui.ctx()));
        });
        output.textures_delta.clear();
    }
    let column = hook.rect.expect("the column was laid out");
    assert!(screen.contains_rect(column), "{column:?}");
    assert!(column.width() >= 400.0 && column.right() < screen.center().x);
    let map = map.expect("a pass");
    let plus = map.find(t::NEW_ARTWORK.id()).expect("+ in its island");
    assert!(plus.rect[0] > column.right(), "+ at the studio's side");
    assert!(map.find(t::SEARCH.id()).is_ok());
}
