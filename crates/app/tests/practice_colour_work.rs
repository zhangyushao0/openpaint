//! Week 2's colour work on a sheet (S82): a generated colour combination (1.18.1's records, S101)
//! painted with the practice brushes and filled, compared in place with its copy read from the
//! canvas's pixels (the first Compare's numbers kept), one area at a time, the eyedropper
//! comparing a point; a value scale painted a cell at a time; and a long colour copy beside the
//! sessions, opened with every tool, compared in place as asked and never ended by Next.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Contact, ContactPhase, Device, InputSample, Intent, Pen, PictureImage,
    PracticeIntent, PracticeLog, PracticeNotice, PracticePens, PracticePick, Probe, Session,
    SheetView, StrokeIntent, Title, Toolbox, Viewport, sheet_of,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    AreasCompared, Arrangement, Compared, Exercise, Family, Figure, GeneratedAreas, ItemRef, Kind,
    Metric, Original, Pack, PackItem, Pair, PictureCompared, PictureKind, Plan, ScaleCompared,
    ScaleTone, SessionRecord, Tier, ValueScale, value,
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
fn opened(record: SessionRecord, packs: &[Pack]) -> Session {
    let [width, height] = sheet_of(&record, packs).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let view = SheetView::of(&record, packs, ([width, height], &Title::untitled()));
    let tools = (brush(Color::rgb(0.2, 0.4, 0.6).expect("colour")), pens());
    let opening = (
        record,
        Vec::new(),
        view,
        openpaint_practice::Tally::default(),
    );
    Session::practising(sheet, tools, opening, &VIEWPORT)
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

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, 1_791_400_000).expect("practised");
}

fn colour([r, g, b]: [u8; 3]) -> Color {
    let c = |v: u8| f32::from(v) / 255.0;
    Color::rgb(c(r), c(g), c(b)).expect("colour")
}

fn colour_work() -> Session {
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Areas(Arrangement::Patches), 0x5EED_0082),
            Exercise::generated(Kind::Scale(ScaleTone::Grey, 9), 0x5EED_0083),
        ],
        warm_up: 0,
    };
    opened(SessionRecord::begun(1, (2, 1), (5, 0), plan), &[])
}

/// The copy's frame as the runtime reads it: `pixels` of the frame's size.
fn read(session: &mut Session, pixels: Vec<u8>) {
    let job = session.practice_read_request().expect("the copy asked for");
    assert_eq!(job.size, [400, 400]);
    let image = PictureImage {
        size: job.size,
        pixels,
    };
    session.practice_read(job.sequence, job.run(&image));
}

/// The comparison shown, as its kind's own.
fn compared<T: Compared + Clone>(session: &Session) -> Option<T> {
    let shown = session.projection().practice()?.compared.clone()?;
    shown.downcast::<T>().cloned()
}

/// How strongly the original shows, the area compared, whether it is outlined over the frames.
fn seen(session: &Session) -> (f32, usize, bool) {
    let projection = session.projection();
    let p = projection.practice().expect("a sheet");
    let haloed = |m: &openpaint_practice::SheetMark| match &m.drawn {
        Figure::Haloed { lines, red } => !red && !lines.is_empty(),
        _ => false,
    };
    (p.shown, p.area, p.overlay.iter().flatten().any(haloed))
}

/// The combination's frames and areas, painted (its tone filled, a stroke of Big in the colour
/// picked) and compared.
fn painted_and_compared(session: &mut Session) -> (Pair, GeneratedAreas) {
    let projection = session.projection();
    let practice = projection.practice().expect("a sheet");
    assert_eq!(practice.family, Some(Family::Areas));
    assert_eq!(practice.instructions, Some(Family::Areas));
    assert_eq!(practice.pen, Some((Pen::Big, 64.0)));
    assert!(
        practice.sheet.painted[0].is_some(),
        "its picture drawn once"
    );
    let pair = practice.sheet.pairs[0];
    let Original::Areas(generated) = practice.sheet.originals[0].clone() else {
        panic!("a colour combination")
    };
    let tone = colour(generated.areas.colours[0]);
    practise(session, PracticeIntent::Fill(tone));
    let at = pair.copy.to_canvas([0.5, 0.5]).map(|v| v as f32);
    draw(session, &[at, [at[0] + 60.0, at[1]]]);
    let artwork = session.artwork();
    let stroke = (artwork.layers().iter())
        .flat_map(|l| l.strokes().iter())
        .next()
        .expect("a stroke");
    let picked = Color::rgb(0.2, 0.4, 0.6).expect("colour");
    assert_eq!(
        (stroke.style().size(), stroke.style().color()),
        (64.0, picked)
    );
    let refused = session.practise(PracticeIntent::Pen(Pen::Draw), 1);
    assert!(refused.is_err(), "a painting pen");
    practise(session, PracticeIntent::Compare(true));
    (pair, generated)
}

#[test]
fn a_colour_combination_is_painted_and_compared_its_numbers_kept_from_its_pixels() {
    let mut session = colour_work();
    let (_, areas) = painted_and_compared(&mut session);
    // Until the copy's frame is read, the picture shows without numbers.
    let Some(AreasCompared { read: None, .. }) = compared(&session) else {
        panic!("the picture over the copy, not read yet")
    };
    assert_eq!(seen(&session).0, 0.5);
    read(&mut session, areas.picture(400));
    let read = compared::<AreasCompared>(&session).and_then(|a| a.read);
    let read = read.expect("read");
    let (_, chosen, outlined) = seen(&session);
    assert_eq!(chosen, 0);
    let all: Vec<_> = areas.areas.colours.iter().copied().map(Some).collect();
    assert_eq!(read.copy.colours, all);
    assert!(read.strong.is_empty() && outlined);
    let record = session.practice_save_request().expect("the record changed");
    let metrics = &record.result(0).expect("compared").metrics;
    assert_eq!(value(metrics, Metric::AreasDelta), Some(0.0));
    assert_eq!(value(metrics, Metric::AreasMadeStrong), Some(0.0));
}

#[test]
fn a_combination_s_card_shows_any_area_and_the_eyedropper_compares_a_point() {
    let mut session = colour_work();
    let (pair, areas) = painted_and_compared(&mut session);
    read(&mut session, areas.picture(400));
    practise(&mut session, PracticeIntent::Area(2));
    practise(&mut session, PracticeIntent::Shown(0.8));
    let (shown, chosen, _) = seen(&session);
    assert_eq!(chosen, 2, "area 2");
    assert!((shown - 0.8).abs() < 1e-6);
    assert!(session.practise(PracticeIntent::Area(9), 1).is_err());
    let point = pair.copy.to_canvas([0.3, 0.6]).map(|v| v as f32);
    let picked = session.practice_pick(point);
    let PracticePick::Probe {
        copy,
        on_original: false,
        ..
    } = picked
    else {
        panic!("a probe")
    };
    assert_eq!(copy, point);
    session.practice_probed([1, 2, 3]);
    let probe = session.projection().practice().and_then(|p| p.probe);
    let expected = areas.colour_at([0.3, 0.6], 0.0);
    assert!(matches!(probe, Some(Probe { original, copy: [1, 2, 3], .. }) if original == expected));
    // Another area chosen: the card shows it, not the point.
    practise(&mut session, PracticeIntent::Area(1));
    assert_eq!(session.projection().practice().and_then(|p| p.probe), None);
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
    let projection = session.projection();
    let practice = projection.practice().expect("a sheet");
    assert_eq!(practice.family, Some(Family::Scale));
    assert_eq!(practice.cell, Some(1), "the first cell between the ends");
    let pair = practice.sheet.pairs[1];
    let Original::Scale(scale) = practice.sheet.originals[1] else {
        panic!("a value scale")
    };
    let middle = |cell: usize| {
        let [[l, t], [r, b]] = scale.cell(cell);
        let at = pair.copy.to_canvas([(l + r) / 2.0, (t + b) / 2.0]);
        at.map(|v| v as f32)
    };
    let cell = |session: &Session| session.projection().practice().and_then(|p| p.cell);
    // The given ends are picked as given; the pen never paints them.
    let end = PracticePick::Colour(colour(scale.ends()[0]));
    assert_eq!(session.practice_pick(middle(0)), end);
    draw(
        &mut session,
        &[middle(0), [middle(0)[0], middle(0)[1] + 5.0]],
    );
    let notice = session.projection().practice().and_then(|p| p.notice);
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
    let record = session.practice_save_request().expect("the record changed");
    let metrics = &record.result(1).expect("compared").metrics;
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
fn copy_opened() -> (PracticeLog, SessionRecord, Session, Pair) {
    let log = PracticeLog::new(Vec::new(), Vec::new(), vec![pack()]);
    let elsewhere = ItemRef {
        pack: "personal".to_owned(),
        ..teapot()
    };
    assert!(
        log.begin_copy(&elsewhere, 1, 2).is_none(),
        "a picture no pack holds"
    );
    let (mut record, title) = log.begin_copy(&teapot(), 1, 2).expect("a picture");
    assert_eq!(title.as_str(), "Colour copy 1");
    assert!(record.is_project());
    record.canvas = 40;
    let mut session = opened(record.clone(), &[pack()]);
    let projection = session.projection();
    let practice = projection.practice().expect("a sheet");
    // The full interface only, painted with the toolbox's own brushes: no pen (S104).
    assert!(practice.all_tools, "the full interface");
    assert_eq!(practice.pen, None);
    assert_eq!(practice.instructions, Some(Family::Study));
    let pair = practice.sheet.pairs[0];
    assert_eq!(pair.copy.size, [1600.0, 1200.0]);
    let item = session
        .practice_picture_request()
        .expect("its picture asked for");
    let image = PictureImage {
        size: [80, 60],
        pixels: [200, 120, 40, 255].repeat(80 * 60),
    };
    session.practice_picture_read(item, Some(image));
    (log, record, session, pair)
}

#[test]
fn a_long_colour_copy_opens_with_every_tool_compares_in_place_and_never_ends_by_next() {
    let (_, _, mut session, pair) = copy_opened();
    let on_original = pair.original.to_canvas([0.5, 0.375]).map(|v| v as f32);
    let before = session.practice_pick(on_original);
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
    assert!(session.practice_read_request().is_none(), "nothing to read");
    practise(&mut session, PracticeIntent::Shown(1.0));
    let picked = session.practice_pick(on_original);
    let PracticePick::Probe {
        copy,
        on_original: true,
        ..
    } = picked
    else {
        panic!("a probe")
    };
    assert_eq!(copy, at);
    session.practice_probed([190, 130, 50]);
    let probe = session.projection().practice().and_then(|p| p.probe);
    let probe = probe.expect("compared");
    assert_eq!(
        (probe.original, probe.copy),
        ([200, 120, 40], [190, 130, 50])
    );
    let next = session.practise(PracticeIntent::Next(VIEWPORT), 3);
    assert!(next.is_err(), "never ended by Next");
    let saved = session.practice_save_request().expect("the record changed");
    let result = saved.result(0).map(|r| (r.compares, r.metrics.len()));
    assert_eq!(result, Some((1, 0)));
}

#[test]
fn a_long_colour_copy_is_kept_beside_the_sessions_and_counts_in_none() {
    let (mut log, record, _, _) = copy_opened();
    log.keep(record);
    let today = log.today();
    assert_eq!((today.stage, today.done, today.open), (1, 0, None));
    assert_eq!(today.copies.len(), 1);
    assert_eq!(today.copies[0].name.as_str(), "Colour copy 1");
    assert_eq!(today.copies[0].picture, Some(teapot()));
    let shown = today.picture(&teapot()).expect("the picture offered");
    assert_eq!((shown.copies, shown.sessions), (1, 0));
    let (next, _) = log.begin((1, None), 9, 3);
    assert_eq!((next.number, next.warm_up), (1, 0), "the first session");
    let (second, title) = log.begin_copy(&teapot(), 4, 5).expect("a picture again");
    assert_eq!((second.number, title.as_str()), (2, "Colour copy 2"));
}
