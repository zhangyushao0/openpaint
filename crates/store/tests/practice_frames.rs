//! Practice sheets laid before 1.17.0 (S99): a record written without its layout has its sheet
//! laid anew once, when the library opens with its packs: each copy of a picture a pack holds
//! moved from where the picture lay in its square frame onto the frame of the picture's shape,
//! every sample at the same share of the picture, its clip kept to the new frame, ink in the old
//! margin kept outside it; other kinds' strokes and a copy of a picture no pack holds as they
//! were; the record marked, and nothing written at the next opening.

mod common;

use std::sync::Arc;

use common::{TestDir, records, untitled, write_records};
use openpaint_app::{ImageFormat, ProjectId, ProjectStore};
use openpaint_core::{
    Artwork, BrushRecipe, CanvasPoint, CanvasSize, Color, RegionMask, StrokeParts, StrokeSample,
    StrokeStyle,
};
use openpaint_practice::{
    Exercise, ItemRef, Kind, LineTier, PERSONAL, Plan, Point, SessionRecord, Sheet, Tier,
};
use openpaint_store::{Library, Packs, encode_image, pack_picture};

/// A picture of the user's own, upright 300 by 400, added to the personal pack in `packs`.
fn portrait(packs: &Packs) -> ItemRef {
    let pixels: Vec<u8> = [120, 90, 60, 255].repeat(300 * 400);
    let file = encode_image(ImageFormat::Png, [300, 400], &pixels).expect("a picture");
    let picture = pack_picture(&file).expect("taken");
    let pack = (packs.add_picture(&picture, (None, Tier::Entry))).expect("added");
    ItemRef {
        pack: PERSONAL.to_owned(),
        item: pack.items[0].id.clone(),
    }
}

/// `artwork` with a stroke through `points` (canvas pixels), clipped to the frame of `corners`.
fn drawn(artwork: &Artwork, points: &[Point], corners: [Point; 4]) -> Artwork {
    let recipe = Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 5.0, 1.0, Color::BLACK).expect("style");
    let sample = |i: usize| {
        let [x, y] = points[i].map(|v| v as f32);
        let point = CanvasPoint::new(x, y).expect("point");
        StrokeSample::new(point, 0.5, 0.0, 0.0, i as u32 * 4).expect("sample")
    };
    let corners = corners.map(|p| p.map(|v| v as f32));
    let clip = RegionMask::polygon(artwork.canvas(), &corners).expect("clip");
    let session = (artwork.begin_stroke(style, sample(0)).expect("begin")).clipped(clip);
    let mut session = session.expect("clipped");
    let rest: Vec<StrokeSample> = (1..points.len()).map(sample).collect();
    session.extend(&rest).expect("extend");
    session.finish().artwork
}

/// A line, a shape of the picture `held` and one of a picture no pack holds.
fn record(held: &ItemRef) -> SessionRecord {
    let shape = |picture: ItemRef| Exercise {
        picture: Some(picture),
        ..Exercise::generated(Kind::Shape(Tier::Entry), 2)
    };
    let gone = ItemRef {
        pack: PERSONAL.to_owned(),
        item: "mine-0000gone".to_owned(),
    };
    let plan = Plan {
        exercises: vec![
            Exercise::generated(Kind::Line(LineTier::Arc), 1),
            shape(held.clone()),
            shape(gone),
        ],
        warm_up: 0,
    };
    SessionRecord::begun(1, (2, 1), (5, 0), plan)
}

/// Where a picture 300 by 400 lay in `square` up to 1.16.0, at a share of it: 270 by 360 from
/// (65, 20).
fn as_laid_before(square: [Point; 4], [u, v]: Point) -> Point {
    let [x, y] = square[0];
    [x + 65.0 + 270.0 * u, y + 20.0 + 360.0 * v]
}

const SHARES: [Point; 4] = [[0.0, 0.0], [1.0, 1.0], [0.5, 0.25], [0.2, 0.9]];

/// The sheet as 1.16.0 left it, its record without its layout, in `dir`.
fn laid_before(dir: &std::path::Path, record: &SessionRecord) -> (ProjectId, Artwork) {
    let old = Sheet::of(&[None; 3]);
    let blank = Artwork::new(CanvasSize::new(old.size[0], old.size[1]).expect("size"));
    let [line, shape, gone] = [0, 1, 2].map(|i| old.pairs[i].copy.corners());
    let copied = SHARES.map(|share| as_laid_before(shape, share));
    let mut sheet = drawn(&blank, &copied, shape);
    let [x, y] = line[0];
    sheet = drawn(
        &sheet,
        &[[x + 100.0, y + 200.0], [x + 300.0, y + 210.0]],
        line,
    );
    let [x, y] = shape[0];
    sheet = drawn(
        &sheet,
        &[[x + 10.0, y + 200.0], [x + 30.0, y + 210.0]],
        shape,
    );
    let [x, y] = gone[0];
    sheet = drawn(
        &sheet,
        &[[x + 50.0, y + 50.0], [x + 350.0, y + 350.0]],
        gone,
    );
    let mut library = Library::at(dir);
    let title = untitled();
    let id = (library.start_practice(&blank, &title, record)).expect("started");
    library.save(id, &sheet).expect("saved");
    drop(library);
    let key = [b"p".as_slice(), &record.id.to_be_bytes()].concat();
    let mut stored = records(dir);
    let at = (stored.iter())
        .position(|(_, k, _)| *k == key)
        .expect("the record");
    assert_eq!(stored[at].2.pop(), Some(1), "its layout, the last byte");
    write_records(dir, std::slice::from_ref(&stored[at]));
    (id, sheet)
}

fn strokes(artwork: &Artwork) -> Vec<StrokeParts> {
    artwork.to_parts().layers[0].strokes.clone()
}

#[test]
fn an_old_copy_of_a_picture_is_laid_on_the_frame_of_its_shape_once() {
    let dir = TestDir::new("practice-frames");
    let (library, packs) = (dir.join("library"), dir.join("packs"));
    let held = portrait(&Packs::at(&packs));
    let record = record(&held);
    let (id, before) = laid_before(&library, &record);
    let mut opened = Library::at(&library).with_packs(Packs::at(&packs));
    let sessions = opened.practice().expect("practice");
    assert_eq!(
        sessions,
        vec![SessionRecord {
            canvas: id.get(),
            ..record
        }]
    );
    let laid = opened.sheet(id).expect("its sheet");
    drop(opened);
    let held_packs = Packs::at(&packs).list().expect("listed");
    let frames = Sheet::of_session(&sessions[0], &held_packs);
    let frame = frames.pairs[1].copy;
    assert_eq!(frame.size, [300.0, 400.0]);
    let [was, now] = [strokes(&before), strokes(&laid)];
    // The copy: every sample at the share of the picture it lay at.
    for (sample, [u, v]) in now[0].samples.iter().zip(SHARES) {
        let [x, y] = [frame.origin[0] + 300.0 * u, frame.origin[1] + 400.0 * v];
        let off = (f64::from(sample.x) - x).hypot(f64::from(sample.y) - y);
        assert!(off < 1e-3, "{u}, {v}: {off}");
    }
    let clip = now[0].clip.as_ref().expect("its clip");
    let origin = frame.origin.map(|v| v as u32);
    assert_eq!(
        (clip.origin, clip.size),
        (origin, [300, 400]),
        "the new frame"
    );
    assert_eq!(now[1], was[1], "a line's stroke as it was");
    assert!(
        f64::from(now[2].samples[0].x) < frame.origin[0],
        "ink in the old margin kept, outside the frame"
    );
    assert_eq!(now[3], was[3], "a picture no pack holds: as it was");
    assert_eq!(frames.pairs[2].copy.size, [400.0, 400.0]);
    // Laid once: opened again, nothing written.
    let after = records(&library);
    let mut again = Library::at(&library).with_packs(Packs::at(&packs));
    assert_eq!(again.practice().expect("practice"), sessions);
    drop(again);
    assert_eq!(records(&library), after);
}

#[test]
fn opened_without_packs_an_old_sheet_keeps_its_strokes_and_is_marked() {
    let dir = TestDir::new("practice-frames-bare");
    let (library, packs) = (dir.join("library"), dir.join("packs"));
    let held = portrait(&Packs::at(&packs));
    let record = record(&held);
    let (id, before) = laid_before(&library, &record);
    let mut opened = Library::at(&library);
    let laid = opened.sheet(id).expect("its sheet");
    assert_eq!(
        strokes(&laid),
        strokes(&before),
        "no pack: every picture square"
    );
    drop(opened);
    let key = [b"p".as_slice(), &record.id.to_be_bytes()].concat();
    let stored = records(&library);
    let value = (stored.iter())
        .find(|(_, k, _)| *k == key)
        .map(|(_, _, v)| v.clone());
    assert_eq!(
        value.and_then(|v| v.last().copied()),
        Some(1),
        "its layout written"
    );
}
