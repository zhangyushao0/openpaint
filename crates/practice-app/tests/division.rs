//! Divisions and whole copies on a practice sheet (S95): a composition beside the empty frame of
//! its shape (S99); a division drawn with the levels' pens (no Sketch, Draw or Correct), its
//! picture laid exactly over the frame under its lines at Compare as strongly as the slider sets
//! (a line drawn at a known place of the picture lies on that pixel), its lines joined, counted
//! and measured against the picture's longer side, the frame's, the first numbers kept,
//! lines drawn after Compare part of it; a whole copy's division on a layer of its own, dimmed
//! under Draw's line drawing and hidden at Compare, none of it a step undo takes back; stage 4
//! from the log with and without the composition pack.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Session, StrokeIntent, Title,
    Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    DivisionCheck, DivisionCompared, Exercise, Family, Frame, ItemRef, Kind, Level, Metric, Pack,
    PackItem, PictureCompared, PictureKind, Plan, Point, Sheet, Tier, resample, value,
};
use openpaint_practice_app::{
    DIVISION_DIMMED, DIVISION_LAYER, LINES_LAYER, Pen, PracticeIntent, PracticeLog, PracticePens,
    PracticeProjection, SheetView,
};
use support::OnSheet;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
const NOW: u64 = 1_791_900_000;

/// A composition pack of two pictures, 800 by 600.
fn compositions() -> Pack {
    let item = |id: &str, tier: Tier| PackItem {
        id: id.to_owned(),
        kind: PictureKind::Composition,
        tier,
        size: [800, 600],
        title: "Test".to_owned(),
        credit: None,
        outline: None,
        masses: None,
        areas: None,
    };
    Pack {
        id: "test-compositions".to_owned(),
        title: "Test compositions".to_owned(),
        items: vec![item("harbour", Tier::Simple), item("hills", Tier::Medium)],
    }
}

fn opened(kind: Kind) -> Session {
    let packs = [compositions()];
    let exercise = Exercise {
        picture: Some(ItemRef {
            pack: "test-compositions".to_owned(),
            item: "harbour".to_owned(),
        }),
        ..Exercise::generated(kind, 0x5EED_0095)
    };
    let plan = Plan {
        exercises: vec![exercise],
        warm_up: 0,
    };
    let record = Sheet::session(1, 4, 0, plan);
    let record = record.framed(&packs);
    let [width, height] = openpaint_practice::Layout::of_sheet(&record).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let view = SheetView::of(&record, &packs, ([width, height], &Title::untitled(), 1));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library.clone()).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let tally = openpaint_practice::Tally::default();
    let mut session = openpaint_practice_app::open_sheet(
        sheet,
        (brush, pens),
        (record, Vec::new(), view, tally),
        &VIEWPORT,
    );
    practise(&mut session, PracticeIntent::Instructions(false));
    session
}

/// A pen stroke along `points` (frame units of `frame`).
fn draw(session: &mut Session, frame: &Frame, points: &[Point]) {
    let path: Vec<[f32; 2]> = (resample(points, 0.002).into_iter())
        .map(|p| frame.to_canvas(p).map(|v| v as f32))
        .collect();
    let sample = |index: usize| InputSample {
        position: path[index],
        pressure: 0.7,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 4 * index as i64,
    };
    let last = path.len() - 1;
    for (phase, samples) in [
        (ContactPhase::Down, vec![sample(0)]),
        (ContactPhase::Move, (1..last).map(sample).collect()),
        (ContactPhase::Up, vec![sample(last)]),
    ] {
        let contact = Contact {
            id: 7,
            device: Device::Pen,
            phase,
            samples,
        };
        session
            .dispatch(Intent::Stroke(StrokeIntent::Contact(contact)))
            .expect("dispatch");
    }
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, NOW).expect("practised");
}

fn copy_frame(session: &Session) -> Frame {
    let practice = PracticeProjection::of(session).expect("a sheet");
    practice.sheet.pairs[0].copy
}

/// A division drawn on `session`'s sheet: two main lines (one in two pieces through the picture's
/// pixel (400, 330) of 800 by 600, returned with its frame), a part's frame in three strokes with
/// Second and a short line with Third.
fn divided(session: &mut Session) -> (Frame, Point) {
    let frame = copy_frame(session);
    let practice = PracticeProjection::of(session).expect("a sheet");
    let picture = practice.sheet.pictures[0].clone().expect("its picture");
    let known = picture.fit.to_frame([400.0, 330.0]);
    draw(session, &frame, &[[0.08, known[1]], known]);
    draw(session, &frame, &[known, [0.92, known[1] + 0.01]]);
    draw(session, &frame, &[[0.08, 0.3], [0.92, 0.28]]);
    practise(session, PracticeIntent::Pen(Pen::Second));
    let corners = [[0.3, 0.6], [0.3, 0.4], [0.45, 0.4], [0.45, 0.6]];
    for side in corners.windows(2) {
        draw(session, &frame, side);
    }
    practise(session, PracticeIntent::Pen(Pen::Third));
    draw(session, &frame, &[[0.6, 0.6], [0.66, 0.5]]);
    (frame, known)
}

/// The division's overlay as compared: the picture's corners, how strongly it shows, the check.
fn division_overlay(session: &Session) -> ([[f32; 2]; 4], f32, DivisionCheck) {
    let practice = PracticeProjection::of(session).expect("a sheet");
    let compared = practice.compared.as_ref().expect("compared");
    let division = compared
        .downcast::<DivisionCompared>()
        .expect("a division's overlay");
    let frame = practice.sheet.pairs[practice.current.expect("shown")].copy;
    let corners = division
        .corners
        .map(|p| frame.to_canvas(p).map(|v| v as f32));
    (corners, practice.shown, division.check.clone())
}

#[test]
fn a_division_is_drawn_by_level_and_compared_with_its_picture_under_it() {
    let mut session = opened(Kind::Division(Tier::Simple));
    let practice = PracticeProjection::of(&session).expect("a sheet");
    assert_eq!(practice.family, Some(Family::Division));
    assert_eq!(practice.pen.map(|(pen, _)| pen), Some(Pen::Main));
    assert!(!practice.aligned_offered);
    for refused in [Pen::Sketch, Pen::Draw, Pen::Correct, Pen::Big] {
        assert!(session.practise(PracticeIntent::Pen(refused), NOW).is_err());
    }
    let (frame, known) = divided(&mut session);
    practise(&mut session, PracticeIntent::Compare(true));
    let (laid, shown, check) = division_overlay(&session);
    // The frames are the picture's shape and the picture lies exactly over the copy's (S99).
    assert_eq!(frame.size, [400.0, 300.0]);
    assert_eq!(practice.sheet.pairs[0].original.size, frame.size);
    assert_eq!(laid, frame.corners().map(|p| p.map(|v| v as f32)));
    assert_eq!(check.rows.map(|row| row.lines), [2, 3, 1]);
    assert!((shown - 0.5).abs() < 1e-6, "at half strength");
    // The slider raises the picture under the same lines.
    practise(&mut session, PracticeIntent::Shown(0.9));
    let (_, raised, again) = division_overlay(&session);
    assert!((raised - 0.9).abs() < 1e-6);
    assert_eq!(again, check);
    // The picture lies in place: its pixel (400, 330) on the canvas pixel the line was drawn at.
    let (u, v) = (400.0 / 800.0, 330.0 / 600.0);
    let laid_at = [
        laid[0][0] + (laid[1][0] - laid[0][0]) * u,
        laid[0][1] + (laid[3][1] - laid[0][1]) * v,
    ];
    let drawn = frame.to_canvas(known);
    assert!((f64::from(laid_at[0]) - drawn[0]).abs() < 0.01);
    assert!((f64::from(laid_at[1]) - drawn[1]).abs() < 0.01);
    let main = check.lines.iter().find(|line| line.level == Level::Main);
    assert!(main.is_some_and(|line| {
        line.points
            .iter()
            .any(|p| (p[0] - known[0]).hypot(p[1] - known[1]) < 0.003)
    }));
}

#[test]
fn lines_drawn_after_a_divisions_compare_are_part_of_it_and_the_first_numbers_kept() {
    let mut session = opened(Kind::Division(Tier::Simple));
    let (frame, _) = divided(&mut session);
    practise(&mut session, PracticeIntent::Compare(true));
    let (_, _, check) = division_overlay(&session);
    // No correcting pen: lines drawn after Compare are the division's.
    let pen = PracticeProjection::of(&session).and_then(|p| p.pen);
    assert_eq!(pen.map(|(pen, _)| pen), Some(Pen::Third));
    let first = check.metrics.clone();
    assert_eq!(value(&first, Metric::DivisionLines(Level::Main)), Some(2.0));
    practise(&mut session, PracticeIntent::Pen(Pen::Main));
    // In the frame, the picture's shape: 1 wide, 0.75 tall (S99).
    assert_eq!(frame.extent(), [1.0, 0.75]);
    for k in 0..4 {
        let y = 0.55 + 0.04 * f64::from(k);
        draw(&mut session, &frame, &[[0.1, y], [0.5, y]]);
    }
    let gone = (PracticeProjection::of(&session)).is_some_and(|p| !p.comparing);
    assert!(gone, "the comparison goes as the pen lands");
    practise(&mut session, PracticeIntent::Compare(true));
    let (_, _, check) = division_overlay(&session);
    assert_eq!(check.rows[0].lines, 6);
    assert!(check.too_many_main());
    let record = session.sheet().save_request().expect("the record changed");
    let result = record.first(0).expect("compared");
    assert_eq!(record.laid(0).map_or(0, |l| l.compares), 2);
    assert_eq!(result.metrics, first, "the first Compare's numbers kept");
}

/// The layer named `name` of `session`'s artwork: its opacity, whether it shows, its strokes and
/// whether it is drawn on.
fn layer(session: &Session, name: &str) -> (f32, bool, usize, bool) {
    let artwork = session.artwork();
    let layer = (artwork.layers().iter())
        .find(|layer| layer.name().as_str() == name)
        .unwrap_or_else(|| panic!("no layer {name}"));
    let props = layer.props();
    let active = artwork.active() == layer.id();
    (
        props.opacity(),
        props.visible(),
        layer.strokes().len(),
        active,
    )
}

/// How strongly a whole copy's picture lies over the copy, compared (no outline).
fn picture_shown(session: &Session) -> f32 {
    let practice = PracticeProjection::of(session).expect("a sheet");
    let compared = practice.compared.as_ref().expect("compared");
    assert!(
        compared.downcast::<PictureCompared>().is_some(),
        "the picture, no outline"
    );
    practice.shown
}

#[test]
fn a_whole_copys_division_dims_under_draw_and_hides_at_compare_never_a_step() {
    let mut session = opened(Kind::WholeCopy(Tier::Simple));
    let frame = copy_frame(&session);
    assert_eq!(
        session.artwork().layers().len(),
        3,
        "the sheet's and two more"
    );
    assert_eq!(layer(&session, DIVISION_LAYER), (1.0, true, 0, true));
    draw(&mut session, &frame, &[[0.08, 0.5], [0.92, 0.45]]);
    practise(&mut session, PracticeIntent::Pen(Pen::Second));
    draw(&mut session, &frame, &[[0.3, 0.7], [0.3, 0.4]]);
    assert_eq!(layer(&session, DIVISION_LAYER).2, 2);
    // Draw: the division dims under the line drawing, which goes on its own layer.
    practise(&mut session, PracticeIntent::Pen(Pen::Draw));
    assert_eq!(
        layer(&session, DIVISION_LAYER),
        (DIVISION_DIMMED, true, 2, false)
    );
    draw(&mut session, &frame, &[[0.1, 0.52], [0.9, 0.47]]);
    draw(&mut session, &frame, &[[0.29, 0.72], [0.31, 0.38]]);
    assert_eq!(layer(&session, LINES_LAYER), (1.0, true, 2, true));
    // Undo takes back strokes only: four strokes, four steps; the division stays dimmed.
    assert_eq!(session.history().undo_steps(), 4);
    session
        .dispatch(Intent::History(openpaint_app::HistoryIntent::Undo))
        .expect("undo");
    assert_eq!(layer(&session, LINES_LAYER).2, 1);
    assert_eq!(layer(&session, DIVISION_LAYER).0, DIVISION_DIMMED);
    session
        .dispatch(Intent::History(openpaint_app::HistoryIntent::Redo))
        .expect("redo");
    // Compare: the picture over the copy at half strength, the division hidden; the slider.
    practise(&mut session, PracticeIntent::Compare(true));
    assert!(!layer(&session, DIVISION_LAYER).1, "hidden");
    assert!((picture_shown(&session) - 0.5).abs() < 1e-6);
    practise(&mut session, PracticeIntent::Shown(0.85));
    assert!((picture_shown(&session) - 0.85).abs() < 1e-6);
    let record = session.sheet().save_request().expect("compared");
    let metrics = &record.first(0).expect("compared").metrics;
    for level in [Level::Main, Level::Second] {
        assert_eq!(value(metrics, Metric::DivisionLines(level)), Some(1.0));
    }
    // A line added: the comparison goes, the division shows again, dimmed under Draw.
    draw(&mut session, &frame, &[[0.5, 0.6], [0.6, 0.6]]);
    assert_eq!(
        layer(&session, DIVISION_LAYER),
        (DIVISION_DIMMED, true, 2, false)
    );
    // Back to a level's pen: the division whole again, drawn on.
    practise(&mut session, PracticeIntent::Pen(Pen::Third));
    assert_eq!(layer(&session, DIVISION_LAYER), (1.0, true, 2, true));
    assert_eq!(session.history().undo_steps(), 5);
}

#[test]
fn stage_four_holds_a_division_and_a_whole_copy_with_the_composition_pack() {
    let bare = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    let (record, _) = bare.begin((4, None), 0x5EED_0095, NOW);
    let families: Vec<Family> = record
        .exercises()
        .map(|l| &l.exercise)
        .map(|e| e.kind.family())
        .collect();
    assert_eq!(families[3], Family::Cylinder, "S94's mix");
    assert!(!bare.today().compositions);
    let log = PracticeLog::new(Vec::new(), Vec::new(), vec![compositions()]);
    assert!(log.today().compositions);
    let (record, _) = log.begin((4, None), 0x5EED_0095, NOW);
    let families: Vec<Family> = record
        .exercises()
        .map(|l| &l.exercise)
        .map(|e| e.kind.family())
        .collect();
    assert_eq!(
        families,
        [
            Family::Object,
            Family::Object,
            Family::Object,
            Family::Division,
            Family::WholeCopy
        ]
    );
    let pictures: Vec<&ItemRef> = (record.exercises().skip(3).map(|l| &l.exercise))
        .filter_map(|e| e.picture.as_ref())
        .collect();
    assert_eq!(pictures.len(), 2);
    assert_ne!(pictures[0], pictures[1]);
    let (drill, _) = log
        .begin_drill(Family::Division, 7, NOW)
        .expect("a division drill");
    assert_eq!(drill.exercises().count(), 3);
    assert!(bare.begin_drill(Family::WholeCopy, 7, NOW).is_none());
    let pack = (log.today().packs.first().cloned()).expect("the pack");
    assert_eq!((pack.pictures, pack.outlines), (2, None));
}
