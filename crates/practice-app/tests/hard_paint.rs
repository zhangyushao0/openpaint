//! Hard Paint (S104): the practice's hard tip in the toolbox, from one definition. It opens the
//! Painting set with V大's other three; toolboxes kept before get it where a first start puts it,
//! their arrangement kept; a long colour copy opens on it at a size for its frame, in the full
//! interface, with no pen of its own.

use std::sync::Arc;

use openpaint_app::{
    BrushIntent, BrushKey, BrushLibrary, BrushSet, Contact, ContactPhase, Device, HARD_PAINT,
    InputSample, Intent, Session, StrokeIntent, Title, Tool, ToolIntent, Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, Deposit, StrokeStyle};
use openpaint_practice::{
    Exercise, ItemRef, Kind, LineTier, Pack, PackItem, PictureKind, Plan, Sheet, Tally, Tier,
};
use openpaint_practice_app::{
    Pen, PracticeIntent, PracticeLog, PracticePens, PracticeProjection, SheetView, copy_brush,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
const NOW: u64 = 1_792_000_000;

fn toolbox() -> Toolbox {
    Toolbox::new(BrushLibrary::built_in().expect("built-in brushes"))
}

fn hard_paint() -> BrushKey {
    BrushKey::new(HARD_PAINT)
}

/// The names `set` lists now.
fn listed(toolbox: &Toolbox, set: BrushSet) -> Vec<String> {
    let brush = toolbox.projection();
    brush.rows(set).iter().map(|row| row.name.clone()).collect()
}

#[test]
fn hard_paint_is_the_practice_s_hard_tip_and_opens_the_painting_set_with_v_s_four() {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let brush = library.brush(&hard_paint()).expect("a built-in");
    assert_eq!((brush.name(), brush.is_built_in()), ("Hard Paint", true));
    let pens = PracticePens::built_in().expect("pens");
    for pen in [Pen::Big, Pen::Small, Pen::Dark, Pen::Middle, Pen::Light] {
        assert_eq!(
            pens.style(pen).recipe(),
            brush.recipe(),
            "{pen:?}: one definition"
        );
    }
    // Round, a hard edge, full opacity at any pressure; pressure on its size only.
    let recipe = brush.recipe();
    assert!(recipe.softness() == 0.0 && !recipe.shape().is_stamped());
    assert!(matches!(recipe.deposit(), Deposit::Cover));
    assert_eq!(brush.opacity(), 1.0);
    for pressure in [0.0, 0.02, 0.5, 1.0] {
        assert_eq!(recipe.opacity().at(pressure, 6.0, 0.8), 1.0, "{pressure}");
    }
    assert!((recipe.size().at(0.0, 0.0, 0.0) - 0.6).abs() < 1e-6);
    let toolbox = toolbox();
    assert_eq!(
        listed(&toolbox, BrushSet::Painting),
        [
            "Hard Paint",
            "Hard Round",
            "Soft Brush",
            "Textured Round",
            "Flat",
            "Opaque Paint"
        ]
    );
    assert_eq!(listed(&toolbox, BrushSet::Texture), ["Speckle"]);
}

/// A toolbox as 1.18.0 kept it: the library before Hard Paint (a copy of Hard Round made, so it
/// is stored brush by brush), its sets arranged (Opaque Paint first in Painting, Textured Round in
/// Texture as it lay then), Recent, and the brush tool on Opaque Paint at a size of its own.
fn kept_by_1_18_0() -> openpaint_app::EditorState {
    let mut toolbox = toolbox();
    toolbox.dispatch(&BrushIntent::Choose(BrushKey::new("hard-round")));
    toolbox.dispatch(&BrushIntent::Duplicate);
    toolbox.dispatch(&BrushIntent::Choose(BrushKey::new("opaque-paint")));
    toolbox.dispatch(&BrushIntent::Size {
        position: 0.4,
        settled: true,
    });
    toolbox.drew(openpaint_core::StrokeTool::Paint);
    let mut state = toolbox.state();
    let older = |key: &String| key != HARD_PAINT;
    state.library.retain(|entry| older(&entry.key));
    state.paint.brushes.retain(|setting| older(&setting.key));
    state.erase.brushes.retain(|setting| older(&setting.key));
    state.smudge.brushes.retain(|setting| older(&setting.key));
    let keys = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect();
    state.sets = [
        keys(&["pencil"]),
        keys(&["ink-pen", "inker", "monoline"]),
        keys(&["opaque-paint", "hard-round", "copy-1", "soft-brush", "flat"]),
        keys(&["textured-round", "speckle"]),
    ];
    assert!(!state.library.is_empty() && state.library.len() == 11);
    state
}

#[test]
fn a_toolbox_kept_before_gets_hard_paint_first_in_painting_and_keeps_its_arrangement() {
    let kept = kept_by_1_18_0();
    let mut reopened = toolbox();
    assert!(reopened.restore(&kept), "everything taken over");
    assert!(reopened.dropped().is_empty() && reopened.fell_back().is_empty());
    let keys: Vec<&str> = (reopened.library().brushes().iter())
        .map(|brush| brush.key().as_str())
        .collect();
    assert_eq!(&keys[3..5], ["monoline", HARD_PAINT], "where its file lies");
    assert_eq!(
        listed(&reopened, BrushSet::Painting),
        [
            "Hard Paint",
            "Opaque Paint",
            "Hard Round",
            "Hard Round copy",
            "Soft Brush",
            "Flat"
        ]
    );
    assert_eq!(
        listed(&reopened, BrushSet::Texture),
        ["Textured Round", "Speckle"],
        "the arrangement kept"
    );
    assert_eq!(listed(&reopened, BrushSet::Recent), ["Opaque Paint"]);
    let brush = reopened.projection();
    assert_eq!(brush.current, BrushKey::new("opaque-paint"), "still chosen");
    assert!(
        (brush.size - 0.16f32.mul_add(398.0, 2.0)).abs() < 1e-3,
        "its size kept"
    );
    // A toolbox nobody arranged takes the new sets whole.
    let unarranged = openpaint_app::EditorState {
        sets: Default::default(),
        ..kept
    };
    let mut reopened = toolbox();
    assert!(reopened.restore(&unarranged));
    assert_eq!(
        listed(&reopened, BrushSet::Painting)[..4],
        ["Hard Paint", "Hard Round", "Hard Round copy", "Soft Brush"]
    );
    assert_eq!(listed(&reopened, BrushSet::Texture), ["Speckle"]);
}

fn vase_pack() -> Pack {
    Pack {
        id: "test-shapes".to_owned(),
        title: "Test shapes".to_owned(),
        items: vec![PackItem {
            id: "vase".to_owned(),
            kind: PictureKind::Shape,
            tier: Tier::Medium,
            size: [600, 800],
            title: "Vase".to_owned(),
            credit: None,
            outline: None,
            masses: None,
            areas: None,
        }],
    }
}

/// `record`'s sheet as it opens: its canvas and what the session opens with.
fn sheet(record: Sheet) -> (Artwork, openpaint_practice_app::Practising) {
    let packs = [vase_pack()];
    let record = record.framed(&packs);
    let [width, height] = openpaint_practice::Layout::of_sheet(&record).size;
    let artwork = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let view = SheetView::of(&record, &packs, ([width, height], &Title::untitled(), 1));
    (artwork, (record, Vec::new(), view, Tally::default()))
}

/// A pen stroke from `at` 60 pixels right at `pressure`.
fn draw(session: &mut Session, at: [f32; 2], pressure: f32) {
    for (phase, dx, time_ms) in [
        (ContactPhase::Down, 0.0, 1000),
        (ContactPhase::Move, 30.0, 1008),
        (ContactPhase::Up, 60.0, 1016),
    ] {
        let sample = InputSample {
            position: [at[0] + dx, at[1]],
            pressure,
            tilt: 0.2,
            azimuth: 0.0,
            time_ms,
        };
        let contact = Contact {
            id: 9,
            device: Device::Pen,
            phase,
            samples: vec![sample],
        };
        let intent = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(intent).expect("dispatch");
    }
}

/// A long colour copy of the vase as it opens: its canvas and what the session opens with.
fn copy_of_the_vase() -> (Artwork, openpaint_practice_app::Practising) {
    let log = PracticeLog::new(Vec::new(), Vec::new(), vec![vase_pack()]);
    let vase = ItemRef {
        pack: "test-shapes".to_owned(),
        item: "vase".to_owned(),
    };
    let (record, _) = log.begin_copy(&vase, 1, 2).expect("a picture");
    sheet(record)
}

#[test]
fn a_long_copy_opens_on_hard_paint_at_a_size_for_its_frame_as_the_hand_would_choose_it() {
    let (key, size) = copy_brush(&copy_of_the_vase().1).expect("a copy opens on a brush");
    // As large to its 1,600-pixel frame as Big is to an exercise's 400.
    assert_eq!((key.as_str(), size), (HARD_PAINT, 256.0));
    let lines = Plan {
        exercises: vec![Exercise::generated(Kind::Line(LineTier::Arc), 7)],
        warm_up: 0,
    };
    let session = Sheet::session(1, 1, 0, lines);
    assert_eq!(copy_brush(&sheet(session).1), None, "a session's sheet");
    // The brush tool, Hard Paint, its size; the eraser chosen before.
    let mut toolbox = toolbox();
    toolbox.dispatch_tool(&ToolIntent::Use(Tool::Eraser));
    toolbox.save_request();
    toolbox.paint_with(&key, size);
    let brush = toolbox.projection();
    let chosen = (brush.tool, brush.current.clone(), brush.size, brush.set);
    assert_eq!(
        chosen,
        (Tool::Brush, key.clone(), 256.0, BrushSet::Painting)
    );
    assert!(toolbox.save_request().is_some(), "stored");
    // Opened again after a size of one's own: at the size for its frame again.
    toolbox.dispatch(&BrushIntent::Size {
        position: 0.2,
        settled: true,
    });
    toolbox.paint_with(&key, size);
    assert_eq!(toolbox.projection().size, 256.0);
}

#[test]
fn a_long_copy_is_painted_with_the_toolbox_s_brush_in_the_full_interface_only() {
    let (artwork, opening) = copy_of_the_vase();
    let (key, size) = copy_brush(&opening).expect("a copy opens on a brush");
    let mut toolbox = toolbox();
    toolbox.paint_with(&key, size);
    // The colour picked, a light stroke: Hard Paint at full opacity in it, no pen of the sheet's.
    toolbox.dispatch_tool(&ToolIntent::Color {
        color: Color::rgb(0.6, 0.2, 0.3).expect("colour"),
        settled: true,
    });
    let colour = toolbox.projection().color;
    let style = toolbox.style().expect("style");
    let pens = PracticePens::built_in().expect("pens");
    let mut session =
        openpaint_practice_app::open_sheet(artwork, (style, pens.clone()), opening, &VIEWPORT);
    let practice = PracticeProjection::of(&session).expect("a sheet");
    assert!(practice.all_tools && practice.pen.is_none());
    let at = practice.sheet.pairs[0].copy.to_canvas([0.4, 0.5]);
    draw(&mut session, at.map(|v| v as f32), 0.02);
    let strokes = session
        .artwork()
        .layers()
        .iter()
        .flat_map(|l| l.strokes().iter());
    let stroke = strokes.last().expect("painted").style().clone();
    assert_eq!(stroke.recipe(), pens.style(Pen::Big).recipe());
    let kept = (stroke.size(), stroke.opacity(), stroke.color());
    assert_eq!(kept, (256.0, 1.0, colour));
    // No tools of its own: one way of painting.
    for refused in [
        PracticeIntent::AllTools(false),
        PracticeIntent::Pen(Pen::Big),
    ] {
        assert!(
            openpaint_practice_app::practise(&mut session, refused, NOW).is_err(),
            "{refused:?}"
        );
    }
    let own = StrokeStyle::new(Arc::clone(stroke.recipe()), 80.0, 1.0, colour);
    session.set_brush(own.expect("style"));
    assert_eq!(PracticeProjection::of(&session).and_then(|p| p.pen), None);
}
