//! The sheet (S75, S99): pairs three a row, apart and inside; a picture's frames of its shape,
//! its longer side a frame's, in its pair's place; a picture filling its frame; a colour copy's
//! one pair shaped as its picture.

use openpaint_practice::{
    Exercise, FRAME, ItemRef, Kind, Layout, Pack, PackItem, PictureFit, PictureKind, Sheet, Tier,
    stage_one,
};

#[test]
fn a_picture_fills_its_frame_one_unit_to_its_longer_side() {
    let wide = PictureFit::of([2000, 1000]);
    assert_eq!(wide.size, [1.0, 0.5]);
    assert_eq!(wide.to_frame([2000.0, 1000.0]), [1.0, 0.5]);
    assert_eq!(wide.corners()[2], [1.0, 0.5]);
    let tall = PictureFit::of([300, 900]);
    assert!((tall.size[0] - 1.0 / 3.0).abs() < 1e-12 && tall.size[1] == 1.0);
    let middle = tall.to_picture([1.0 / 6.0, 0.5]);
    assert!((middle[0] - 150.0).abs() < 1e-9 && (middle[1] - 450.0).abs() < 1e-9);
}

#[test]
fn the_sheet_lays_pairs_three_a_row_apart_and_inside() {
    let sheet = Layout::of(&[None; 18]);
    assert_eq!(sheet.size, [2920, 3160]);
    assert_eq!(
        Layout::of(&[None; 16]).size,
        [2920, 3160],
        "a first session's sheet as S75's"
    );
    assert_eq!(sheet.pairs.len(), 18);
    apart_and_inside(&sheet);
    for pair in &sheet.pairs {
        assert_eq!(pair.original.size, [FRAME, FRAME]);
        assert_eq!(pair.copy.size, [FRAME, FRAME]);
    }
    let pair = sheet.pairs[4];
    assert!(pair.copy.origin[0] > pair.original.origin[0]);
    assert_eq!(pair.copy.origin[1], pair.original.origin[1]);
    let centre = pair.centre();
    assert!(pair.original.contains(centre, 30.0) && pair.copy.contains(centre, 30.0));
}

/// No two frames of `sheet` touch, and each lies on it.
fn apart_and_inside(sheet: &Layout) {
    let [width, height] = sheet.size.map(f64::from);
    let frames: Vec<_> = (sheet.pairs.iter())
        .flat_map(|pair| [pair.original, pair.copy])
        .collect();
    for (index, a) in frames.iter().enumerate() {
        assert!(a.origin[0] >= 0.0 && a.origin[0] + a.size[0] <= width);
        assert!(a.origin[1] >= 0.0 && a.origin[1] + a.size[1] <= height);
        for b in &frames[index + 1..] {
            let apart = (0..2).any(|axis| {
                a.origin[axis] + a.size[axis] < b.origin[axis]
                    || b.origin[axis] + b.size[axis] < a.origin[axis]
            });
            assert!(apart, "{a:?} and {b:?} overlap");
        }
    }
}

#[test]
fn frames_follow_their_picture_in_their_pair_s_place() {
    let pictures = [
        Some([300, 400]),
        Some([480, 300]),
        Some([480, 480]),
        None,
        Some([1, 2048]),
    ];
    let sheet = Layout::of(&pictures);
    let squares = Layout::of(&[None; 5]);
    assert_eq!(sheet.size, squares.size, "the sheet's size never moves");
    apart_and_inside(&sheet);
    let [portrait, landscape, square, plain, thin] = [0, 1, 2, 3, 4].map(|i| sheet.pairs[i]);
    // Portrait: as tall as a frame, three quarters as wide; the copy's frame where a square
    // copy's begins, the original's ending where a square original ends.
    assert_eq!(portrait.original.size, [300.0, 400.0]);
    assert_eq!(portrait.copy.size, portrait.original.size);
    let slot = squares.pairs[0];
    assert_eq!(portrait.copy.origin, slot.copy.origin);
    assert_eq!(
        portrait.original.origin[0] + 300.0,
        slot.original.origin[0] + FRAME
    );
    // Landscape: as wide as a frame, in its slot's middle from top to bottom.
    assert_eq!(landscape.copy.size, [400.0, 250.0]);
    let slot = squares.pairs[1];
    assert_eq!(landscape.copy.origin[1], slot.copy.origin[1] + 75.0);
    assert_eq!(landscape.original.origin[1], landscape.copy.origin[1]);
    assert_eq!(
        square, squares.pairs[2],
        "a square picture's frames a square's"
    );
    assert_eq!(plain, squares.pairs[3]);
    assert!(thin.copy.size[0] > 0.0 && thin.copy.size[1] == FRAME);
    for pair in [portrait, landscape, square, plain] {
        let gap = pair.copy.origin[0] - pair.original.origin[0] - pair.original.size[0];
        assert_eq!(gap, 40.0, "the pair held together");
    }
    // One unit to the longer side: the picture's corner is the frame's.
    let fit = PictureFit::of([480, 300]);
    let corner = landscape.copy.to_canvas(fit.size);
    assert_eq!(corner, landscape.copy.corners()[2]);
    assert_eq!(landscape.copy.extent(), fit.size);
    assert_eq!(
        landscape.copy.to_frame(landscape.copy.middle()),
        [0.5, 0.3125]
    );
}

#[test]
fn a_sheet_s_frames_follow_the_pictures_its_packs_held() {
    let item = |id: &str| ItemRef {
        pack: "test-frames".to_owned(),
        item: id.to_owned(),
    };
    let mut plan = stage_one(5, &[]);
    plan.exercises[1] = Exercise {
        picture: Some(item("tall")),
        ..Exercise::generated(Kind::Shape(Tier::Entry), 9)
    };
    plan.exercises[2] = Exercise {
        picture: Some(item("gone")),
        ..Exercise::generated(Kind::Shape(Tier::Entry), 9)
    };
    let record = Sheet::session(1, 1, 0, plan);
    let pack = Pack {
        id: "test-frames".to_owned(),
        title: "Test frames".to_owned(),
        items: vec![PackItem {
            id: "tall".to_owned(),
            kind: PictureKind::Shape,
            tier: Tier::Entry,
            size: [600, 800],
            title: "Tall".to_owned(),
            credit: None,
            outline: None,
            masses: None,
            areas: None,
        }],
    };
    let framed = record.clone().framed(std::slice::from_ref(&pack));
    assert_eq!(framed.laid(1).map(|laid| laid.frame), Some([600, 800]));
    let sheet = Layout::of_sheet(&framed);
    assert_eq!(sheet.pairs[1].copy.size, [300.0, 400.0]);
    assert_eq!(
        sheet.pairs[2].copy.size,
        [FRAME, FRAME],
        "a picture no pack held when it started: square"
    );
    assert_eq!(sheet.pairs[0].copy.size, [FRAME, FRAME], "a line's");
    let without = Layout::of_sheet(&record);
    assert_eq!(without.pairs[1].copy.size, [FRAME, FRAME]);
    assert_eq!(without.size, sheet.size);
    // A colour copy's sheet is laid by its picture's size as it was when it began.
    let copy = Sheet::copy(2, (5, 0), item("tall")).framed(&[pack]);
    assert_eq!(Layout::of_sheet(&copy), Layout::study([600, 800]));
}

#[test]
fn a_colour_copy_s_sheet_is_one_pair_of_frames_shaped_as_its_picture() {
    let sheet = Layout::study([1200, 900]);
    assert_eq!(sheet.pairs.len(), 1);
    let pair = sheet.pairs[0];
    assert_eq!(pair.original.size, [1600.0, 1200.0]);
    assert_eq!(pair.copy.size, pair.original.size);
    assert_eq!(pair.copy.origin[1], pair.original.origin[1]);
    assert!(pair.copy.origin[0] > pair.original.origin[0] + 1600.0);
    assert_eq!(sheet.size, [3400, 1360]);
    let tall = Layout::study([600, 1000]);
    assert_eq!(tall.pairs[0].copy.size, [960.0, 1600.0]);
    assert_eq!(
        tall.pairs[0].copy.unit(),
        1600.0,
        "one unit to the longer side"
    );
    // Read back from its canvas, as a copy whose picture was removed is laid out.
    for picture in [[1200, 900], [600, 1000], [777, 333], [1, 2048]] {
        let made = Layout::study(picture);
        assert_eq!(Layout::study_on(made.size), made, "{picture:?}");
    }
    // The picture fills its frame; frame units run evenly, one to the frame's longer side.
    let fit = PictureFit::of([1200, 900]);
    assert_eq!(fit.size, [1.0, 0.75]);
    let corner = pair.copy.to_canvas([1.0, 0.75]);
    assert_eq!(
        corner,
        [pair.copy.origin[0] + 1600.0, pair.copy.origin[1] + 1200.0]
    );
    assert!(pair.copy.contains(corner, 0.0));
    assert_eq!(pair.copy.to_frame(corner), [1.0, 0.75]);
}
