//! Headless egui passes for the screens' tests: a 1440 by 900 point screen (or the tablet's)
//! in the theme and a language, the controls each pass registered and what it drew (its words,
//! its outlines in a colour), and a tap by a control's name.

use egui::{Color32, Context, Pos2, Rect, Shape, Ui};
use openpaint_ui::ControlMap;
use openpaint_ui::components::{chrome, controls};

use crate::text::{Language, set_language};

/// The screen the passes lay out on, and the tablet's in landscape (2880 by 1800 at 1.75), in
/// points.
pub(crate) const SCREEN: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1440.0, 900.0));
pub(crate) const TABLET: Rect =
    Rect::from_min_max(Pos2::ZERO, Pos2::new(2880.0 / 1.75, 1800.0 / 1.75));

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
    at: (f64, Language, Vec<egui::Event>),
    add: impl FnMut(&mut Ui),
) -> ControlMap {
    drawn(ctx, at, add).0
}

/// As `pass` on `screen`.
pub(crate) fn pass_on(
    ctx: &Context,
    screen: Rect,
    at: (f64, Language, Vec<egui::Event>),
    add: impl FnMut(&mut Ui),
) -> ControlMap {
    drawn_on(ctx, screen, at, add).0
}

/// As `pass`, with what it drew: each shape, groups flattened.
pub(crate) fn drawn(
    ctx: &Context,
    at: (f64, Language, Vec<egui::Event>),
    add: impl FnMut(&mut Ui),
) -> (ControlMap, Vec<Shape>) {
    drawn_on(ctx, SCREEN, at, add)
}

/// As `drawn` on `screen`.
fn drawn_on(
    ctx: &Context,
    screen: Rect,
    (time, language, events): (f64, Language, Vec<egui::Event>),
    mut add: impl FnMut(&mut Ui),
) -> (ControlMap, Vec<Shape>) {
    let input = egui::RawInput {
        screen_rect: Some(screen),
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
    let mut shapes = Vec::new();
    let mut open: Vec<Shape> = output
        .shapes
        .into_iter()
        .map(|clipped| clipped.shape)
        .collect();
    while let Some(shape) = open.pop() {
        match shape {
            Shape::Vec(inner) => open.extend(inner),
            shape => shapes.push(shape),
        }
    }
    (map, shapes)
}

/// The words `shapes` show.
pub(crate) fn words(shapes: &[Shape]) -> Vec<String> {
    (shapes.iter())
        .filter_map(|shape| match shape {
            Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect()
}

/// How many of `shapes` are outlines or rings in `colour`.
pub(crate) fn outlined(shapes: &[Shape], colour: Color32) -> usize {
    (shapes.iter())
        .filter(|shape| match shape {
            Shape::Path(path) => path.stroke.color == egui::epaint::ColorMode::Solid(colour),
            Shape::Circle(circle) => circle.stroke.color == colour,
            _ => false,
        })
        .count()
}

/// Passes from `time` a sixtieth of a second apart until things rest (two seconds at most);
/// the last one's controls.
pub(crate) fn settled(ctx: &Context, at: (f64, Language), add: impl FnMut(&mut Ui)) -> ControlMap {
    settled_drawn(ctx, at, add).0
}

/// As `settled`, with what the last pass drew.
pub(crate) fn settled_drawn(
    ctx: &Context,
    (time, language): (f64, Language),
    mut add: impl FnMut(&mut Ui),
) -> (ControlMap, Vec<Shape>) {
    let mut last = (ControlMap::default(), Vec::new());
    for step in 0..120 {
        last = drawn(
            ctx,
            (time + f64::from(step) / 60.0, language, Vec::new()),
            &mut add,
        );
    }
    last
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
