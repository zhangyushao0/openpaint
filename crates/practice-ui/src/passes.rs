//! Headless egui passes for the screens' tests: a 1440 by 900 point screen in the theme and a
//! language, the controls each pass registered, and a tap by a control's name.

use egui::{Context, Pos2, Rect, Ui};
use openpaint_ui::ControlMap;
use openpaint_ui::components::{chrome, controls};

use crate::text::{Language, set_language};

/// The screen the passes lay out on, in points.
pub(crate) const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1440.0, 900.0));

/// A context in the theme.
pub(crate) fn context() -> Context {
    let ctx = Context::default();
    openpaint_ui::design::install(&ctx);
    ctx
}

/// One pass at `time` seconds in `language` with `events`, `add` laying out the screen; the
/// controls it registered.
pub(crate) fn pass(
    ctx: &Context,
    (time, language, events): (f64, Language, Vec<egui::Event>),
    mut add: impl FnMut(&mut Ui),
) -> ControlMap {
    let input = egui::RawInput {
        screen_rect: Some(SCREEN),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut map = ControlMap::default();
    let mut output = ctx.run_ui(input, |ui| {
        controls::begin(ui.ctx());
        chrome::begin(ui.ctx());
        set_language(ui.ctx(), language);
        add(ui);
        chrome::finish(ui.ctx());
        map = controls::take(ui.ctx());
    });
    output.textures_delta.clear();
    map
}

/// Passes from `time` a sixtieth of a second apart until things rest (two seconds at most);
/// the last one's controls.
pub(crate) fn settled(
    ctx: &Context,
    (time, language): (f64, Language),
    mut add: impl FnMut(&mut Ui),
) -> ControlMap {
    let mut map = ControlMap::default();
    for step in 0..120 {
        map = pass(
            ctx,
            (time + f64::from(step) / 60.0, language, Vec::new()),
            &mut add,
        );
    }
    map
}

/// The control `name` is registered once and lies whole on the screen.
pub(crate) fn assert_on_screen(map: &ControlMap, name: &str) {
    let control = map
        .find(name)
        .unwrap_or_else(|why| panic!("{name}: {why:?} among {:?}", map.names()));
    let [left, top, right, bottom] = control.rect;
    assert!(
        left >= 0.0 && top >= 0.0 && right <= SCREEN.right() && bottom <= SCREEN.bottom(),
        "{name} off the screen: {:?}",
        control.rect
    );
}

/// A finger's tap at the middle of the control `name` in `map`: the press, then the release.
pub(crate) fn tap(map: &ControlMap, name: &str) -> [Vec<egui::Event>; 2] {
    let control = map.find(name).expect("a control to tap");
    let [x, y] = control.at([0.5, 0.5]);
    let at = Pos2::new(x, y);
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    [
        vec![egui::Event::PointerMoved(at), button(true)],
        vec![button(false)],
    ]
}
