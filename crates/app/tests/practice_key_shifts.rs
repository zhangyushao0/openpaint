//! Key shifts on a practice sheet (S107): a crop beside an empty frame of its shape and its
//! tendency; its original never picked, the copy's own paint picked; Compare reading the copy on
//! the worker against the crop's areas and the tendency (what is off, the marked areas' outlines,
//! both frames in grey), its numbers kept; the Grey view only while compared; stage 5 offered
//! once a pack holds crops.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, PictureImage, PracticeIntent, PracticeLog, PracticeNotice, PracticePens,
    PracticePick, Session, SheetView, Title, Toolbox, Viewport, sheet_of,
};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Areas, Exercise, Family, Figure, ItemRef, KeyCompared, KeyReading, Kind, Lean, Metric, Pack,
    PackItem, PictureKind, Plan, SessionRecord, Tendency, Tier, value,
};

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// The crop: 600 by 400 pixels, three bands across: a rust, a grey and a light cream.
const SIZE: [u32; 2] = [600, 400];
const BANDS: [[u8; 3]; 3] = [[150, 80, 50], [110, 112, 116], [230, 225, 205]];

fn bands(size: [u32; 2], colours: [[u8; 3]; 3]) -> Vec<u8> {
    (0..size[0] * size[1])
        .flat_map(|at| {
            let [r, g, b] = colours[(3 * (at % size[0]) / size[0]) as usize];
            [r, g, b, 255]
        })
        .collect()
}

fn crops() -> Pack {
    let found = Areas::found(SIZE, &bands(SIZE, BANDS));
    Pack {
        id: "test-crops".to_owned(),
        title: "Test crops".to_owned(),
        items: vec![PackItem {
            id: "rust".to_owned(),
            kind: PictureKind::Crop,
            tier: Tier::Simple,
            size: SIZE,
            title: "Rust".to_owned(),
            credit: None,
            outline: None,
            masses: None,
            areas: Some(Arc::new(found.areas)),
        }],
    }
}

/// A cool blue tendency, across from the rust.
const BLUE: Tendency = Tendency {
    lean: Lean::CoolPure,
    hue: 250,
};

fn opened(packs: &[Pack]) -> Session {
    let exercise = Exercise {
        picture: Some(ItemRef {
            pack: "test-crops".to_owned(),
            item: "rust".to_owned(),
        }),
        ..Exercise::generated(Kind::KeyShift(BLUE), 7)
    };
    let plan = Plan {
        exercises: vec![exercise],
        warm_up: 0,
    };
    let record = SessionRecord::begun(1, (5, 1), (5, 0), plan);
    let [width, height] = sheet_of(&record, packs).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let view = SheetView::of(&record, packs, ([width, height], &Title::untitled()));
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush = StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK);
    let tools = (
        brush.expect("style"),
        PracticePens::built_in().expect("pens"),
    );
    let opening = (
        record,
        Vec::new(),
        view,
        openpaint_practice::Tally::default(),
    );
    Session::practising(sheet, tools, opening, &VIEWPORT)
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, 1_791_400_000).expect("practised");
}

/// The sheet opened with the crops, its picture read.
fn read_sheet() -> Session {
    let mut session = opened(&[crops()]);
    let item = session.practice_picture_request().expect("its picture");
    let image = PictureImage {
        size: SIZE,
        pixels: bands(SIZE, BANDS),
    };
    session.practice_picture_read(item, Some(image));
    session
}

fn overlay(session: &Session) -> Option<(KeyCompared, bool, bool)> {
    let projection = session.projection();
    let practice = projection.practice()?;
    let compared = practice
        .compared
        .as_ref()?
        .downcast::<KeyCompared>()?
        .clone();
    Some((compared, practice.grey, practice.greys.is_some()))
}

#[test]
fn a_key_shift_shows_its_tendency_and_its_original_is_never_picked() {
    let mut session = read_sheet();
    let projection = session.projection();
    let practice = projection.practice().expect("a sheet");
    assert_eq!(practice.family, Some(Family::KeyShift));
    let tendency = practice.sheet.givens[0]
        .iter()
        .find_map(|mark| match mark.drawn {
            Figure::Tendency(colour) => Some(colour),
            _ => None,
        });
    assert_eq!(tendency, Some(BLUE.colour()), "printed beside the copy");
    assert!(practice.sheet.areas(0).is_some(), "the crop's areas");
    let pair = practice.sheet.pairs[0];
    let on = pair.original.to_canvas([0.1, 0.3]).map(|v| v as f32);
    assert_eq!(session.practice_pick(on), PracticePick::Refused);
    let notice = session.projection().practice().and_then(|p| p.notice);
    assert_eq!(notice, Some(PracticeNotice::OriginalLocked));
    // The copy's frame picks the canvas, compared or not.
    let mine = pair.copy.to_canvas([0.1, 0.3]).map(|v| v as f32);
    assert_eq!(session.practice_pick(mine), PracticePick::Canvas);
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    practise(&mut session, PracticeIntent::Compare(true));
    assert_eq!(session.practice_pick(mine), PracticePick::Canvas);
    assert_eq!(session.practice_pick(on), PracticePick::Refused);
}

#[test]
fn compare_marks_only_what_is_off_and_shows_grey_while_compared() {
    let mut session = read_sheet();
    assert!(session.practise(PracticeIntent::Grey(true), 1).is_err());
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    practise(&mut session, PracticeIntent::Compare(true));
    let job = session.practice_read_request().expect("the copy asked for");
    let Some(KeyReading { tendency, .. }) = job.reading.downcast::<KeyReading>() else {
        panic!("read as a key shift")
    };
    assert_eq!((*tendency, job.original.is_some()), (BLUE, true));
    // The copy: the rust left as it was, the grey made far lighter, the cream as it was.
    let copy = [BANDS[0], [200, 202, 206], BANDS[2]];
    let pixels = PictureImage {
        size: job.size,
        pixels: bands(job.size, copy),
    };
    session.practice_read(job.sequence, job.run(&pixels));
    let Some((
        KeyCompared {
            read: Some(read), ..
        },
        false,
        _,
    )) = overlay(&session)
    else {
        panic!("the copy read")
    };
    let areas = Areas::found(SIZE, &bands(SIZE, BANDS)).areas;
    let find = |red: u8| (0..areas.count()).find(|a| areas.colours[*a][0] == red);
    let (rust, grey) = (find(150).expect("the rust"), find(110).expect("the grey"));
    assert!(read.check.unmoved.contains(&rust), "the rust left behind");
    assert!(read.check.off_lightness.contains(&grey), "the grey lighter");
    assert_eq!(read.check.lighter_darker(), (1, 0));
    assert!(read.outline(grey).is_some() && read.outline(rust).is_some());
    let result = session
        .projection()
        .practice()
        .and_then(|p| p.result.clone());
    let metrics = result.expect("stored").metrics;
    assert_eq!(value(&metrics, Metric::KeyLightness), Some(1.0));
    // The Grey view, until the comparison goes.
    practise(&mut session, PracticeIntent::Grey(true));
    let Some((_, true, true)) = overlay(&session) else {
        panic!("the Grey view, its frames in grey")
    };
    practise(&mut session, PracticeIntent::Fill(Color::WHITE));
    assert!(overlay(&session).is_none());
}

#[test]
fn stage_five_and_its_tile_are_offered_once_a_pack_holds_crops() {
    let without = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    assert!(without.begin_drill(Family::KeyShift, 3, 4).is_none());
    let with = PracticeLog::new(Vec::new(), Vec::new(), vec![crops()]);
    let (drill, _) = with.begin_drill(Family::KeyShift, 3, 4).expect("a drill");
    assert_eq!(drill.exercises.len(), 2);
    assert!(
        (drill.exercises.iter())
            .all(|e| matches!(e.kind, Kind::KeyShift(_)) && e.picture.is_some())
    );
    let (session, _) = with.begin((5, None), 3, 4);
    assert_eq!(session.stage, 5);
    let shifts = (session.exercises.iter()).filter(|e| e.kind.family() == Family::KeyShift);
    assert_eq!(shifts.count(), 3);
    assert!(
        with.today()
            .drills
            .iter()
            .any(|t| t.stage == 5 && t.family == Family::KeyShift)
    );
}
