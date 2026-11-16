//! Key shifts on a practice sheet (S107): a crop beside an empty frame of its shape and its
//! tendency; its original never picked, the copy's own paint picked; Compare reading the copy on
//! the worker against the crop's areas and the tendency (what is off, the marked areas' outlines,
//! both frames in grey), its numbers kept, the learner's eye judging it (S145); the Grey view only
//! while compared, the copy read once its crop is (S158k); a lesson of key shifts started once a
//! pack holds crops.

mod support;

use std::sync::Arc;

use openpaint_app::{BrushLibrary, Toolbox, Viewport};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Areas, Exercise, Figure, ItemRef, Judge, KeyCompared, KeyReading, Kind, Lean, Metric, Outcome,
    Pack, PackItem, PictureKind, SheetPicture, Skill, SkillLevel, Tendency, Tier, Verdict, value,
};
use openpaint_practice_app::{
    Judged, PictureImage, PracticeIntent, PracticeLog, PracticePens, PracticePick, SheetSession,
    Start,
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

fn opened(packs: &[Pack]) -> SheetSession {
    let exercise = Exercise {
        picture: Some(ItemRef {
            pack: "test-crops".to_owned(),
            item: "rust".to_owned(),
        }),
        ..Exercise::generated(Kind::KeyShift(BLUE), 7)
    };
    let record = support::laid(vec![exercise], 0).framed(packs);
    let [width, height] = openpaint_practice::Layout::of_sheet(&record).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let opening = support::opening(&record, (packs, Vec::new()), [width, height]);
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let style = Toolbox::new(library).style().expect("style");
    let brush = StrokeStyle::new(Arc::clone(style.recipe()), 8.0, 1.0, Color::BLACK);
    let tools = (
        brush.expect("style"),
        PracticePens::built_in().expect("pens"),
    );
    SheetSession::open(sheet, tools, opening, &VIEWPORT)
}

fn practise(session: &mut SheetSession, intent: PracticeIntent) {
    session.practise(intent, 1_791_400_000).expect("practised");
}

/// The sheet opened with the crops, its picture read.
fn read_sheet() -> SheetSession {
    let mut session = opened(&[crops()]);
    let item = session.sheet_mut().picture_request().expect("its picture");
    let image = PictureImage {
        size: SIZE,
        pixels: bands(SIZE, BANDS),
    };
    session.sheet_mut().picture_read(item, Some(image));
    session
}

fn overlay(session: &SheetSession) -> Option<(KeyCompared, bool, bool)> {
    let practice = session.projection();
    let compared = practice
        .compared
        .as_ref()?
        .downcast::<KeyCompared>()?
        .clone();
    Some((compared, practice.grey, practice.read_pictures.is_some()))
}

#[test]
fn a_key_shift_shows_its_tendency_and_its_original_is_never_picked() {
    let mut session = read_sheet();
    let practice = session.projection();
    assert_eq!(practice.kind, Some(Kind::KeyShift(BLUE)));
    let tendency = practice.sheet.givens[0]
        .iter()
        .find_map(|mark| match mark.drawn {
            Figure::Tendency(colour) => Some(colour),
            _ => None,
        });
    assert_eq!(tendency, Some(BLUE.colour()), "printed beside the copy");
    assert!(
        practice.sheet.pictures[0]
            .as_deref()
            .and_then(SheetPicture::areas_in_frame)
            .is_some(),
        "the crop's areas"
    );
    let pair = practice.sheet.pairs[0];
    let on = pair.original.to_canvas([0.1, 0.3]).map(|v| v as f32);
    assert_eq!(session.sheet_mut().pick(on), PracticePick::Refused);
    assert_eq!(
        session.projection().notice,
        None,
        "refused without a word (S146)"
    );
    // The copy's frame picks the canvas, compared or not.
    let mine = pair.copy.to_canvas([0.1, 0.3]).map(|v| v as f32);
    assert_eq!(session.sheet_mut().pick(mine), PracticePick::Canvas);
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    practise(&mut session, PracticeIntent::Compare(true));
    assert_eq!(session.sheet_mut().pick(mine), PracticePick::Canvas);
    assert_eq!(session.sheet_mut().pick(on), PracticePick::Refused);
}

#[test]
fn compare_marks_only_what_is_off_and_shows_grey_while_compared() {
    let mut session = read_sheet();
    assert!(session.practise(PracticeIntent::Grey(true), 1).is_err());
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    practise(&mut session, PracticeIntent::Compare(true));
    let job = session
        .sheet_mut()
        .read_request()
        .expect("the copy asked for");
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
    session.sheet_mut().read(job.sequence, job.run(&pixels));
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
    let result = session.projection().result;
    let metrics = result.expect("stored").metrics;
    assert_eq!(value(&metrics, Metric::KeyLightness), Some(1.0));
    // Marked, the eye's still (S145): its three asked, nothing chosen for it.
    let waiting = Judged {
        verdict: Verdict::Eye,
        result: None,
    };
    assert_eq!(session.projection().judged, Some(waiting));
    // The Grey view, until the comparison goes.
    practise(&mut session, PracticeIntent::Grey(true));
    let Some((_, true, true)) = overlay(&session) else {
        panic!("the Grey view, its frames in grey")
    };
    // The learner's choice, once, kept with the exercise.
    practise(&mut session, PracticeIntent::Judge(Outcome::Redo));
    let kept = Some((Outcome::Redo, Judge::Eye));
    assert_eq!(session.projection().judged.and_then(|j| j.result), kept);
    practise(&mut session, PracticeIntent::Fill(Color::WHITE));
    assert!(overlay(&session).is_none());
}

#[test]
fn compare_before_the_crop_is_read_waits_for_it_so_the_grey_view_greys_the_original() {
    let mut session = opened(&[crops()]);
    let item = session.sheet_mut().picture_request().expect("its picture");
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    practise(&mut session, PracticeIntent::Compare(true));
    assert!(
        session.sheet_mut().read_request().is_none(),
        "waits for its crop"
    );
    let image = PictureImage {
        size: SIZE,
        pixels: bands(SIZE, BANDS),
    };
    session.sheet_mut().picture_read(item, Some(image));
    let job = session
        .sheet_mut()
        .read_request()
        .expect("read with its crop");
    assert!(job.original.is_some());
    let pixels = PictureImage {
        size: job.size,
        pixels: bands(job.size, BANDS),
    };
    session.sheet_mut().read(job.sequence, job.run(&pixels));
    practise(&mut session, PracticeIntent::Grey(true));
    let practice = session.projection();
    let pictures = practice.read_pictures.as_ref().expect("read");
    assert_eq!(pictures.len(), 2, "the copy and the crop in grey");
    let grey = |p: &PictureImage| p.pixels.chunks(4).all(|c| c[0] == c[1] && c[1] == c[2]);
    assert!(pictures.iter().all(grey));
    // A crop that cannot be read is read without it.
    let mut lost = opened(&[crops()]);
    let item = lost.sheet_mut().picture_request().expect("its picture");
    practise(&mut lost, PracticeIntent::Fill(Color::BLACK));
    practise(&mut lost, PracticeIntent::Compare(true));
    lost.sheet_mut().picture_read(item, None);
    let job = lost
        .sheet_mut()
        .read_request()
        .expect("read without its crop");
    assert!(job.original.is_none());
}

#[test]
fn a_lesson_of_key_shifts_starts_once_a_pack_holds_crops() {
    let shifts = Skill::of_key("key-shift").expect("key shifts' skill");
    let start = Start::At(shifts, SkillLevel::First);
    let without = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    assert!(without.start(start, 3, 4).is_none());
    let with = PracticeLog::new(Vec::new(), Vec::new(), vec![crops()]);
    let (lesson, _) = with.start(start, 3, 4).expect("a lesson");
    assert!(lesson.exercises().count() > 0);
    assert!(
        (lesson.exercises().map(|l| &l.exercise))
            .all(|e| matches!(e.kind, Kind::KeyShift(_)) && e.picture.is_some())
    );
}
