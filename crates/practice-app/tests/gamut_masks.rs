//! Gamut masks on a practice sheet (S153): a crop beside an empty frame of its shape, the mask on
//! its disc and its corners' swatches beside the copy's frame; the crop never picked, a swatch
//! picked whole; Compare reading the copy on the worker against the crop's areas and the mask, the
//! app judging it (an area outside the mask marked, its reason first), the areas outside kept.

mod support;

use std::sync::Arc;

use openpaint_app::{BrushLibrary, Toolbox, Viewport};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_practice::{
    Areas, CardItem, Exercise, Figure, ItemRef, Judge, Kind, Mask, MaskCompared, MaskReading,
    MaskShape, Metric, Outcome, Pack, PackItem, PictureKind, Skill, SkillLevel, Tier, Verdict,
    beside, value,
};
use openpaint_practice_app::{
    PictureImage, PracticeIntent, PracticeLog, PracticePens, PracticePick, SheetSession, Start,
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

/// Gurney's triangle toward a blue, across from the rust: the rust must be greyed.
const BLUE: Mask = Mask {
    shape: MaskShape::Triangle,
    hue: 240,
};

fn opened(packs: &[Pack]) -> SheetSession {
    let exercise = Exercise {
        picture: Some(ItemRef {
            pack: "test-crops".to_owned(),
            item: "rust".to_owned(),
        }),
        ..Exercise::generated(Kind::GamutMask(BLUE), 7)
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

#[test]
fn the_mask_and_its_swatches_lie_beside_the_copy_a_swatch_picked_whole_the_crop_never() {
    let mut session = read_sheet();
    let practice = session.projection();
    assert_eq!(practice.kind, Some(Kind::GamutMask(BLUE)));
    let givens = practice.sheet.givens[0].clone();
    let disc = givens.iter().find_map(|mark| match &mark.drawn {
        Figure::Gamut { corners, .. } => Some(corners.clone()),
        _ => None,
    });
    assert_eq!(disc, Some(BLUE.corners()), "the mask beside the copy");
    let swatches: Vec<([f64; 2], [u8; 3])> = (givens.iter())
        .filter_map(|mark| match mark.drawn {
            Figure::Beside { at, colour, .. } => Some((at, colour)),
            _ => None,
        })
        .collect();
    assert_eq!(
        swatches.iter().map(|(_, c)| *c).collect::<Vec<_>>(),
        BLUE.swatches(),
        "its corners' colours"
    );
    let pair = practice.sheet.pairs[0];
    for (at, colour) in swatches {
        let on = beside(&pair.copy, at).map(|v| v as f32);
        let [r, g, b] = colour.map(|c| f32::from(c) / 255.0);
        let picked = Color::rgb(r, g, b).expect("a colour");
        assert_eq!(session.sheet_mut().pick(on), PracticePick::Colour(picked));
    }
    // Beside the swatches, the canvas; the crop never; the copy's frame the canvas.
    let off = beside(&pair.copy, [0.3, 0.9]).map(|v| v as f32);
    assert_eq!(session.sheet_mut().pick(off), PracticePick::Canvas);
    let crop = pair.original.to_canvas([0.1, 0.3]).map(|v| v as f32);
    assert_eq!(session.sheet_mut().pick(crop), PracticePick::Refused);
    let mine = pair.copy.to_canvas([0.1, 0.3]).map(|v| v as f32);
    assert_eq!(session.sheet_mut().pick(mine), PracticePick::Canvas);
}

#[test]
fn compare_reads_the_copy_and_the_app_judges_it_by_the_mask() {
    let mut session = read_sheet();
    practise(&mut session, PracticeIntent::Fill(Color::BLACK));
    practise(&mut session, PracticeIntent::Compare(true));
    let job = session
        .sheet_mut()
        .read_request()
        .expect("the copy asked for");
    let Some(MaskReading { mask, .. }) = job.reading.downcast::<MaskReading>() else {
        panic!("read as a gamut mask")
    };
    assert_eq!((*mask, job.original.is_some()), (BLUE, true));
    // The copy as the crop: the grey and the cream inside the mask, the rust across from it.
    let pixels = PictureImage {
        size: job.size,
        pixels: bands(job.size, BANDS),
    };
    session.sheet_mut().read(job.sequence, job.run(&pixels));
    let practice = session.projection();
    let compared = (practice.compared.as_ref())
        .and_then(|c| c.downcast::<MaskCompared>())
        .expect("compared as a gamut mask")
        .clone();
    let read = compared.read.as_ref().expect("read");
    let areas = Areas::found(SIZE, &bands(SIZE, BANDS)).areas;
    let rust = (0..areas.count())
        .find(|a| areas.colours[*a][0] == 150)
        .expect("the rust");
    assert_eq!(read.check.outside.len(), 1);
    assert!(read.check.is_outside(rust) && read.outline(rust).is_some());
    let judged = practice.judged.expect("judged");
    assert_eq!(judged.verdict, Verdict::App(Outcome::Near));
    assert_eq!(judged.result, Some((Outcome::Near, Judge::App)));
    let CardItem::Says(say) = &practice.card[0] else {
        panic!("the reason first: {:?}", practice.card[0])
    };
    assert_eq!(say.key, "outside-mask");
    let metrics = practice.result.expect("stored").metrics;
    assert_eq!(value(&metrics, Metric::MaskOutside), Some(1.0));
    // The Grey view while compared; a stroke takes the comparison away.
    practise(&mut session, PracticeIntent::Grey(true));
    assert!(session.projection().grey);
    practise(&mut session, PracticeIntent::Fill(Color::WHITE));
    assert!(session.projection().compared.is_none());
}

#[test]
fn a_lesson_of_gamut_masks_starts_once_a_pack_holds_crops() {
    let masks = Skill::of_key("gamut-mask").expect("gamut masks' skill");
    let start = Start::At(masks, SkillLevel::First);
    let without = PracticeLog::new(Vec::new(), Vec::new(), Vec::new());
    assert!(without.start(start, 3, 4).is_none());
    let with = PracticeLog::new(Vec::new(), Vec::new(), vec![crops()]);
    let (lesson, _) = with.start(start, 3, 4).expect("a lesson");
    let kinds: Vec<Kind> = lesson.exercises().map(|l| l.exercise.kind).collect();
    assert_eq!(kinds.len(), 1);
    assert!(matches!(
        kinds[0],
        Kind::GamutMask(Mask {
            shape: MaskShape::Triangle,
            ..
        })
    ));
}
