//! The brush picker's sets (S47): the five set words in two columns, the shown one changed by a
//! tap on another; Recent's hint while nothing was drawn; a held row lifting in a set the hand
//! orders and never in Recent; the editor's Set row under the brush's name, moving the brush.

use std::collections::HashMap;

use openpaint_app::{BrushIntent, BrushKey, BrushLibrary, BrushSet, Intent, Toolbox};
use openpaint_core::StrokeTool;

use super::{ListState, RECENT_EMPTY, list};
use crate::brush_editor::BrushEditor;
use crate::components::{ControlMap, controls};
use crate::design;

/// One pass of `draw` at `time` seconds on a screen a panel wide, with `events`: what it asked
/// for, the controls it laid out and every text it painted.
fn pass(
    ctx: &egui::Context,
    (time, events): (f64, Vec<egui::Event>),
    mut draw: impl FnMut(&mut egui::Ui, &mut Vec<Intent>),
) -> (Vec<Intent>, ControlMap, Vec<String>) {
    let screen =
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(design::PANEL_WIDTH, 2000.0));
    let input = egui::RawInput {
        screen_rect: Some(screen),
        events,
        time: Some(time),
        ..Default::default()
    };
    let mut taps = Vec::new();
    let mut map = ControlMap::default();
    let mut output = ctx.run_ui(input, |ui| {
        controls::begin(ui.ctx());
        draw(ui, &mut taps);
        map = controls::take(ui.ctx());
    });
    output.textures_delta.clear();
    let texts = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect();
    (taps, map, texts)
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    design::install(&ctx);
    ctx
}

fn button(at: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// A tap at `[x, y]` (points: the context draws one pixel per point).
fn tap([x, y]: [f32; 2]) -> Vec<egui::Event> {
    let at = egui::pos2(x, y);
    vec![
        egui::Event::PointerMoved(at),
        button(at, true),
        button(at, false),
    ]
}

fn toolbox() -> Toolbox {
    Toolbox::new(BrushLibrary::built_in().expect("brushes"))
}

/// The list of `toolbox` at `time` with `events`, its state in `state`.
fn shown(
    ctx: &egui::Context,
    toolbox: &Toolbox,
    state: &mut ListState,
    (time, events): (f64, Vec<egui::Event>),
) -> (Vec<Intent>, ControlMap, Vec<String>) {
    let brush = toolbox.projection();
    let textures = HashMap::new();
    pass(ctx, (time, events), |ui, taps| {
        list(ui, &brush, (state, &textures), taps);
    })
}

fn top(map: &ControlMap, name: &str) -> f32 {
    map.find(name).map_or(f32::NAN, |control| control.rect[1])
}

#[test]
fn the_sets_are_words_in_two_columns_and_a_tap_shows_another() {
    let ctx = context();
    let toolbox = toolbox();
    let mut state = ListState::default();
    let (_, map, _) = shown(&ctx, &toolbox, &mut state, (1.0, Vec::new()));
    let words = BrushSet::ALL.map(BrushSet::label);
    assert_eq!(
        top(&map, words[0]),
        top(&map, words[1]),
        "Recent, Sketching"
    );
    assert_eq!(top(&map, words[2]), top(&map, words[3]), "Inking, Painting");
    assert!(top(&map, words[2]) > top(&map, words[0]));
    assert!(
        top(&map, words[4]) > top(&map, words[2]),
        "Texture alone below"
    );
    // Opening on the chosen brush's set: the Ink Pen's.
    assert!(top(&map, "Ink Pen") > top(&map, "Texture"));
    assert!(map.find("Monoline").is_ok() && map.find("Pencil").is_err());
    let texture = map.find("Texture").expect("Texture").at([0.5, 0.5]);
    let (taps, _, _) = shown(&ctx, &toolbox, &mut state, (2.0, tap(texture)));
    assert!(taps.is_empty(), "showing a set asks the toolbox nothing");
    assert_eq!(state.shown, Some(BrushSet::Texture));
    let (_, map, _) = shown(&ctx, &toolbox, &mut state, (3.0, Vec::new()));
    assert!(map.find("Speckle").is_ok() && map.find("Ink Pen").is_err());
    let speckle = map.find("Speckle").expect("Speckle").at([0.5, 0.5]);
    let (taps, _, _) = shown(&ctx, &toolbox, &mut state, (4.0, tap(speckle)));
    assert_eq!(
        taps,
        [Intent::Brush(BrushIntent::Choose(BrushKey::new("speckle")))]
    );
    // The shown word tapped again changes nothing.
    let (_, map, _) = shown(&ctx, &toolbox, &mut state, (5.0, Vec::new()));
    let texture = map.find("Texture").expect("Texture").at([0.5, 0.5]);
    shown(&ctx, &toolbox, &mut state, (6.0, tap(texture)));
    assert_eq!(state.shown, Some(BrushSet::Texture));
}

#[test]
fn recent_says_what_comes_there_until_a_brush_was_drawn_with() {
    let ctx = context();
    let mut toolbox = toolbox();
    let mut state = ListState {
        shown: Some(BrushSet::Recent),
        drag: None,
    };
    let (_, map, texts) = shown(&ctx, &toolbox, &mut state, (1.0, Vec::new()));
    assert!(texts.iter().any(|text| text == RECENT_EMPTY), "{texts:?}");
    assert!(map.find("Ink Pen").is_err());
    toolbox.drew(StrokeTool::Paint);
    let (_, map, texts) = shown(&ctx, &toolbox, &mut state, (2.0, Vec::new()));
    assert!(!texts.iter().any(|text| text == RECENT_EMPTY));
    assert!(map.find("Ink Pen").is_ok());
}

/// Presses `name` from `t` seconds on, holds it still half a second and lets go: whether its
/// row lifted, and what the letting go asked for.
fn held(
    ctx: &egui::Context,
    toolbox: &Toolbox,
    state: &mut ListState,
    (name, t): (&str, f64),
) -> (bool, Vec<Intent>) {
    let (_, map, _) = shown(ctx, toolbox, state, (t, Vec::new()));
    let at = map.find(name).expect("the row").at([0.5, 0.5]);
    let at = egui::pos2(at[0], at[1]);
    let press = vec![egui::Event::PointerMoved(at), button(at, true)];
    shown(ctx, toolbox, state, (t + 0.1, press));
    shown(ctx, toolbox, state, (t + 0.6, Vec::new()));
    let lifted = state.drag.is_some();
    let (taps, _, _) = shown(ctx, toolbox, state, (t + 0.7, vec![button(at, false)]));
    (lifted, taps)
}

#[test]
fn a_held_row_lifts_in_a_set_and_never_in_recent() {
    let ctx = context();
    let mut toolbox = toolbox();
    toolbox.drew(StrokeTool::Paint);
    toolbox.dispatch(&BrushIntent::Choose(BrushKey::new("inker")));
    toolbox.drew(StrokeTool::Paint);
    let mut state = ListState::default();
    let (lifted, taps) = held(&ctx, &toolbox, &mut state, ("Ink Pen", 10.0));
    assert!(lifted, "Inking: it lifts");
    assert!(taps.is_empty(), "let go where it lay: nothing, not a tap");
    assert!(state.drag.is_none(), "let go");
    state.shown = Some(BrushSet::Recent);
    let (lifted, taps) = held(&ctx, &toolbox, &mut state, ("Ink Pen", 20.0));
    assert!(!lifted, "Recent: never");
    let chosen = Intent::Brush(BrushIntent::Choose(BrushKey::new("ink-pen")));
    assert_eq!(taps, [chosen], "a slow tap there chooses");
}

#[test]
fn the_set_row_lies_under_the_name_and_moves_the_brush() {
    let ctx = context();
    let mut toolbox = toolbox();
    toolbox.dispatch(&BrushIntent::Choose(BrushKey::new("speckle")));
    let mut editor = BrushEditor::default();
    let mut page = |toolbox: &Toolbox, (time, events)| {
        let brush = toolbox.projection();
        pass(&ctx, (time, events), |ui, taps| {
            editor.page(ui, &brush, taps);
        })
    };
    let (_, map, texts) = page(&toolbox, (1.0, Vec::new()));
    assert!(texts.iter().any(|text| text == "Set"), "{texts:?}");
    let name = top(&map, "Speckle");
    for word in BrushSet::EDITABLE.map(BrushSet::label) {
        assert!(top(&map, word) > name, "{word} under the name");
        assert!(
            top(&map, word) < top(&map, "Preview"),
            "{word} above the strip"
        );
    }
    assert!(map.find("Recent").is_err(), "Recent is no brush's set");
    let texture = map.find("Texture").expect("Texture").at([0.5, 0.5]);
    let (taps, _, _) = page(&toolbox, (2.0, tap(texture)));
    assert!(taps.is_empty(), "its own set: nothing");
    let painting = map.find("Painting").expect("Painting").at([0.5, 0.5]);
    let (taps, _, _) = page(&toolbox, (3.0, tap(painting)));
    let moved = Intent::Brush(BrushIntent::MoveToSet(BrushSet::Painting));
    assert_eq!(taps, [moved]);
    assert_eq!(editor.moved, Some(BrushSet::Painting), "the list shows it");
}
