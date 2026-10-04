//! Region masks and fills: packing, runs for storage, the fill's place among the layer's
//! strokes, Alpha Lock and lock, storage round trips and rejections, history cost.

use std::sync::Arc;

use openpaint_core::{
    Artwork, ArtworkCommand, BrushRecipe, CanvasPoint, CanvasSize, Color, CoreError, EditParts,
    FillParts, LayerCommand, RegionMask, StrokeSample, StrokeStyle,
};

fn canvas() -> CanvasSize {
    CanvasSize::new(40, 6).unwrap()
}

/// A ring-shaped region with a hole, wider than one 32-bit word.
fn ring() -> RegionMask {
    let mut inside = vec![false; 40 * 6];
    for y in 1..5 {
        for x in 2..37 {
            inside[y * 40 + x] = !(y == 2 && x == 20);
        }
    }
    RegionMask::from_pixels(canvas(), &inside).unwrap()
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

fn layer(command: LayerCommand) -> ArtworkCommand {
    ArtworkCommand::Layer(command)
}

#[test]
fn a_mask_keeps_its_tight_bounds_its_hole_and_round_trips_through_runs() {
    let mask = ring();
    assert_eq!(
        (mask.origin(), mask.size(), mask.stride()),
        ([2, 1], [35, 4], 2)
    );
    assert!(mask.contains([2, 1]) && mask.contains([36, 4]));
    assert!(!mask.contains([20, 2]), "the hole stays out");
    assert!(!mask.contains([1, 1]) && !mask.contains([37, 1]) && !mask.contains([2, 5]));
    let runs = mask.runs();
    assert_eq!(runs[..3], [0, 35 + 18, 1]);
    assert_eq!(runs.iter().map(|&run| u64::from(run)).sum::<u64>(), 35 * 4);
    let back = RegionMask::from_runs(canvas(), mask.origin(), mask.size(), &runs).unwrap();
    assert_eq!(back, mask);
    assert!(!back.shares(&mask));
}

#[test]
fn bad_masks_are_rejected() {
    let invalid = Err(CoreError::InvalidValue {
        what: "region mask",
    });
    assert_eq!(RegionMask::from_pixels(canvas(), &[false; 240]), invalid);
    assert_eq!(RegionMask::from_pixels(canvas(), &[true; 3]), invalid);
    let runs = |origin, size, runs: &[u32]| RegionMask::from_runs(canvas(), origin, size, runs);
    assert_eq!(
        runs([0, 0], [4, 1], &[1, 2]),
        invalid,
        "runs short of the rectangle"
    );
    assert_eq!(runs([0, 0], [4, 1], &[1, 2, 2]), invalid, "runs past it");
    assert_eq!(runs([0, 0], [4, 1], &[4]), invalid, "nothing inside");
    assert_eq!(runs([38, 0], [4, 1], &[0, 4]), invalid, "off the canvas");
    assert_eq!(runs([0, 0], [0, 1], &[0]), invalid, "an empty rectangle");
    assert!(runs([36, 5], [4, 1], &[0, 4]).is_ok());
}

#[test]
fn a_fill_lies_after_the_strokes_so_far_and_keeps_alpha_lock() {
    let drawn = draw(&Artwork::new(canvas()));
    let id = drawn.active();
    let filled = drawn.fill_region(ring(), Color::WHITE).unwrap().artwork;
    let edit = filled.layer(id).unwrap().edits()[0].fill().unwrap();
    assert_eq!((edit.after(), edit.alpha_locked()), (1, false));
    let again = draw(&filled);
    assert_eq!(
        again.layer(id).unwrap().edits().len(),
        1,
        "a stroke keeps the fills"
    );
    let lock = LayerCommand::SetAlphaLocked {
        layer: id,
        locked: true,
    };
    let locked = again.apply(layer(lock)).unwrap().artwork;
    let atop = locked.fill_region(ring(), Color::BLACK).unwrap().artwork;
    let second = atop.layer(id).unwrap().edits()[1].fill().unwrap();
    assert_eq!((second.after(), second.alpha_locked()), (2, true));
    let hard_lock = LayerCommand::SetLocked {
        layer: id,
        locked: true,
    };
    let frozen = atop.apply(layer(hard_lock)).unwrap().artwork;
    assert_eq!(
        frozen.fill_region(ring(), Color::WHITE),
        Err(CoreError::LayerLocked(id))
    );
}

#[test]
fn a_mask_from_a_larger_canvas_is_refused() {
    let big = CanvasSize::new(80, 6).unwrap();
    let mut inside = vec![false; 80 * 6];
    inside[79] = true;
    let mask = RegionMask::from_pixels(big, &inside).unwrap();
    let result = Artwork::new(canvas()).fill_region(mask, Color::WHITE);
    assert!(matches!(result, Err(CoreError::InvalidValue { .. })));
}

#[test]
fn fills_round_trip_through_storage_and_bad_ones_are_rejected() {
    let filled = draw(&Artwork::new(canvas()))
        .fill_region(ring(), Color::rgb(0.2, 0.4, 0.6).unwrap())
        .unwrap()
        .artwork;
    let parts = filled.to_parts();
    let fill = |parts: &mut openpaint_core::ArtworkParts| match &mut parts.layers[0].edits[0] {
        EditParts::Fill(fill) => fill.clone(),
        EditParts::Transform(_) | EditParts::Adjust(_) => panic!("a fill"),
    };
    assert_eq!(fill(&mut parts.clone()).after, 1);
    let loaded = Artwork::from_storage(parts.clone()).unwrap();
    assert_eq!(loaded, filled);
    assert_eq!(loaded.to_parts(), parts);
    let rejects = |change: fn(&mut FillParts)| {
        let mut broken = parts.clone();
        let mut edit = fill(&mut broken);
        change(&mut edit);
        broken.layers[0].edits[0] = EditParts::Fill(edit);
        Artwork::from_storage(broken).is_err()
    };
    assert!(rejects(|fill| fill.after = 2), "past the strokes");
    assert!(rejects(|fill| fill.origin = [10, 3]), "off the canvas");
    assert!(rejects(|fill| fill.color = [2.0, 0.0, 0.0]));
    assert!(rejects(|fill| fill.runs.push(1)));
    let mut unordered = parts;
    let mut first = fill(&mut unordered);
    first.after = 0;
    unordered.layers[0].edits.push(EditParts::Fill(first));
    assert!(Artwork::from_storage(unordered).is_err(), "places go back");
}

#[test]
fn a_fill_costs_its_mask_in_history_and_later_versions_share_it() {
    let base = Artwork::new(canvas());
    let filled = base.fill_region(ring(), Color::WHITE).unwrap().artwork;
    let mask_bytes = ring().words().len() * 4;
    assert!(filled.bytes_since(&base) >= mask_bytes);
    let drawn = draw(&filled);
    let unfilled = draw(&base);
    assert_eq!(
        drawn.bytes_since(&filled),
        unfilled.bytes_since(&base),
        "the mask is shared"
    );
}
