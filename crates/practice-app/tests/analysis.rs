//! Abstract analysis on a practice sheet (S112): its picture laid on a locked layer at the bottom
//! of the sheet once read, never an undo step, never picked, a second never clearing the first, a
//! sheet opened again finding them laid; a pen waiting for it; the card's choices taken before
//! the first Compare and kept with it, fixed after; Compare reading the picture's pixels into its
//! views, one shown at a time under the marks.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    AnalysisCompared, Choices, Exercise, Figure, Frame, ItemRef, Kind, Metric, Pack, PackItem,
    PictureKind, Point, Source, Tier, resample, value,
};
use openpaint_practice_app::{
    PICTURE_LAYER, Pen, PictureImage, PracticeIntent, PracticeNotice, PracticePens, PracticePick,
    SheetSession,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
const NOW: u64 = 1_792_100_000;
const SIZE: [u32; 2] = [800, 600];

fn compositions() -> Pack {
    let item = |id: &str| PackItem {
        id: id.to_owned(),
        kind: PictureKind::Composition,
        tier: Tier::Simple,
        size: SIZE,
        title: "Test".to_owned(),
        credit: None,
        outline: None,
        masses: None,
        areas: None,
    };
    Pack {
        id: "test-compositions".to_owned(),
        title: "Test compositions".to_owned(),
        items: vec![item("harbour"), item("hills")],
    }
}

fn analysis(item: &str) -> Exercise {
    Exercise {
        picture: Some(ItemRef {
            pack: "test-compositions".to_owned(),
            item: item.to_owned(),
        }),
        ..Exercise::generated(Kind::Analysis(Tier::Simple), 0x5EED_0112)
    }
}

/// A dark block on a light ground.
fn picture() -> PictureImage {
    let pixels = (0..SIZE[0] * SIZE[1])
        .flat_map(|at| {
            let (x, y) = (at % SIZE[0], at / SIZE[0]);
            let grey = if (300..500).contains(&x) && y > 250 {
                40
            } else {
                210
            };
            [grey, grey, grey, 255]
        })
        .collect();
    PictureImage { size: SIZE, pixels }
}

fn opened(artwork: Option<Artwork>) -> SheetSession {
    let packs = [compositions()];
    let record = support::laid(vec![analysis("harbour"), analysis("hills")], 0).framed(&packs);
    let [width, height] = openpaint_practice::Layout::of_sheet(&record).size;
    let sheet =
        artwork.unwrap_or_else(|| Artwork::new(CanvasSize::new(width, height).expect("size")));
    let opening = support::opening(&record, (&packs, Vec::new()), [width, height]);
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let mut session = SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT);
    session
        .practise(PracticeIntent::Instructions(false), NOW)
        .expect("How closed");
    session
}

/// Every picture the sheet asks for, read.
fn read_pictures(session: &mut SheetSession) {
    while let Some(item) = session.sheet_mut().picture_request() {
        session.picture_read(item, Some(picture()));
    }
}

/// A pen stroke along `points` (frame units of `frame`).
fn draw(session: &mut SheetSession, frame: &Frame, points: &[Point]) {
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

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, NOW).expect("practised");
}

/// The picture layer: its place among the layers (0 the bottom), locked, its edits, active.
fn picture_layer(session: &SheetSession) -> Option<(usize, bool, usize, bool)> {
    let artwork = session.session().artwork();
    let (at, layer) = (artwork.layers().iter().enumerate())
        .find(|(_, layer)| layer.name().as_str() == PICTURE_LAYER)?;
    let active = artwork.active() == layer.id();
    Some((at, layer.props().locked(), layer.edits().len(), active))
}

/// All of the stroke ring's strokes on the sheet.
fn strokes(session: &SheetSession) -> usize {
    let artwork = session.session().artwork();
    artwork
        .layers()
        .iter()
        .map(|layer| layer.strokes().len())
        .sum()
}

fn ring(session: &mut SheetSession, frame: &Frame) {
    let ring = [[0.4, 0.5], [0.6, 0.5], [0.6, 0.7], [0.4, 0.7], [0.4, 0.5]];
    draw(session, frame, &ring);
}

#[test]
fn the_picture_lies_locked_under_the_marks_once_read_never_an_undo_step() {
    let mut session = opened(None);
    let frame = session.projection().sheet.pairs[0].copy;
    assert_eq!(
        session.projection().pen.map(|(pen, _)| pen),
        Some(Pen::Main)
    );
    // Before its picture is read the pen waits.
    ring(&mut session, &frame);
    assert_eq!((picture_layer(&session), strokes(&session)), (None, 0));
    read_pictures(&mut session);
    assert_eq!(picture_layer(&session), Some((0, true, 1, false)));
    assert!(
        !session.session().history().can_undo(),
        "never an undo step"
    );
    ring(&mut session, &frame);
    assert_eq!(strokes(&session), 1, "the ring on the sheet's own layer");
    let base = &session.session().artwork().layers()[1];
    assert_eq!(base.strokes().len(), 1);
    // The eyedropper never reads the picture under the marks.
    let at = frame.to_canvas([0.5, 0.5]).map(|v| v as f32);
    assert_eq!(session.sheet_mut().pick(at), PracticePick::Refused);
    assert_eq!(
        session.projection().notice,
        Some(PracticeNotice::OriginalLocked)
    );
    // Undo takes the ring back, never the picture.
    session
        .dispatch(Intent::History(openpaint_app::HistoryIntent::Undo))
        .expect("undo");
    assert_eq!(strokes(&session), 0);
    assert_eq!(picture_layer(&session), Some((0, true, 1, false)));
}

/// The Compare asked read as the worker reads it: the picture into its five views.
fn read_on_worker(session: &mut SheetSession) {
    let job = session
        .sheet_mut()
        .read_request()
        .expect("read on the worker");
    assert!(job.original.is_some(), "the picture as read for the screen");
    let copy = PictureImage {
        size: job.size,
        pixels: vec![255; job.size[0] as usize * job.size[1] as usize * 4],
    };
    let read = job.run(&copy).expect("read");
    assert_eq!(read.pictures.len(), 5, "the five views");
    session.sheet_mut().read(job.sequence, Some(read));
}

#[test]
fn choices_go_with_the_first_compare_and_the_views_lie_under_the_marks() {
    let mut session = opened(None);
    read_pictures(&mut session);
    let frame = session.projection().sheet.pairs[0].copy;
    assert!(!session.projection().task.is_empty(), "the card's steps");
    for intent in [
        PracticeIntent::Toggle(2),
        PracticeIntent::Toggle(3),
        PracticeIntent::Toggle(0),
        PracticeIntent::Toggle(0),
        PracticeIntent::Shape(3),
    ] {
        practise(&mut session, intent);
    }
    assert!(session.practise(PracticeIntent::Toggle(6), NOW).is_err());
    let chosen = Choices {
        chips: 0b1100,
        shape: Some(3),
    };
    assert_eq!(session.projection().choices, chosen);
    ring(&mut session, &frame);
    practise(&mut session, PracticeIntent::Compare(true));
    let first = session.projection().result.expect("kept").metrics;
    assert_eq!(value(&first, Metric::ChosenChips), Some(12.0));
    assert_eq!(value(&first, Metric::ChosenShape), Some(3.0));
    assert!(
        session.practise(PracticeIntent::Toggle(1), NOW).is_err(),
        "fixed"
    );
    assert!(
        session.practise(PracticeIntent::View(1), NOW).is_err(),
        "unread"
    );
    read_on_worker(&mut session);
    practise(&mut session, PracticeIntent::View(4));
    let practice = session.projection();
    assert_eq!(practice.view, 4);
    let overlay = practice.overlay.expect("compared");
    assert!(matches!(
        overlay[0].drawn,
        Figure::Picture {
            source: Source::Read(4),
            ..
        }
    ));
    let compared = practice.compared.expect("compared");
    let analysis = compared.downcast::<AnalysisCompared>().expect("its own");
    assert_eq!(analysis.choices, chosen);
    assert_eq!(
        practice.read_pictures.map(|pictures| pictures.len()),
        Some(5)
    );
}

#[test]
fn a_second_picture_never_clears_the_first_and_a_sheet_opened_again_finds_them() {
    let mut session = opened(None);
    read_pictures(&mut session);
    let frame = session.projection().sheet.pairs[0].copy;
    ring(&mut session, &frame);
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    assert_eq!(picture_layer(&session), Some((0, true, 2, false)));
    let artwork = session.session().artwork().clone();
    let pixels = |artwork: &Artwork| {
        let layer = (artwork.layers().iter())
            .find(|layer| layer.name().as_str() == PICTURE_LAYER)
            .expect("the picture layer");
        layer.edits().to_vec()
    };
    let laid = pixels(&artwork);
    // Opened again on the same artwork: nothing laid twice.
    let mut again = opened(Some(artwork));
    read_pictures(&mut again);
    assert_eq!(picture_layer(&again).map(|layer| layer.2), Some(2));
    assert_eq!(pixels(again.session().artwork()), laid);
}
