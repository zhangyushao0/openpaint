//! The brush editor's controls (S46): every setting has its control under its name, each slider
//! gives the ends of the range the engine allows, the chosen shape's word carries the pill and
//! the paper leaves only the disc, and Reset is grey while a built-in is as its file has it.

use openpaint_app::{
    BrushChange, BrushIntent, BrushKey, BrushLibrary, BrushProjection, Intent, Knob, Toolbox,
};
use openpaint_core::{Rotation, ShapeTexture, Stamp};

use super::{settings, shown, stamp_words};
use crate::brush_editor::BrushEditor;
use crate::components::{ControlMap, controls};
use crate::design;

const KNOBS: [Knob; 16] = [
    Knob::Spacing,
    Knob::TaperStart,
    Knob::TaperEnd,
    Knob::TaperTip,
    Knob::Aspect,
    Knob::Angle,
    Knob::RotationJitter,
    Knob::SizeJitter,
    Knob::Scatter,
    Knob::Softness,
    Knob::GrainStrength,
    Knob::GrainScale,
    Knob::Flow,
    Knob::MaxDensity,
    Knob::SizeMin,
    Knob::SizeMax,
];

fn toolbox(key: &str) -> Toolbox {
    let mut toolbox = Toolbox::new(BrushLibrary::built_in().expect("brushes"));
    toolbox.dispatch(&BrushIntent::Choose(BrushKey::new(key)));
    toolbox
}

/// One pass of `draw` on a screen a panel wide and tall enough for every row, with `events`:
/// what it asked for and the controls it laid out.
fn pass(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    mut draw: impl FnMut(&mut egui::Ui, &mut Vec<Intent>),
) -> (Vec<Intent>, ControlMap) {
    let screen =
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(design::PANEL_WIDTH, 4000.0));
    // A second apart, so two taps at one place are two taps, not a double one.
    let time = ctx.input(|input| input.time) + 1.0;
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
    (taps, map)
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    design::install(&ctx);
    ctx
}

/// A tap at `[x, y]` (points: the context draws one pixel per point).
fn tap([x, y]: [f32; 2]) -> Vec<egui::Event> {
    let at = egui::pos2(x, y);
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    vec![egui::Event::PointerMoved(at), button(true), button(false)]
}

fn columns(
    ctx: &egui::Context,
    brush: &BrushProjection,
    events: Vec<egui::Event>,
) -> (Vec<Intent>, ControlMap) {
    pass(ctx, events, |ui, taps| {
        settings(ui, brush, taps);
    })
}

#[test]
fn every_setting_has_its_control_and_its_value_reads_in_its_units() {
    let ctx = context();
    let brush = toolbox("speckle").projection();
    let (_, map) = columns(&ctx, &brush, Vec::new());
    let mut names: Vec<&str> = KNOBS.iter().map(|knob| knob.label()).collect();
    names.extend([
        "StreamLine",
        "Disc",
        "Rough square",
        "Fixed",
        "Direction",
        "Azimuth",
    ]);
    names.extend(["Covering", "Builds up", "Flat", "Bell", "None", "Sketch"]);
    names.extend(["Size dynamics", "Opacity dynamics"]);
    for name in names {
        assert!(map.find(name).is_ok(), "{name}: {:?}", map.names());
    }
    let reads = [
        (Knob::Spacing, 0.0, "Continuous"),
        (Knob::Spacing, 0.4, "40 %"),
        (Knob::TaperStart, 5.0, "5.0 × size"),
        (Knob::Angle, -30.0, "-30°"),
        (Knob::RotationJitter, 15.0, "± 15°"),
        (Knob::SizeJitter, 0.5, "± 50 %"),
        (Knob::Scatter, 3.0, "3.0 × radius"),
        (Knob::GrainScale, 5.0, "5 px"),
        (Knob::Flow, 1.2, "1.20"),
        (Knob::SizeMin, 2.5, "2.5 px"),
        (Knob::SizeMax, 400.0, "400 px"),
    ];
    for (knob, value, text) in reads {
        assert_eq!(shown(knob, value), text);
    }
}

#[test]
fn each_slider_gives_the_ends_of_the_range_the_engine_allows() {
    let ctx = context();
    let brush = toolbox("speckle").projection();
    let (_, map) = columns(&ctx, &brush, Vec::new());
    for knob in KNOBS {
        let Some(range) = brush.edited.range(knob) else {
            continue;
        };
        let control = map.find(knob.label()).expect("a control");
        for (fraction, end) in [(0.999, range.max), (0.001, range.min)] {
            let (taps, _) = columns(&ctx, &brush, tap(control.at([fraction, 0.5])));
            let asked = Intent::Brush(BrushIntent::Edit {
                change: BrushChange::Set(knob, end),
                settled: true,
            });
            assert_eq!(taps, [asked], "{knob:?} at {fraction}");
        }
    }
    let continuous = toolbox("inker").projection();
    assert_eq!(
        continuous.edited.range(Knob::Scatter),
        None,
        "grey: no stamps"
    );
    let (_, map) = columns(&ctx, &continuous, Vec::new());
    let scatter = map.find("Scatter").expect("still listed");
    let (taps, _) = columns(&ctx, &continuous, tap(scatter.at([0.9, 0.5])));
    assert!(taps.is_empty(), "a grey slider takes nothing");
}

#[test]
fn the_chosen_shape_carries_the_pill_and_the_paper_leaves_only_the_disc() {
    let chosen = |key: &str| {
        let brush = toolbox(key).projection();
        let words = stamp_words(&brush.edited);
        let on: Vec<Stamp> = words.iter().filter(|w| w.2).map(|w| w.0).collect();
        let offered = words.iter().filter(|w| w.3).count();
        (on, offered, brush.edited.rotation())
    };
    let chalk = Stamp::Texture(ShapeTexture::Chalk);
    assert_eq!(
        chosen("textured-round"),
        (vec![chalk], 7, Some(Rotation::Fixed))
    );
    assert_eq!(
        chosen("flat"),
        (vec![Stamp::Square], 7, Some(Rotation::Direction))
    );
    assert_eq!(
        chosen("ink-pen"),
        (vec![Stamp::Disc], 7, None),
        "continuous: the disc"
    );
    assert_eq!(
        chosen("pencil"),
        (vec![Stamp::Disc], 1, None),
        "on paper: only the disc"
    );
}

#[test]
fn reset_is_grey_while_the_built_in_is_as_its_file_has_it() {
    let ctx = context();
    let mut toolbox = toolbox("inker");
    let mut editor = BrushEditor::default();
    let mut page = |toolbox: &Toolbox, events| {
        let brush = toolbox.projection();
        pass(&ctx, events, |ui, taps| {
            editor.page(ui, &brush, taps);
        })
    };
    // The column knows its height from the second pass on.
    page(&toolbox, Vec::new());
    let (_, map) = page(&toolbox, Vec::new());
    let reset = map
        .find("Reset")
        .expect("a built-in has Reset")
        .at([0.5, 0.5]);
    assert!(map.find("Delete").is_err(), "a built-in is never deleted");
    let (taps, _) = page(&toolbox, tap(reset));
    assert!(taps.is_empty(), "grey: as its file has it");
    toolbox.dispatch(&BrushIntent::Edit {
        change: BrushChange::Set(Knob::TaperStart, 5.0),
        settled: true,
    });
    // egui hits what the pass before laid out: the word takes taps from the pass after it shows.
    page(&toolbox, Vec::new());
    let (taps, _) = page(&toolbox, tap(reset));
    assert_eq!(taps, [Intent::Brush(BrushIntent::Reset)]);
    toolbox.dispatch(&BrushIntent::Duplicate);
    let (_, map) = page(&toolbox, Vec::new());
    assert!(
        map.find("Delete").is_ok() && map.find("Reset").is_err(),
        "a copy has Delete"
    );
}
