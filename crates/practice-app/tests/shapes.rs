//! Single shapes on a practice sheet (S99): a picture that is not square in frames of its shape,
//! the copy's the same shape and size; its ground faded outside its outline as its original's
//! frame shows it, the object whole, the picture itself kept for Compare; Compare laying the
//! picture exactly over the copy's frame with its outline in red; a picture without an outline
//! shown whole.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Session, StrokeIntent, Title,
    Toolbox, Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Exercise, Figure, ItemRef, Kind, Outline, Pack, PackItem, PictureKind, Plan, SHAPE_SHOWN,
    ShapeCompared, Sheet, Tier,
};
use openpaint_practice_app::{
    GROUND_SHOWN, PictureImage, PracticeIntent, PracticePens, PracticeProjection, SheetView,
};
use support::OnSheet;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
const NOW: u64 = 1_791_950_000;

/// A pack of two upright pictures, 300 by 400: `tall` outlined by a rectangle round its middle,
/// `plain` without an outline.
fn pack() -> Pack {
    let item = |id: &str, outline: Option<Outline>| PackItem {
        id: id.to_owned(),
        kind: PictureKind::Shape,
        tier: Tier::Entry,
        size: [300, 400],
        title: "Test".to_owned(),
        credit: None,
        outline,
        masses: None,
        areas: None,
    };
    let rectangle = vec![[75.0, 100.0], [225.0, 100.0], [225.0, 300.0], [75.0, 300.0]];
    Pack {
        id: "test-frames".to_owned(),
        title: "Test frames".to_owned(),
        items: vec![
            item(
                "tall",
                Some(Outline {
                    contours: vec![rectangle],
                }),
            ),
            item("plain", None),
        ],
    }
}

fn opened(picture: &str) -> Session {
    let packs = [pack()];
    let exercise = Exercise {
        picture: Some(ItemRef {
            pack: "test-frames".to_owned(),
            item: picture.to_owned(),
        }),
        ..Exercise::generated(Kind::Shape(Tier::Entry), 0x5EED_0099)
    };
    let plan = Plan {
        exercises: vec![exercise],
        warm_up: 0,
    };
    let record = Sheet::session(1, 2, 0, plan);
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

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, NOW).expect("practised");
}

fn practice(session: &Session) -> PracticeProjection {
    PracticeProjection::of(session).expect("a sheet")
}

/// A pen stroke through `points` (canvas pixels).
fn draw(session: &mut Session, points: &[[f32; 2]]) {
    let sample = |index: usize| InputSample {
        position: points[index],
        pressure: 0.8,
        tilt: 0.2,
        azimuth: 0.0,
        time_ms: 1000 + 8 * index as i64,
    };
    let last = points.len() - 1;
    let steps = [
        (ContactPhase::Down, vec![sample(0)]),
        (ContactPhase::Move, (1..last).map(sample).collect()),
        (ContactPhase::Up, vec![sample(last)]),
    ];
    for (phase, samples) in steps {
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

/// The picture read for the screen at half its size, one colour all over.
fn read_picture(session: &mut Session) -> PictureImage {
    let item = session
        .sheet()
        .picture_request()
        .expect("its picture asked for");
    let image = PictureImage {
        size: [150, 200],
        pixels: [90, 120, 60, 255].repeat(150 * 200),
    };
    session.sheet().picture_read(item, Some(image.clone()));
    image
}

#[test]
fn a_tall_shape_s_frames_are_its_picture_and_its_ground_fades_outside_its_outline() {
    let mut session = opened("tall");
    let pair = practice(&session).sheet.pairs[0];
    assert_eq!(pair.original.size, [300.0, 400.0]);
    assert_eq!(
        pair.copy.size, pair.original.size,
        "the copy's frame the same"
    );
    assert!(practice(&session).aligned_offered);
    let image = read_picture(&mut session);
    let shown = practice(&session);
    assert_eq!(
        shown.images[0].1.as_deref(),
        Some(&image),
        "kept for Compare"
    );
    let (_, grounded) = shown.grounded.first().expect("its ground faded");
    let alpha = |x: usize, y: usize| grounded.pixels[4 * (y * 150 + x) + 3];
    // The outline's rectangle at half size: 37.5 to 112.5 across, 50 to 150 down.
    assert_eq!(alpha(75, 100), 255, "the object whole");
    assert_eq!(alpha(60, 60), 255);
    let faded = (255.0 * GROUND_SHOWN).round() as u8;
    for (x, y) in [(2, 2), (147, 197), (75, 20), (20, 100), (130, 160)] {
        assert_eq!(alpha(x, y), faded, "the ground at {x}, {y}");
    }
    let edge = alpha(37, 100);
    assert!(faded < edge && edge < 255, "the edge between: {edge}");
    assert_eq!(grounded.pixels[..3], [90, 120, 60], "only its strength");
}

#[test]
fn compare_lays_the_picture_exactly_over_the_copy_s_frame() {
    let mut session = opened("tall");
    read_picture(&mut session);
    let frame = practice(&session).sheet.pairs[0].copy;
    // The rectangle drawn round the object (frame units: a picture pixel is 1/400).
    let corners = [
        [75.0, 100.0],
        [225.0, 100.0],
        [225.0, 300.0],
        [75.0, 300.0],
        [75.0, 100.0],
    ];
    let points: Vec<[f32; 2]> = (corners.iter())
        .map(|[x, y]| frame.to_canvas([x / 400.0, y / 400.0]).map(|v| v as f32))
        .collect();
    draw(&mut session, &points);
    practise(&mut session, PracticeIntent::Compare(true));
    let shown = practice(&session);
    let compared = shown.compared.as_ref().expect("compared");
    let shape = compared
        .downcast::<ShapeCompared>()
        .expect("a picture's overlay");
    let canvas = |p: [f64; 2]| frame.to_canvas(p).map(|v| v as f32);
    assert_eq!(
        shape.corners.map(canvas),
        frame.corners().map(|p| p.map(|v| v as f32))
    );
    let laid = (shown.overlay.iter().flatten()).find_map(|mark| match &mark.drawn {
        Figure::Picture { shown, .. } => Some(*shown),
        _ => None,
    });
    assert!(laid.is_some_and(|laid| (laid - SHAPE_SHOWN).abs() < 1e-6));
    let first = (shape.outline.iter().flatten())
        .next()
        .and_then(|line| line.first());
    assert_eq!(
        first.map(|p| canvas(*p)),
        Some(points[0]),
        "the outline where the picture's is"
    );
}

#[test]
fn a_picture_without_an_outline_shows_whole() {
    let mut session = opened("plain");
    assert!(!practice(&session).aligned_offered);
    read_picture(&mut session);
    assert!(practice(&session).grounded.is_empty());
}
