//! Transforms: placements (around a pivot, composed, inverted, checked), the transform's place
//! among the layer's strokes, lock, storage round trips and rejections, history cost.

use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError, EditParts,
    LayerCommand, Patch, Placement, RegionMask, StrokeSample, StrokeStyle,
};

fn canvas() -> CanvasSize {
    CanvasSize::new(40, 30).unwrap()
}

fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
}

fn draw(artwork: &Artwork) -> Artwork {
    let recipe = Arc::new(BrushRecipe::plain());
    let style = StrokeStyle::new(recipe, 2.0, 1.0, Color::BLACK).unwrap();
    let sample = |x: f32, t: u32| {
        StrokeSample::new(CanvasPoint::new(x, 3.0).unwrap(), 1.0, 0.0, 0.0, t).unwrap()
    };
    let mut session = artwork.begin_stroke(style, sample(1.0, 0)).unwrap();
    session.extend(&[sample(9.0, 4)]).unwrap();
    session.finish().artwork
}

/// A 3 x 2 patch at (4, 5), each pixel a different grey.
fn patch() -> Patch {
    let pixels = (0..6u8)
        .flat_map(|i| [i * 10, i * 10, i * 10, 255])
        .collect();
    Patch::new(canvas(), [4, 5], [3, 2], pixels).unwrap()
}

#[test]
fn a_placement_turns_about_its_pivot_and_composes_and_inverts() {
    let pivot = [10.0, 20.0];
    let quarter = Placement::around(pivot, 2.0, FRAC_PI_2, [false; 2]).unwrap();
    assert!(close(quarter.apply(pivot), pivot), "the pivot stays");
    // Clockwise on the canvas (y down): one pixel right of the pivot goes two pixels down.
    assert!(close(quarter.apply([11.0, 20.0]), [10.0, 22.0]));
    let moved = Placement::moved([3.0, -1.0]).unwrap();
    let both = quarter.then(moved).unwrap();
    assert!(
        close(both.apply([11.0, 20.0]), [13.0, 21.0]),
        "then moves after"
    );
    let [a, b, c, d, e, f] = both.inverse_rows();
    let [x, y] = both.apply([7.5, -2.0]);
    assert!(close([a * x + b * y + c, d * x + e * y + f], [7.5, -2.0]));
    let mirrored = Placement::around(pivot, 1.0, 0.0, [true, false]).unwrap();
    assert!(close(mirrored.apply([12.0, 25.0]), [8.0, 25.0]));
    assert_eq!(
        Placement::IDENTITY.inverse_rows(),
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0]
    );
}

#[test]
fn placements_that_collapse_or_explode_are_refused() {
    assert!(Placement::around([0.0; 2], 0.001, 0.0, [false; 2]).is_none());
    assert!(Placement::around([0.0; 2], 1000.0, 0.0, [false; 2]).is_none());
    assert!(
        Placement::new([1.0, 1.0, 1.0, 1.0], [0.0; 2]).is_none(),
        "flat"
    );
    assert!(Placement::new([1.0, 0.0, 0.0, f32::NAN], [0.0; 2]).is_none());
    assert!(Placement::moved([2.0e6, 0.0]).is_none());
}

#[test]
fn a_patch_must_fit_its_pixels_and_the_canvas() {
    assert!(Patch::new(canvas(), [38, 0], [3, 1], vec![0; 12]).is_err());
    assert!(Patch::new(canvas(), [0, 0], [3, 1], vec![0; 11]).is_err());
    assert!(Patch::new(canvas(), [0, 0], [0, 1], Vec::new()).is_err());
}

#[test]
fn a_transform_lies_after_the_strokes_so_far_and_respects_the_lock() {
    let drawn = draw(&Artwork::new(canvas()));
    let placement = Placement::moved([5.0, 2.0]).unwrap();
    let moved = drawn
        .transform_region(None, patch(), placement)
        .unwrap()
        .artwork;
    let id = drawn.active();
    let edit = moved.layer(id).unwrap().edits()[0].transform().unwrap();
    assert_eq!((edit.after(), edit.placement()), (1, placement));
    assert!(edit.lift().is_none(), "the whole layer");
    let lock = LayerCommand::SetLocked {
        layer: id,
        locked: true,
    };
    let locked = drawn.apply(ArtworkCommand::Layer(lock)).unwrap().artwork;
    assert_eq!(
        locked.transform_region(None, patch(), placement),
        Err(CoreError::LayerLocked(id))
    );
}

#[test]
fn transforms_round_trip_through_storage_and_bad_ones_are_rejected() {
    let mut inside = vec![false; 40 * 30];
    inside[5 * 40 + 4..5 * 40 + 7].fill(true);
    let lift = RegionMask::from_pixels(canvas(), &inside).unwrap();
    let turned = Placement::around([5.0, 6.0], 1.5, 0.3, [true, false]).unwrap();
    let moved = draw(&Artwork::new(canvas()))
        .transform_region(Some(lift), patch(), turned)
        .unwrap()
        .artwork;
    let parts = moved.to_parts();
    let loaded = Artwork::from_storage(parts.clone()).unwrap();
    assert_eq!(loaded, moved);
    assert_eq!(loaded.to_parts(), parts);
    let rejects = |change: fn(&mut openpaint_core::TransformParts)| {
        let mut broken = parts.clone();
        let EditParts::Transform(edit) = &mut broken.layers[0].edits[0] else {
            panic!("a transform");
        };
        change(edit);
        Artwork::from_storage(broken).is_err()
    };
    assert!(rejects(|edit| edit.after = 2), "past the strokes");
    assert!(rejects(|edit| {
        if let Some(patch) = &mut edit.patch {
            patch.origin = [38, 0];
        }
    }));
    assert!(rejects(|edit| {
        if let Some(patch) = &mut edit.patch {
            patch.pixels.pop();
        }
    }));
    assert!(rejects(|edit| edit.matrix = [0.0; 4]), "collapsed");
    assert!(rejects(|edit| {
        if let Some(lift) = &mut edit.lift {
            lift.runs.push(1);
        }
    }));
}

#[test]
fn a_transform_costs_its_pixels_in_history_and_later_versions_share_them() {
    let base = Artwork::new(canvas());
    let moved = base
        .transform_region(None, patch(), Placement::IDENTITY)
        .unwrap()
        .artwork;
    assert!(moved.bytes_since(&base) >= patch().pixels().len());
    let drawn = draw(&moved);
    assert_eq!(
        drawn.bytes_since(&moved),
        draw(&base).bytes_since(&base),
        "the pixels are shared"
    );
}

#[test]
fn a_transform_record_holds_no_pixels_and_round_trips() {
    let drawn = draw(&Artwork::new(canvas()));
    let turned = Placement::around([5.0, 6.0], 3.0, 0.3, [false, true]).unwrap();
    let whole = drawn.place_region(None, turned).unwrap().artwork;
    let edit = whole.layer(drawn.active()).unwrap().edits()[0]
        .transform()
        .unwrap();
    assert_eq!((edit.after(), edit.placement()), (1, turned));
    assert!(edit.patch().is_none() && edit.lift().is_none(), "a record");
    let mut inside = vec![false; 40 * 30];
    inside[2 * 40 + 1..2 * 40 + 9].fill(true);
    let lift = RegionMask::from_pixels(canvas(), &inside).unwrap();
    let both = whole
        .place_region(Some(lift.clone()), turned)
        .unwrap()
        .artwork;
    println!(
        "history bytes: whole {}, selection {} ({} mask bytes)",
        whole.bytes_since(&drawn),
        both.bytes_since(&whole),
        size_of_val(lift.words())
    );
    // A version's own records (under a kilobyte) and, for the selection, its mask's words.
    assert!(whole.bytes_since(&drawn) < 1024);
    assert!(both.bytes_since(&whole) < 1024 + size_of_val(lift.words()));
    let parts = both.to_parts();
    let EditParts::Transform(stored) = &parts.layers[0].edits[1] else {
        panic!("a transform");
    };
    assert!(stored.patch.is_none());
    let loaded = Artwork::from_storage(parts.clone()).unwrap();
    assert_eq!(loaded, both);
    assert_eq!(loaded.to_parts(), parts);
    let lock = LayerCommand::SetLocked {
        layer: drawn.active(),
        locked: true,
    };
    let locked = drawn.apply(ArtworkCommand::Layer(lock)).unwrap().artwork;
    assert_eq!(
        locked.place_region(None, turned),
        Err(CoreError::LayerLocked(drawn.active()))
    );
}
