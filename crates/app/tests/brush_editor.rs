//! The brush editor's intents (S46): a setting changes the brush for the next stroke of every
//! tool while strokes drawn keep their recipes; rename, duplicate, reset and delete; the library
//! stored with the toolbox and taken over again.

use openpaint_app::{
    BrushChange, BrushIntent, BrushKey, BrushLibrary, Contact, ContactPhase, Device, InputSample,
    Intent, Knob, Session, StrokeIntent, Tool, ToolIntent, Toolbox,
};
use openpaint_core::{CanvasSize, ShapeTexture, Stamp};

fn toolbox() -> Toolbox {
    Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"))
}

fn inker() -> BrushKey {
    BrushKey::new("inker")
}

fn set(knob: Knob, value: f32) -> BrushIntent {
    BrushIntent::Edit {
        change: BrushChange::Set(knob, value),
        settled: true,
    }
}

fn names(toolbox: &Toolbox) -> Vec<String> {
    let brush = toolbox.projection();
    brush.brushes.iter().map(|row| row.name.clone()).collect()
}

#[test]
fn a_setting_changes_the_brush_and_reset_brings_its_file_back() {
    let mut toolbox = toolbox();
    toolbox.dispatch(&BrushIntent::Choose(inker()));
    let first = toolbox.projection().edited;
    assert!(first.built_in && !first.resettable, "as its file has it");
    assert_eq!(first.value(Knob::TaperStart), 2.0);
    toolbox.dispatch(&BrushIntent::Edit {
        change: BrushChange::Set(Knob::TaperStart, 5.0),
        settled: false,
    });
    assert_eq!(toolbox.save_request(), None, "a held slider is not stored");
    toolbox.dispatch(&set(Knob::TaperStart, 5.0));
    let edited = toolbox.projection().edited;
    assert_eq!(edited.value(Knob::TaperStart), 5.0);
    assert_eq!(
        toolbox.style().expect("style").recipe().taper().start(),
        5.0
    );
    assert!(edited.resettable);
    assert!(toolbox.save_request().is_some(), "stored");
    // Out of the control's range: held at its end, never refused.
    toolbox.dispatch(&set(Knob::TaperEnd, 99.0));
    assert_eq!(toolbox.projection().edited.value(Knob::TaperEnd), 6.0);
    toolbox.dispatch(&BrushIntent::StreamLine {
        amount: 0.5,
        settled: true,
    });
    toolbox.dispatch(&BrushIntent::Reset);
    let reset = toolbox.projection();
    assert_eq!(reset.edited.recipe, first.recipe, "the file's values");
    assert!(!reset.edited.resettable, "Reset grey again");
    assert_eq!(reset.streamline.amount(), 0.0, "the file's StreamLine");
    assert_eq!(toolbox.state().library, [], "the built-ins as their files");
}

#[test]
fn the_next_stroke_takes_the_edit_and_strokes_drawn_keep_their_recipes() {
    let mut toolbox = toolbox();
    toolbox.dispatch(&BrushIntent::Choose(inker()));
    let mut session = Session::new(
        CanvasSize::new(200, 100).expect("size"),
        toolbox.style().expect("style"),
    );
    draw(&mut session, 10.0);
    toolbox.dispatch(&set(Knob::TaperStart, 5.0));
    // As the runtime does after every brush intent: the session draws with the new style.
    session.set_brush(toolbox.style().expect("style"));
    draw(&mut session, 100.0);
    let layer = &session.artwork().layers()[0];
    let strokes: Vec<_> = layer.strokes().iter().collect();
    assert_eq!(
        strokes[0].style().recipe().taper().start(),
        2.0,
        "drawn before"
    );
    assert_eq!(
        strokes[1].style().recipe().taper().start(),
        5.0,
        "drawn after"
    );
    // The eraser and the smudge on the Inker take the edit too.
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Eraser));
    toolbox.dispatch(&BrushIntent::Choose(inker()));
    assert_eq!(
        toolbox.style().expect("style").recipe().taper().start(),
        5.0
    );
}

fn draw(session: &mut Session, x: f32) {
    for (phase, dx, time) in [
        (ContactPhase::Down, 0.0, 0),
        (ContactPhase::Move, 30.0, 8),
        (ContactPhase::Up, 60.0, 16),
    ] {
        let sample = InputSample {
            position: [x + dx, 40.0],
            pressure: 0.7,
            tilt: 0.0,
            azimuth: 0.0,
            time_ms: time,
        };
        let contact = Contact {
            id: 1,
            device: Device::Pen,
            phase,
            samples: vec![sample],
        };
        let intent = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(intent).expect("pen");
    }
}

#[test]
fn a_copy_comes_after_its_source_chosen_and_deleting_it_chooses_the_source() {
    let mut toolbox = toolbox();
    toolbox.dispatch(&BrushIntent::Choose(inker()));
    toolbox.dispatch(&BrushIntent::Size {
        position: 0.5,
        settled: true,
    });
    let before = toolbox.projection();
    toolbox.dispatch(&BrushIntent::Duplicate);
    let copy = toolbox.projection();
    assert_eq!(copy.edited.name, "Inker copy");
    assert!(!copy.edited.built_in && !copy.edited.resettable);
    assert_ne!(copy.current, before.current, "a key of its own");
    assert_eq!(copy.edited.recipe, before.edited.recipe, "the same values");
    assert_eq!((copy.size, copy.opacity), (before.size, before.opacity));
    let listed = names(&toolbox);
    let at = listed
        .iter()
        .position(|name| name == "Inker")
        .expect("Inker");
    assert_eq!(listed[at + 1], "Inker copy", "right after its source");
    toolbox.dispatch(&set(Knob::Spacing, 0.4));
    toolbox.dispatch(&BrushIntent::Edit {
        change: BrushChange::Stamp(Stamp::Texture(ShapeTexture::Chalk)),
        settled: true,
    });
    let chalk = toolbox.projection().edited;
    assert_eq!(chalk.stamp(), Some(Stamp::Texture(ShapeTexture::Chalk)));
    assert!((chalk.value(Knob::Spacing) - 0.4).abs() < 1e-6);
    let inker_now = toolbox.library().brush(&inker()).expect("inker");
    assert!(
        !inker_now.recipe().shape().is_stamped(),
        "the source keeps its own"
    );
    toolbox.dispatch(&BrushIntent::Rename("  Chalk liner ".into()));
    assert_eq!(toolbox.projection().edited.name, "Chalk liner");
    toolbox.dispatch(&BrushIntent::Rename("   ".into()));
    assert_eq!(
        toolbox.projection().edited.name,
        "Chalk liner",
        "no name: kept"
    );
    toolbox.dispatch(&BrushIntent::Delete);
    let after = toolbox.projection();
    assert_eq!(after.current, inker(), "its source chosen");
    assert_eq!(
        names(&toolbox),
        names(&Toolbox::new(BrushLibrary::built_in().expect("files")))
    );
    toolbox.dispatch(&BrushIntent::Delete);
    assert_eq!(toolbox.projection().current, inker(), "a built-in stays");
}

#[test]
fn the_library_round_trips_through_the_stored_toolbox() {
    let mut toolbox = toolbox();
    toolbox.dispatch(&BrushIntent::Choose(inker()));
    toolbox.dispatch(&set(Knob::TaperStart, 4.0));
    toolbox.dispatch(&BrushIntent::Duplicate);
    toolbox.dispatch(&set(Knob::Spacing, 0.4));
    toolbox.dispatch(&BrushIntent::StreamLine {
        amount: 0.25,
        settled: true,
    });
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Smudge));
    let state = toolbox.state();
    assert_eq!(state.library.len(), 11);
    let edited = state
        .library
        .iter()
        .filter(|brush| brush.brush.is_some())
        .count();
    assert_eq!(edited, 2, "the Inker and its copy whole, the others by key");
    let mut reopened = self::toolbox();
    assert!(reopened.restore(&state));
    assert!(reopened.fell_back().is_empty());
    assert_eq!(reopened.state(), state);
    assert_eq!(reopened.projection(), toolbox.projection());
    reopened.dispatch_tool(&ToolIntent::Use(Tool::Brush));
    let copy = reopened.projection();
    assert_eq!(copy.edited.name, "Inker copy");
    assert_eq!(copy.streamline.amount(), 0.25);
}
