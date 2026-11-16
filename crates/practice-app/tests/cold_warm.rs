//! Cold-warm copies in the app (S154): begun of a picture chosen as a colour copy is, titled and
//! numbered among their own, their pair the next; painted in the full interface beside the two
//! swatches, compared by the picture laid at the slider's strength, both frames in grey on asking
//! and a point compared on the disc with the two tendencies; no result asked or kept, and no lesson,
//! dot or count of lessons moved; the options listing them among the colour copies.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, ProjectId, StrokeIntent,
    Title, Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    CardItem, Figure, Given, In, ItemRef, Kind, Layout, Leans, Outcome, Pack, PackItem, Pair,
    PictureKind, STUDY_SIDE, Sheet, ShiftReading, Span, Tier, Verdict,
};
use openpaint_practice_app::{
    PictureImage, PracticeIntent, PracticeLog, PracticePens, PracticePick, SheetSession,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

fn vase() -> ItemRef {
    ItemRef {
        pack: "test-shapes".to_owned(),
        item: "vase".to_owned(),
    }
}

fn packs() -> Vec<Pack> {
    vec![Pack {
        id: "test-shapes".to_owned(),
        title: "Shapes".to_owned(),
        items: vec![PackItem {
            id: "vase".to_owned(),
            kind: PictureKind::Shape,
            tier: Tier::Medium,
            size: [800, 600],
            title: "Vase".to_owned(),
            credit: None,
            outline: None,
            masses: None,
            areas: None,
        }],
    }]
}

fn opened(record: &Sheet) -> SheetSession {
    let size = Layout::of_sheet(record).size;
    let sheet = Artwork::new(CanvasSize::new(size[0], size[1]).expect("size"));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush =
        StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK).expect("brush");
    let pens = PracticePens::built_in().expect("pens");
    let log = PracticeLog::new(Vec::new(), Vec::new(), packs());
    let printed = Given::of_sheet(record, &packs());
    let opening = log.practising(record, &printed, (size, &Title::untitled()));
    let mut session = SheetSession::open(sheet, (brush, pens), opening, &VIEWPORT);
    practise(&mut session, PracticeIntent::Instructions(false));
    session
}

/// A brush stroke across the copy's frame, from `from` to `to` (frame units).
fn paint(session: &mut SheetSession, [from, to]: [[f64; 2]; 2]) {
    let frame = session.projection().sheet.pairs[0].copy;
    let sample = |p: [f64; 2], time_ms: i64| InputSample {
        position: frame.to_canvas(p).map(|v| v as f32),
        pressure: 0.8,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms,
    };
    let middle = [f64::midpoint(from[0], to[0]), f64::midpoint(from[1], to[1])];
    let steps = [
        (ContactPhase::Down, vec![sample(from, 1000)]),
        (ContactPhase::Move, vec![sample(middle, 1008)]),
        (ContactPhase::Up, vec![sample(to, 1016)]),
    ];
    for (phase, samples) in steps {
        let contact = Contact {
            id: 7,
            device: Device::Pen,
            phase,
            samples,
        };
        let stroke = Intent::Stroke(StrokeIntent::Contact(contact));
        session.dispatch(stroke).expect("dispatch");
    }
}

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, NOON + 3700).expect("practised");
}

#[test]
fn a_cold_warm_copy_is_a_project_of_its_own_counting_in_no_lesson() {
    let mut log = PracticeLog::new(Vec::new(), Vec::new(), packs());
    let learner = log.learner().clone();
    let (mut first, title) = log.begin_cold_warm(&vase(), 9, NOON).expect("a picture");
    assert_eq!(title.as_str(), "Cold-warm copy 1");
    assert!(first.is_copy());
    let kind = first.laid(0).map(|laid| laid.exercise.kind);
    assert_eq!(kind, Some(Kind::ColdWarm(Leans::nth(0))));
    first.canvas = 5;
    first.spans = vec![Span {
        from: NOON,
        to: NOON + 3600,
    }];
    log.keep(first);
    log.name(ProjectId::new(5), title);
    assert_eq!(log.learner(), &learner, "no lesson, no dot");
    let path = log.path();
    assert_eq!(path.book.book.lessons, 0);
    let listed = &path.copies[0];
    assert!(!listed.from_life && listed.name.as_str() == "Cold-warm copy 1");
    let (second, title) = log
        .begin_cold_warm(&vase(), 10, NOON + 4000)
        .expect("again");
    assert_eq!(title.as_str(), "Cold-warm copy 2");
    let kind = second.laid(0).map(|laid| laid.exercise.kind);
    assert_eq!(kind, Some(Kind::ColdWarm(Leans::nth(1))), "the next pair");
    let (_, copy) = log
        .begin_copy(&vase(), 11, NOON + 5000)
        .expect("a colour copy");
    assert_eq!(copy.as_str(), "Colour copy 1", "numbered among its own");
}

/// The first cold-warm copy of the vase begun and opened, its picture read: the session and its
/// pair of frames.
fn begun() -> (SheetSession, Pair) {
    let log = PracticeLog::new(Vec::new(), Vec::new(), packs());
    let (record, _) = log.begin_cold_warm(&vase(), 9, NOON).expect("a picture");
    let pair = Layout::of_sheet(&record).pairs[0];
    let mut session = opened(&record);
    let item = session.sheet_mut().picture_request().expect("its picture");
    let picture = PictureImage {
        size: [800, 600],
        pixels: [90, 120, 160, 255].repeat(800 * 600),
    };
    session.sheet_mut().picture_read(item, Some(picture));
    (session, pair)
}

#[test]
fn it_is_painted_in_the_full_interface_its_two_swatches_picked_whole() {
    let (mut session, pair) = begun();
    assert_eq!(
        pair.copy.size,
        [STUDY_SIDE, 1200.0],
        "a colour copy's frames"
    );
    let shown = session.projection();
    assert!(shown.copy && shown.all_tools, "the full interface");
    let swatches = (shown.sheet.givens[0].iter())
        .filter(|mark| mark.on == In::Copy && matches!(mark.drawn, Figure::Beside { .. }))
        .count();
    assert_eq!(swatches, 2, "the lights' and the shadows' beside the frame");
    let lights = openpaint_practice::beside(&pair.copy, [0.02 + 0.035, 0.035]).map(|v| v as f32);
    let [r, g, b] = Leans::nth(0).lights().map(|c| f32::from(c) / 255.0);
    let warm = Color::rgb(r, g, b).expect("a colour");
    assert_eq!(session.sheet_mut().pick(lights), PracticePick::Colour(warm));
}

#[test]
fn it_is_compared_with_a_grey_view_and_a_disc_no_result_kept() {
    let (mut session, pair) = begun();
    paint(&mut session, [[0.2, 0.3], [0.8, 0.4]]);
    practise(&mut session, PracticeIntent::Compare(true));
    let job = (session.sheet_mut().read_request()).expect("read for its greys");
    assert!(job.reading.downcast::<ShiftReading>().is_some());
    let copy = PictureImage {
        size: job.size,
        pixels: [200, 150, 90, 255].repeat((job.size[0] * job.size[1]) as usize),
    };
    let read = job.run(&copy).expect("read");
    assert_eq!(read.pictures.len(), 2, "both frames in grey");
    session.sheet_mut().read(job.sequence, Some(read));
    let shown = session.projection();
    let verdict = shown.judged.map(|judged| judged.verdict);
    assert_eq!(verdict, Some(Verdict::Nobody));
    assert_eq!(shown.card[..2], [CardItem::Grey, CardItem::Slider]);
    let judging = session.practise(PracticeIntent::Judge(Outcome::Clean), NOON + 3700);
    assert!(judging.is_err(), "no result to choose");
    practise(&mut session, PracticeIntent::Grey(true));
    assert!(session.projection().grey);
    practise(&mut session, PracticeIntent::Grey(false));
    // A point compared: on the disc beside the two tendencies, with its lightness bar.
    let point = pair.copy.to_canvas([0.5, 0.375]).map(|v| v as f32);
    let probe = session.sheet_mut().pick(point);
    assert!(matches!(probe, PracticePick::Probe { .. }));
    session.sheet_mut().probed([200, 150, 90]);
    let card = session.projection().card;
    let tendencies = Leans::nth(0).on_disc();
    let leaning = (card.iter())
        .any(|item| matches!(item, CardItem::Leaning { tendencies: on, .. } if *on == tendencies));
    assert!(leaning, "{card:?}");
    let saved = (session.sheet_mut().save_request()).expect("its Compare kept");
    let laid = saved.laid(0).expect("the copy");
    assert!(laid.result.is_none(), "no result");
    let first = laid.first.as_ref().expect("compared");
    assert!(first.metrics.is_empty(), "no number");
}
