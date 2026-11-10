//! Colour combinations of real crops (S101): a crop of a pack beside an empty frame of its shape,
//! filled and compared, its copy read against the areas the pack holds (the outline and the
//! weak contrasts made strong in the frame's units), the eyedropper reading the pack's picture;
//! Compare unavailable once the crop is removed; and the practice options knowing whether a
//! pack kept holds crops, a combinations drill only then.

mod support;

use std::sync::Arc;

use openpaint_app::{BrushLibrary, Session, Title, Toolbox, Viewport};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Areas, AreasCompared, Exercise, Family, Figure, ItemRef, Kind, Pack, PackItem, PictureKind,
    Plan, Sheet, Tier,
};
use openpaint_practice_app::{
    PictureImage, PracticeIntent, PracticeLog, PracticePens, PracticePick, PracticeProjection,
    SheetView,
};
use support::OnSheet;

const VIEWPORT: Viewport = Viewport {
    origin: [0.0, 84.0],
    size: [2880.0, 1716.0],
    pixels_per_point: 1.75,
};
/// The crop: 600 by 400 pixels, three bands across, the first two a weak contrast apart.
const SIZE: [u32; 2] = [600, 400];
const BANDS: [[u8; 3]; 3] = [[120, 80, 60], [128, 86, 64], [150, 150, 145]];

/// The crop's pixels: `bands`' colours a third of its width each.
fn picture(bands: [[u8; 3]; 3]) -> Vec<u8> {
    (0..SIZE[0] * SIZE[1])
        .flat_map(|at| {
            let [r, g, b] = bands[(3 * (at % SIZE[0]) / SIZE[0]) as usize];
            [r, g, b, 255]
        })
        .collect()
}

fn crops() -> Pack {
    let found = Areas::found(SIZE, &picture(BANDS));
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

fn rust() -> ItemRef {
    ItemRef {
        pack: "test-crops".to_owned(),
        item: "rust".to_owned(),
    }
}

/// A sheet of one combination of the crop, opened with `packs`.
fn opened(packs: &[Pack]) -> Session {
    let combination = Exercise {
        picture: Some(rust()),
        ..Exercise::generated(Kind::Crop(Tier::Simple), 7)
    };
    let plan = Plan {
        exercises: vec![combination],
        warm_up: 0,
    };
    let record = Sheet::session(1, 2, 0, plan);
    let record = record.framed(packs);
    let [width, height] = openpaint_practice::Layout::of_sheet(&record).size;
    let sheet = Artwork::new(CanvasSize::new(width, height).expect("size"));
    let view = SheetView::of(&record, packs, ([width, height], &Title::untitled(), 1));
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
    openpaint_practice_app::open_sheet(sheet, tools, opening, &VIEWPORT)
}

fn practise(session: &mut Session, intent: PracticeIntent) {
    session.practise(intent, 1_791_400_000).expect("practised");
}

/// The colour combination as compared, and the outline of the area the card compares as the
/// frames show it.
fn overlay(session: &Session) -> Option<(AreasCompared, Vec<Vec<[f64; 2]>>)> {
    let practice = PracticeProjection::of(session)?;
    let compared = practice
        .compared
        .as_ref()?
        .downcast::<AreasCompared>()?
        .clone();
    let outline = (practice.overlay.iter().flatten())
        .find_map(|mark| match &mark.drawn {
            Figure::Haloed { lines, red: false } => Some(lines.clone()),
            _ => None,
        })
        .unwrap_or_default();
    Some((compared, outline))
}

/// The copy Compare asks for, read as painted: the bands right, but the second pushed far from
/// the first.
fn read_wide_copy(session: &mut Session) {
    let job = session.sheet().read_request().expect("the copy asked for");
    assert_eq!(job.size, [400, 267]);
    let wide = [BANDS[0], [200, 170, 120], BANDS[2]];
    let copy: Vec<u8> = (0..400 * 267)
        .flat_map(|at| {
            let [r, g, b] = wide[(3 * (at % 400) / 400) as usize];
            [r, g, b, 255]
        })
        .collect();
    let pixels = PictureImage {
        size: job.size,
        pixels: copy,
    };
    session.sheet().read(job.sequence, job.run(&pixels));
}

#[test]
fn a_crop_is_copied_beside_it_in_a_frame_of_its_shape_and_read_against_its_areas() {
    let mut session = opened(&[crops()]);
    let practice = PracticeProjection::of(&session).expect("a sheet");
    assert_eq!(practice.family, Some(Family::Areas));
    let pair = practice.sheet.pairs[0];
    let height = 400.0 * 400.0 / 600.0;
    assert!((pair.copy.size[1] - height).abs() < 1e-9 && pair.copy.size[0] == 400.0);
    let (areas, extent) = practice.sheet.areas(0).expect("its areas");
    assert_eq!((areas.count(), areas.weak_pairs().len()), (3, 1));
    assert!((extent[1] - 2.0 / 3.0).abs() < 1e-9);
    assert!(
        practice.sheet.painted[0].is_none(),
        "the pack's picture, read"
    );
    let item = session
        .sheet()
        .picture_request()
        .expect("its picture asked");
    let image = PictureImage {
        size: SIZE,
        pixels: picture(BANDS),
    };
    session.sheet().picture_read(item, Some(image));
    practise(
        &mut session,
        PracticeIntent::Fill(Color::rgb(0.5, 0.35, 0.25).expect("c")),
    );
    practise(&mut session, PracticeIntent::Compare(true));
    read_wide_copy(&mut session);
    let Some((
        AreasCompared {
            read: Some(read), ..
        },
        outline,
    )) = overlay(&session)
    else {
        panic!("the copy read")
    };
    assert_eq!(read.copy.colours[0], Some(BANDS[0]));
    assert!(
        !read.strong.is_empty(),
        "the weak contrast made strong, marked"
    );
    let inside = |p: &[f64; 2]| p[0] <= 1.0 + 1e-9 && p[1] <= extent[1] + 1e-9;
    assert!(
        read.strong.iter().flatten().all(inside),
        "in the frame's units"
    );
    assert!(outline.iter().flatten().all(inside));
    // The eyedropper on the original compares the pack's picture there.
    let on = pair.original.to_canvas([0.1, 0.3]).map(|v| v as f32);
    let PracticePick::Probe { original, .. } = session.sheet().pick(on) else {
        panic!("a probe")
    };
    let expected = Color::rgb(120.0 / 255.0, 80.0 / 255.0, 60.0 / 255.0).expect("c");
    assert_eq!(original, expected);
}

#[test]
fn a_removed_crop_shows_no_areas_and_compares_nothing() {
    let mut session = opened(&[]);
    let practice = PracticeProjection::of(&session).expect("a sheet");
    assert!(practice.sheet.areas(0).is_none());
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    assert!(session.practise(PracticeIntent::Compare(true), 1).is_err());
}

#[test]
fn combinations_are_offered_only_where_a_pack_holds_crops() {
    let without = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    assert!(!without.today().crops);
    assert!(without.begin_drill(Family::Areas, 3, 4).is_none());
    let with = PracticeLog::new(Vec::new(), Vec::new(), vec![crops()]);
    assert!(with.today().crops);
    let (drill, _) = with.begin_drill(Family::Areas, 3, 4).expect("a drill");
    assert!(
        drill
            .exercises()
            .map(|l| &l.exercise)
            .all(|e| e.picture == Some(rust()))
    );
    // A stage-2 session takes the crop for its combination.
    let (session, _) = with.begin((2, None), 3, 4);
    let combination =
        (session.exercises().map(|l| &l.exercise)).find(|e| e.kind.family() == Family::Areas);
    assert_eq!(combination.and_then(|e| e.picture.clone()), Some(rust()));
}
