//! Brush sets (S47): the sets a first start shows, a copy right after its source, Recent updated
//! by every committed stroke of any tool (newest first, no repeats, six at most), a carried row
//! landing at another place of its set, the editor's Set row moving a brush to another set's end,
//! the chosen brush kept throughout, and the sets and Recent through the stored toolbox.

use openpaint_app::{
    BrushIntent, BrushKey, BrushLibrary, BrushSet, Contact, ContactPhase, Device, InputSample,
    Intent, RECENT_BRUSHES, Session, StrokeIntent, Tool, ToolIntent, Toolbox,
};
use openpaint_core::CanvasSize;

fn toolbox() -> Toolbox {
    Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"))
}

fn key(key: &str) -> BrushKey {
    BrushKey::new(key)
}

/// The names `set` lists now.
fn listed(toolbox: &Toolbox, set: BrushSet) -> Vec<String> {
    let brush = toolbox.projection();
    brush.rows(set).iter().map(|row| row.name.clone()).collect()
}

fn choose(toolbox: &mut Toolbox, brush: &str) {
    toolbox.dispatch(&BrushIntent::Choose(key(brush)));
}

#[test]
fn a_first_start_puts_every_built_in_in_the_set_of_its_job() {
    let toolbox = toolbox();
    let sets: Vec<(BrushSet, Vec<String>)> = BrushSet::ALL
        .into_iter()
        .map(|set| (set, listed(&toolbox, set)))
        .collect();
    let expected = [
        (BrushSet::Recent, vec![]),
        (BrushSet::Sketching, vec!["Pencil"]),
        (BrushSet::Inking, vec!["Ink Pen", "Inker", "Monoline"]),
        (
            BrushSet::Painting,
            vec!["Hard Round", "Soft Brush", "Flat", "Opaque Paint"],
        ),
        (BrushSet::Texture, vec!["Textured Round", "Speckle"]),
    ];
    let expected: Vec<(BrushSet, Vec<String>)> = expected
        .into_iter()
        .map(|(set, names)| (set, names.into_iter().map(String::from).collect()))
        .collect();
    assert_eq!(sets, expected);
    assert_eq!(
        BrushSet::ALL.map(BrushSet::label),
        ["Recent", "Sketching", "Inking", "Painting", "Texture"]
    );
    let brush = toolbox.projection();
    assert_eq!(
        brush.set,
        BrushSet::Inking,
        "the Ink Pen's: the picker opens there"
    );
    let mut smudge = toolbox.clone();
    smudge.dispatch_tool(&ToolIntent::Use(Tool::Smudge));
    assert_eq!(
        smudge.projection().set,
        BrushSet::Painting,
        "the Soft Brush's"
    );
    let state = toolbox.state();
    assert_eq!(
        state.sets,
        <[Vec<String>; 4]>::default(),
        "stored as nothing"
    );
    assert!(state.recent_brushes.is_empty());
}

#[test]
fn a_copy_lies_right_after_its_source_and_leaves_with_its_deletion() {
    let mut toolbox = toolbox();
    choose(&mut toolbox, "speckle");
    toolbox.dispatch(&BrushIntent::MoveToSet(BrushSet::Painting));
    toolbox.dispatch(&BrushIntent::Reorder {
        key: key("speckle"),
        to: 1,
    });
    toolbox.dispatch(&BrushIntent::Duplicate);
    let copy = toolbox.projection();
    assert_eq!(copy.set, BrushSet::Painting, "its source's set");
    assert_eq!(
        listed(&toolbox, BrushSet::Painting),
        [
            "Hard Round",
            "Speckle",
            "Speckle copy",
            "Soft Brush",
            "Flat",
            "Opaque Paint"
        ]
    );
    toolbox.drew(openpaint_core::StrokeTool::Paint);
    assert_eq!(listed(&toolbox, BrushSet::Recent), ["Speckle copy"]);
    toolbox.dispatch(&BrushIntent::Delete);
    assert_eq!(toolbox.projection().current, key("speckle"), "its source");
    assert!(!listed(&toolbox, BrushSet::Painting).contains(&"Speckle copy".to_owned()));
    assert!(
        listed(&toolbox, BrushSet::Recent).is_empty(),
        "gone from Recent too"
    );
}

/// A stroke from `x`, committed (or, `cancelled`, taken back by a palm), and what the runtime
/// does after it: the committed stroke's tool goes to the toolbox.
fn stroke(session: &mut Session, toolbox: &mut Toolbox, x: f32, cancelled: bool) {
    session.set_brush(toolbox.style().expect("style"));
    let last = if cancelled {
        ContactPhase::Cancel
    } else {
        ContactPhase::Up
    };
    for (phase, dx, time) in [
        (ContactPhase::Down, 0.0, 0),
        (ContactPhase::Move, 20.0, 8),
        (last, 40.0, 16),
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
        if let Some(tool) = session.stroke_committed() {
            assert_eq!(phase, ContactPhase::Up, "only a lift commits");
            toolbox.drew(tool);
        }
    }
}

#[test]
fn recent_lists_the_brushes_of_committed_strokes_of_any_tool_newest_first() {
    let mut toolbox = toolbox();
    let canvas = CanvasSize::new(400, 100).expect("size");
    let mut session = Session::new(canvas, toolbox.style().expect("style"));
    stroke(&mut session, &mut toolbox, 10.0, false);
    assert_eq!(listed(&toolbox, BrushSet::Recent), ["Ink Pen"]);
    assert!(toolbox.save_request().is_some(), "stored");
    choose(&mut toolbox, "pencil");
    toolbox.save_request();
    stroke(&mut session, &mut toolbox, 60.0, true);
    assert_eq!(
        listed(&toolbox, BrushSet::Recent),
        ["Ink Pen"],
        "a cancelled stroke"
    );
    stroke(&mut session, &mut toolbox, 60.0, false);
    stroke(&mut session, &mut toolbox, 110.0, false);
    assert_eq!(listed(&toolbox, BrushSet::Recent), ["Pencil", "Ink Pen"]);
    toolbox.save_request();
    stroke(&mut session, &mut toolbox, 160.0, false);
    assert_eq!(
        toolbox.save_request(),
        None,
        "the same brush again: nothing new"
    );
    // The eraser (on the Ink Pen at first) and the smudge (on the Soft Brush) count too.
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Eraser));
    stroke(&mut session, &mut toolbox, 210.0, false);
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Smudge));
    stroke(&mut session, &mut toolbox, 260.0, false);
    assert_eq!(
        listed(&toolbox, BrushSet::Recent),
        ["Soft Brush", "Ink Pen", "Pencil"],
        "no repeats"
    );
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Brush));
    for brush in ["flat", "speckle", "inker", "monoline", "pencil"] {
        choose(&mut toolbox, brush);
        stroke(&mut session, &mut toolbox, 300.0, false);
    }
    let recent = listed(&toolbox, BrushSet::Recent);
    assert_eq!(recent.len(), RECENT_BRUSHES);
    assert_eq!(
        recent,
        [
            "Pencil",
            "Monoline",
            "Inker",
            "Speckle",
            "Flat",
            "Soft Brush"
        ]
    );
    // Tapping a Recent row chooses the brush; its own set stays where it lies.
    let brush = toolbox.projection();
    let flat = brush.rows(BrushSet::Recent)[4].key.clone();
    toolbox.dispatch(&BrushIntent::Choose(flat));
    let brush = toolbox.projection();
    assert_eq!(brush.current, key("flat"));
    assert_eq!(brush.set, BrushSet::Painting);
}

#[test]
fn a_carried_row_lands_where_it_is_let_go_and_where_it_was_changes_nothing() {
    let mut toolbox = toolbox();
    toolbox.save_request();
    let reorder = |toolbox: &mut Toolbox, brush: &str, to| {
        toolbox.dispatch(&BrushIntent::Reorder {
            key: key(brush),
            to,
        });
    };
    reorder(&mut toolbox, "soft-brush", 1);
    assert_eq!(toolbox.save_request(), None, "where it was: nothing");
    reorder(&mut toolbox, "opaque-paint", 0);
    assert_eq!(
        listed(&toolbox, BrushSet::Painting),
        ["Opaque Paint", "Hard Round", "Soft Brush", "Flat"]
    );
    assert!(toolbox.save_request().is_some(), "stored");
    reorder(&mut toolbox, "opaque-paint", 99);
    assert_eq!(
        listed(&toolbox, BrushSet::Painting),
        ["Hard Round", "Soft Brush", "Flat", "Opaque Paint"],
        "past the end: last"
    );
    reorder(&mut toolbox, "speckle", 0);
    assert_eq!(
        listed(&toolbox, BrushSet::Texture),
        ["Speckle", "Textured Round"],
        "within its own set"
    );
    reorder(&mut toolbox, "charcoal", 0);
    assert_eq!(
        listed(&toolbox, BrushSet::Inking),
        ["Ink Pen", "Inker", "Monoline"]
    );
    assert_eq!(toolbox.projection().current, key("ink-pen"), "still chosen");
}

#[test]
fn the_set_row_moves_the_brush_to_the_end_of_another_set_and_it_stays_chosen() {
    let mut toolbox = toolbox();
    choose(&mut toolbox, "speckle");
    toolbox.save_request();
    for unmoved in [BrushSet::Recent, BrushSet::Texture] {
        toolbox.dispatch(&BrushIntent::MoveToSet(unmoved));
        assert_eq!(toolbox.save_request(), None, "{unmoved:?}: nothing");
    }
    toolbox.dispatch(&BrushIntent::MoveToSet(BrushSet::Painting));
    let brush = toolbox.projection();
    assert_eq!(
        (brush.current, brush.set),
        (key("speckle"), BrushSet::Painting)
    );
    assert_eq!(listed(&toolbox, BrushSet::Texture), ["Textured Round"]);
    assert_eq!(
        listed(&toolbox, BrushSet::Painting)
            .last()
            .map(String::as_str),
        Some("Speckle")
    );
    toolbox.dispatch(&BrushIntent::MoveToSet(BrushSet::Sketching));
    assert_eq!(listed(&toolbox, BrushSet::Sketching), ["Pencil", "Speckle"]);
    // The eraser's brush is its own: the Ink Pen, in Inking, where its picker opens.
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Eraser));
    assert_eq!(toolbox.projection().set, BrushSet::Inking);
}

#[test]
fn sets_order_and_recent_come_back_through_the_stored_toolbox() {
    let mut toolbox = toolbox();
    choose(&mut toolbox, "speckle");
    toolbox.drew(openpaint_core::StrokeTool::Paint);
    toolbox.dispatch(&BrushIntent::MoveToSet(BrushSet::Painting));
    toolbox.dispatch(&BrushIntent::Duplicate);
    toolbox.dispatch(&BrushIntent::Reorder {
        key: key("flat"),
        to: 0,
    });
    choose(&mut toolbox, "monoline");
    toolbox.drew(openpaint_core::StrokeTool::Paint);
    let state = toolbox.state();
    assert_eq!(state.sets[2].last().map(String::as_str), Some("copy-1"));
    assert_eq!(state.recent_brushes, ["monoline", "speckle"]);
    let mut reopened = self::toolbox();
    assert!(reopened.restore(&state));
    assert!(reopened.dropped().is_empty());
    assert_eq!(reopened.state(), state);
    assert_eq!(reopened.projection(), toolbox.projection());
}

#[test]
fn unknown_keys_are_dropped_and_a_brush_in_no_set_goes_where_a_first_start_puts_it() {
    let mut toolbox = toolbox();
    choose(&mut toolbox, "inker");
    toolbox.dispatch(&BrushIntent::Duplicate);
    let mut state = toolbox.state();
    let keys = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect();
    // Pencil and Monoline in no set; the Speckle twice; a key no brush has.
    state.sets = [
        keys(&[]),
        keys(&["charcoal", "copy-1", "inker", "ink-pen"]),
        keys(&[
            "speckle",
            "hard-round",
            "soft-brush",
            "flat",
            "opaque-paint",
        ]),
        keys(&["textured-round", "speckle"]),
    ];
    // Recent: a key no brush has, a repeat, one past six.
    state.recent_brushes = keys(&[
        "ink-pen",
        "charcoal",
        "ink-pen",
        "pencil",
        "flat",
        "inker",
        "speckle",
        "monoline",
        "hard-round",
    ]);
    let mut reopened = self::toolbox();
    assert!(!reopened.restore(&state));
    assert_eq!(
        reopened.dropped(),
        ["charcoal", "speckle", "charcoal", "ink-pen", "hard-round"]
    );
    assert_eq!(listed(&reopened, BrushSet::Sketching), ["Pencil"]);
    // Monoline right after the brush before it in the library's order that lies there.
    assert_eq!(
        listed(&reopened, BrushSet::Inking),
        ["Inker copy", "Monoline", "Inker", "Ink Pen"]
    );
    assert_eq!(listed(&reopened, BrushSet::Texture), ["Textured Round"]);
    assert_eq!(
        listed(&reopened, BrushSet::Recent),
        ["Ink Pen", "Pencil", "Flat", "Inker", "Speckle", "Monoline"]
    );
    // A copy whose source is gone comes from the default brush, and lies in Painting.
    let mut state = toolbox.state();
    let copy = state
        .library
        .iter_mut()
        .find(|brush| brush.key == "copy-1")
        .and_then(|brush| brush.brush.as_mut())
        .expect("the copy whole");
    copy.source = "charcoal".into();
    let mut orphaned = self::toolbox();
    orphaned.restore(&state);
    assert!(listed(&orphaned, BrushSet::Painting).contains(&"Inker copy".to_owned()));
    assert!(!listed(&orphaned, BrushSet::Inking).contains(&"Inker copy".to_owned()));
}
