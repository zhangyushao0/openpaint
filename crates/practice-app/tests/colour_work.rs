//! Week 2's colour work on a sheet (S82): a value scale painted a cell at a time and read against
//! an even scale (the first Compare's numbers kept); and a long colour copy beside the lessons,
//! opened with every tool, compared in place as asked and never ended by Next. Colour
//! combinations of real crops: `crops.rs`.

mod support;

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, StrokeIntent, Toolbox,
    Viewport,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Compared, Exercise, Figure, ItemRef, Kind, Layout, Metric, Pack, PackItem, Pair, Picked,
    PictureCompared, PictureKind, ScaleCompared, ScaleTone, Sheet, Tier, ValueScale, value,
};
use openpaint_practice_app::{
    PictureImage, PracticeIntent, PracticeLog, PracticeNotice, PracticePens, PracticePick,
    SheetSession, Start,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};

fn brush(color: Color) -> StrokeStyle {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, color).expect("style")
}

fn pens() -> PracticePens {
    PracticePens::built_in().expect("pens")
}

/// The sheet of `record` with `packs`, opened.
fn opened(record: Sheet, packs: &[Pack]) -> SheetSession {
    let record = record.framed(packs);
    let [width, height] = Layout::of_sheet(&record).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let opening = support::opening(&record, (packs, Vec::new()), [width, height]);
    let tools = (brush(Color::rgb(0.2, 0.4, 0.6).expect("colour")), pens());
    SheetSession::open(sheet, tools, opening, &VIEWPORT)
}

/// A pen stroke through `points` (canvas pixels).
fn draw(session: &mut SheetSession, points: &[[f32; 2]]) {
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

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, 1_791_400_000).expect("practised");
}

fn colour([r, g, b]: [u8; 3]) -> Color {
    let c = |v: u8| f32::from(v) / 255.0;
    Color::rgb(c(r), c(g), c(b)).expect("colour")
}

fn colour_work() -> SheetSession {
    let exercises = vec![
        Exercise::generated(Kind::Scale(ScaleTone::Hue, 5), 0x5EED_0082),
        Exercise::generated(Kind::Scale(ScaleTone::Grey, 9), 0x5EED_0083),
    ];
    opened(support::laid(exercises, 0), &[])
}

/// The copy's frame as the runtime reads it: `pixels` of the frame's size.
fn read(session: &mut SheetSession, pixels: Vec<u8>) {
    let job = session.sheet_mut().read_request().expect("asked for");
    assert_eq!(job.size, [400, 400]);
    let image = PictureImage {
        size: job.size,
        pixels,
    };
    session.sheet_mut().read(job.sequence, job.run(&image));
}

/// The comparison shown, as its kind's own.
fn compared<T: Compared + Clone>(session: &SheetSession) -> Option<T> {
    let shown = session.projection().compared.clone()?;
    shown.downcast::<T>().cloned()
}

/// How strongly the original shows, the area compared, whether it is outlined over the frames.
fn seen(session: &SheetSession) -> (f32, usize, bool) {
    let p = session.projection();
    let haloed = |m: &openpaint_practice::SheetMark| match &m.drawn {
        Figure::Haloed { lines, red } => !red && !lines.is_empty(),
        _ => false,
    };
    (p.shown, p.area, p.overlay.iter().flatten().any(haloed))
}

/// A copy's frame with every cell of `scale` painted at its even value, the paper round them.
fn even_scale(scale: &ValueScale) -> Vec<u8> {
    let targets = scale.targets();
    let mut pixels = Vec::with_capacity(400 * 400 * 4);
    for y in 0..400 {
        for x in 0..400 {
            let p = [x, y].map(|v| (f64::from(v) + 0.5) / 400.0);
            let [r, g, b] = (scale.cell_at(p)).map_or([255; 3], |cell| scale.colour(targets[cell]));
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    pixels
}

#[test]
fn a_value_scale_is_painted_a_cell_at_a_time_and_read_against_an_even_scale() {
    let mut session = colour_work();
    practise(&mut session, PracticeIntent::Next(VIEWPORT));
    let practice = session.projection();
    let grey = Some(Kind::Scale(ScaleTone::Grey, 9));
    assert_eq!(
        (practice.kind, practice.cell),
        (grey, Some(1)),
        "its first cell"
    );
    let pair = practice.sheet.pairs[1];
    let Some(&scale) = practice.original.as_deref().expect("in hand").scale() else {
        panic!("a value scale")
    };
    let middle = |cell: usize| {
        let [[l, t], [r, b]] = scale.cell(cell);
        let at = pair.copy.to_canvas([(l + r) / 2.0, (t + b) / 2.0]);
        at.map(|v| v as f32)
    };
    let cell = |session: &SheetSession| session.projection().cell;
    // The given ends are picked as given; the pen never paints them.
    let end = PracticePick::Colour(colour(scale.ends()[0]));
    assert_eq!(session.sheet_mut().pick(middle(0)), end);
    draw(
        &mut session,
        &[middle(0), [middle(0)[0], middle(0)[1] + 5.0]],
    );
    let notice = session.projection().notice;
    assert_eq!(notice, Some(PracticeNotice::DrawInCell));
    // Fill takes the next cell in hand; the pen landing in a cell takes that one.
    let first = scale.colour(scale.targets()[1]);
    practise(&mut session, PracticeIntent::Fill(colour(first)));
    assert_eq!(cell(&session), Some(2));
    draw(
        &mut session,
        &[middle(3), [middle(3)[0] + 4.0, middle(3)[1]]],
    );
    assert_eq!(cell(&session), Some(3));
    practise(&mut session, PracticeIntent::Compare(true));
    read(&mut session, even_scale(&scale));
    let Some(ScaleCompared {
        check: Some(check), ..
    }) = compared(&session)
    else {
        panic!("the scale read")
    };
    assert!(check.uneven.is_empty() && check.reversed.is_empty());
    let record = session.sheet_mut().save_request().expect("changed");
    let metrics = &record.first(1).expect("compared").metrics;
    assert_eq!(value(metrics, Metric::ScaleReversed), Some(0.0));
}

fn pack() -> Pack {
    Pack {
        id: "test-shapes".to_owned(),
        title: "Test shapes".to_owned(),
        items: vec![PackItem {
            id: "teapot".to_owned(),
            kind: PictureKind::Shape,
            tier: Tier::Simple,
            size: [800, 600],
            title: "Teapot".to_owned(),
            credit: None,
            outline: None,
            masses: None,
            areas: None,
        }],
    }
}

/// The pack's one picture.
fn teapot() -> ItemRef {
    ItemRef {
        pack: "test-shapes".to_owned(),
        item: "teapot".to_owned(),
    }
}

/// A colour copy of the pack's one picture begun (chosen) and opened, its picture read: the log,
/// the copy's record, the session and its frames.
fn copy_opened() -> (PracticeLog, Sheet, SheetSession, Pair) {
    let log = PracticeLog::new(Vec::new(), Vec::new(), vec![pack()]);
    let elsewhere = ItemRef {
        pack: "personal".to_owned(),
        ..teapot()
    };
    let none = log.begin_copy(&elsewhere, 1, 2);
    assert!(none.is_none(), "a picture no pack holds");
    let (mut record, title) = log.begin_copy(&teapot(), 1, 2).expect("a picture");
    assert_eq!(title.as_str(), "Colour copy 1");
    assert!(record.is_copy());
    record.canvas = 40;
    let mut session = opened(record.clone(), &[pack()]);
    let practice = session.projection();
    // The full interface only, painted with the toolbox's own brushes: no pen (S104).
    assert!(practice.all_tools, "the full interface");
    assert_eq!(practice.pen, None);
    assert_eq!(practice.how, Some(Kind::Study));
    assert!(practice.lesson.is_none() && practice.sheet.rows.is_empty());
    let pair = practice.sheet.pairs[0];
    assert_eq!(pair.copy.size, [1600.0, 1200.0]);
    let item = session
        .sheet_mut()
        .picture_request()
        .expect("its picture asked for");
    let image = PictureImage {
        size: [80, 60],
        pixels: [200, 120, 40, 255].repeat(80 * 60),
    };
    session.sheet_mut().picture_read(item, Some(image));
    (log, record, session, pair)
}

#[test]
fn a_long_colour_copy_opens_with_every_tool_compares_in_place_and_never_ends_by_next() {
    let (_, _, mut session, pair) = copy_opened();
    let on_original = pair.original.to_canvas([0.5, 0.375]).map(|v| v as f32);
    let before = session.sheet_mut().pick(on_original);
    assert_eq!(before, PracticePick::Refused, "not before a check");
    let at = pair.copy.to_canvas([0.5, 0.375]).map(|v| v as f32);
    draw(&mut session, &[at, [at[0] + 80.0, at[1]]]);
    practise(&mut session, PracticeIntent::Compare(true));
    let Some(PictureCompared { corners, .. }) = compared(&session) else {
        panic!("the picture over the copy")
    };
    assert!((seen(&session).0 - 0.5).abs() < 1e-6);
    let corner = pair.copy.to_canvas([1.0, 0.75]).map(|v| v as f32);
    assert_eq!(pair.copy.to_canvas(corners[2]).map(|v| v as f32), corner);
    let unread = session.sheet_mut().read_request().is_none();
    assert!(unread, "nothing to read");
    practise(&mut session, PracticeIntent::Shown(1.0));
    let picked = session.sheet_mut().pick(on_original);
    let PracticePick::Probe {
        copy,
        on_original: true,
        ..
    } = picked
    else {
        panic!("a probe")
    };
    assert_eq!(copy, at);
    session.sheet_mut().probed([190, 130, 50]);
    let probe = session.projection().probe;
    let probe = probe.expect("compared");
    assert_eq!(
        (probe.original, probe.copy),
        ([200, 120, 40], [190, 130, 50])
    );
    let next = session.practise(PracticeIntent::Next(VIEWPORT), 3);
    assert!(next.is_err(), "never ended by Next");
    let saved = session.sheet_mut().save_request().expect("changed");
    let first = saved.first(0).map(|first| first.metrics.len());
    let compares = saved.laid(0).map(|l| l.compares);
    assert_eq!((compares, first), (Some(1), Some(0)));
}

#[test]
fn a_long_colour_copy_is_kept_beside_the_lessons_and_counts_in_none() {
    let (mut log, record, _, _) = copy_opened();
    log.keep(record);
    let path = log.path();
    assert!(path.first && log.learner().fresh(), "no lesson");
    let copy = &path.copies[..];
    assert_eq!((copy.len(), copy[0].name.as_str()), (1, "Colour copy 1"));
    assert_eq!(copy[0].picture, Some(teapot()));
    let shown = path.picture(&teapot()).expect("the picture offered");
    assert_eq!((shown.copies, shown.lessons), (1, 0));
    let (next, _) = log.start(Start::Next, 9, 3).expect("the first lesson");
    assert_eq!(next.lessons[0].picked, Picked::First);
    let (_, title) = log.begin_copy(&teapot(), 4, 5).expect("a picture again");
    assert_eq!(title.as_str(), "Colour copy 2");
}
