//! Fill, clear and transform records: each round-trips and damage is corrupt.

use super::*;
use openpaint_core::{Artwork, CanvasSize, Color, Patch, Placement, RegionMask};

/// The first edit record of `artwork`'s first layer, and that edit as plain data.
fn first(artwork: &Artwork) -> (Vec<u8>, EditParts) {
    let bytes = encode_edit(&artwork.layers()[0].edits()[0]);
    (bytes, artwork.to_parts().layers[0].edits[0].clone())
}

fn assert_damage_is_corrupt(bytes: &[u8]) {
    assert!(decode_edit(&bytes[..bytes.len() - 1]).is_err(), "cut");
    assert!(decode_edit(&[bytes, &[0][..]].concat()).is_err(), "longer");
    assert!(
        decode_edit(&[&[9], &bytes[1..]].concat()).is_err(),
        "no such kind"
    );
}

#[test]
fn a_fill_record_round_trips_and_damage_is_corrupt() {
    let canvas = CanvasSize::new(300, 4).expect("size");
    let mut inside = vec![false; 1200];
    inside[310..590].fill(true);
    let mask = RegionMask::from_pixels(canvas, &inside).expect("mask");
    let color = Color::rgb(0.25, 0.5, 1.0).expect("colour");
    let artwork = Artwork::new(canvas)
        .fill_region(mask, color)
        .expect("fill")
        .artwork;
    let (bytes, parts) = first(&artwork);
    // One row from x 10 to 289: the runs 0 (one byte) and 280 (two bytes).
    assert_eq!(bytes.len(), 1 + 4 + 12 + 1 + 16 + 4 + 1 + 2);
    assert_eq!(bytes[0], KIND_FILL);
    assert_eq!(decode_edit(&bytes), Ok(parts));
    assert_damage_is_corrupt(&bytes);
}

#[test]
fn a_clear_record_round_trips_and_damage_is_corrupt() {
    let canvas = CanvasSize::new(40, 30).expect("size");
    let clip = RegionMask::polygon(canvas, &[[3.0, 4.0], [15.0, 4.0], [9.0, 13.0]]);
    let recipe = std::sync::Arc::new(openpaint_core::BrushRecipe::plain());
    let style = openpaint_core::StrokeStyle::new(recipe, 4.0, 1.0, Color::BLACK).expect("style");
    let point = openpaint_core::CanvasPoint::new(9.0, 8.0).expect("point");
    let sample = openpaint_core::StrokeSample::new(point, 0.5, 0.0, 0.0, 0).expect("sample");
    let drawn = Artwork::new(canvas)
        .begin_stroke(style, sample)
        .expect("begin");
    let artwork = drawn
        .finish()
        .artwork
        .cut(clip.as_ref())
        .expect("cut")
        .artwork;
    let (bytes, parts) = first(&artwork);
    assert_eq!(bytes[0], KIND_CLEAR);
    assert_eq!(decode_edit(&bytes), Ok(parts));
    assert_damage_is_corrupt(&bytes);
}

#[test]
fn a_transform_with_pixels_keeps_every_pixel_and_damage_is_corrupt() {
    let canvas = CanvasSize::new(40, 30).expect("size");
    let pixels: Vec<u8> = (0..12 * 9 * 4).map(|i| (i * 7 % 256) as u8).collect();
    let patch = Patch::new(canvas, [3, 4], [12, 9], pixels).expect("patch");
    let placement = Placement::around([9.0, 8.0], 1.25, 0.4, [false, true]).expect("map");
    for lift in [
        None,
        RegionMask::polygon(canvas, &[[3.0, 4.0], [15.0, 4.0], [9.0, 13.0]]),
    ] {
        let artwork = Artwork::new(canvas)
            .transform_region(lift, patch.clone(), placement)
            .expect("transform")
            .artwork;
        let (bytes, parts) = first(&artwork);
        assert_eq!(bytes[0], KIND_TRANSFORM);
        assert_eq!(decode_edit(&bytes), Ok(parts));
        assert!(
            decode_edit(&bytes[..bytes.len() - 20]).is_err(),
            "a cut file"
        );
    }
}

#[test]
fn a_transform_record_holds_what_moved_and_where_and_damage_is_corrupt() {
    let canvas = CanvasSize::new(40, 30).expect("size");
    let placement = Placement::around([9.0, 8.0], 3.0, 0.4, [true, false]).expect("map");
    let lift = RegionMask::polygon(canvas, &[[3.0, 4.0], [15.0, 4.0], [9.0, 13.0]]);
    for (lift, length) in [(None, 1 + 4 + 1 + 24 + 1), (lift, 0)] {
        let artwork = Artwork::new(canvas)
            .place_region(lift, placement)
            .expect("transform")
            .artwork;
        let (bytes, parts) = first(&artwork);
        assert_eq!(bytes[0], KIND_TRANSFORM);
        if length > 0 {
            assert_eq!(bytes.len(), length, "no pixels");
        }
        assert_eq!(decode_edit(&bytes), Ok(parts));
        assert_damage_is_corrupt(&bytes);
    }
}
