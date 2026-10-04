//! Transform records at every output scale (S31): content scaled up stays sharp at 2x and 4x
//! (a hard edge one output pixel wide, where the S20 patch blurred it over the placement's
//! scale); every source reduced back to 1x within a stated tolerance; shrunk below half, a
//! record antialiases by its footprint; nested records; banded export equals one piece.

use openpaint_core::{Artwork, Color, Placement, RegionMask};
use openpaint_render::{
    Banding, CanvasTarget, PixelRect, RegionEvaluator, RgbaImage, changed_regions,
};

use crate::fixture::{SIZE, blank, cold, core, plain, request, sample, with_stroke};
use crate::records::{busy, by_patch, by_record, canvas, lasso, turned};
use crate::scale::{difference, export, reduced, whole};

/// A black bar 20 px thick from (100, 90) to (220, 90).
fn bar() -> Artwork {
    let stroke = (
        plain(20.0, 1.0, Color::BLACK),
        vec![sample(100.0, 90.0, 1.0, 0), sample(220.0, 90.0, 1.0, 8)],
    );
    with_stroke(&blank(), stroke)
}

/// Three times as large about a point just below the bar's middle, so its edges land inside
/// pixels (at y 59.4 and 119.4).
fn tripled() -> Placement {
    Placement::around([160.0, 90.3], 3.0, 0.0, [false; 2]).expect("placement")
}

/// Down the column `x` of `image`: how many pixels lie strictly between paper and ink.
fn partial_rows(image: &RgbaImage, x: u32) -> usize {
    let [width, height] = image.size();
    (0..height)
        .filter(|y| {
            let value = image.pixels()[((y * width + x) * 4) as usize];
            (8..=247).contains(&value)
        })
        .count()
}

#[test]
fn a_bar_scaled_up_three_times_keeps_a_one_pixel_edge_at_every_scale() {
    let core = core();
    let record = by_record(&bar(), None, tripled());
    let patch = by_patch(&core, &bar(), None, tripled());
    for scale in [1, 2, 4] {
        let sharp = whole(&core, &record, scale);
        let soft = whole(&core, &patch, scale);
        // The bar's middle column: its top and bottom edge cross it, 60 px apart at 1x.
        let column = 160 * scale;
        let (edges, blurred) = (partial_rows(&sharp, column), partial_rows(&soft, column));
        println!("{scale}x: record {edges} partial pixels across both edges, patch {blurred}");
        assert!(edges <= 2, "{scale}x: {edges} partial pixels");
        if scale == 4 {
            assert!(blurred >= 4 * edges, "the patch blurs over the scale");
        }
    }
}

#[test]
fn every_source_of_a_record_at_2x_and_4x_reduces_to_its_1x_look() {
    let core = core();
    // A selection's cut is its mask's pixel edge enlarged: at 1x a pixel lies in or out, at
    // 2x and 4x its parts do, so reduced they differ there by up to three quarters.
    for (clip, bound) in [(None, 64), (Some(lasso()), 200)] {
        let record = by_record(&busy(), clip.as_ref(), turned());
        let canvas = cold(&core, &record);
        for scale in [2, 4] {
            let large = whole(&core, &record, scale);
            let (largest, mean) = difference(&reduced(&large, scale), canvas.pixels());
            println!("{scale}x reduced against 1x: largest {largest}, mean {mean:.3}");
            assert!(
                mean < 0.5 && largest < bound,
                "{scale}x: largest {largest}, mean {mean}"
            );
        }
    }
}

/// Fine hatching: lines 2 px wide, 6 px apart.
fn hatching() -> Artwork {
    let mut artwork = blank();
    for i in 0..30u16 {
        let x = 20.0 + 9.0 * f32::from(i);
        let line = (
            plain(2.0, 1.0, Color::BLACK),
            vec![sample(x, 20.0, 1.0, 0), sample(x + 30.0, 180.0, 1.0, 8)],
        );
        artwork = with_stroke(&artwork, line);
    }
    artwork
}

#[test]
fn shrunk_to_a_quarter_a_record_antialiases_by_its_footprint() {
    let core = core();
    let quarter = Placement::around([160.0, 100.0], 0.25, 0.2, [false; 2]).expect("quarter");
    let record = by_record(&hatching(), None, quarter);
    // The reference: the same record at 4x, each 4 x 4 block's mean (a box filter).
    let reference = reduced(&whole(&core, &record, 4), 4);
    let (largest, mean) = difference(cold(&core, &record).pixels(), &reference);
    let patch = by_patch(&core, &hatching(), None, quarter);
    let (patch_largest, patch_mean) = difference(cold(&core, &patch).pixels(), &reference);
    println!(
        "a quarter, against the box-filtered 4x: record largest {largest}, mean {mean:.3}; \
         patch largest {patch_largest}, mean {patch_mean:.3}"
    );
    // 2026-10-04: record largest 94, mean 0.26; the S20 patch (point-sampled bilinear) largest
    // 136, mean 1.11.
    assert!(
        mean < 0.5 && largest < 112,
        "largest {largest}, mean {mean}"
    );
    assert!(
        mean * 2.0 < patch_mean,
        "the record aliases less than the patch"
    );
}

/// Whole pixels moved: S20's patches are exact then, so records must show the same, up to the
/// patch's own 8-bit rounding of the layer.
fn moved_by_whole_pixels(lifts: &[Option<RegionMask>]) {
    let core = core();
    let step = Placement::moved([3.0, 1.0]).expect("move");
    let (mut records, mut patches) = (busy(), busy());
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut evaluator = RegionEvaluator::new(&core, &mut target);
    evaluator
        .evaluate(&request(&records), &[PixelRect::full(SIZE)])
        .expect("base");
    for lift in lifts {
        let next = by_record(&records, lift.as_ref(), step);
        let regions = changed_regions(&records, &next);
        evaluator.evaluate(&request(&next), &regions).expect("step");
        records = next;
        patches = by_patch(&core, &patches, lift.as_ref(), step);
    }
    let incremental = target.read(&core).expect("read");
    assert_eq!(
        incremental,
        cold(&core, &records),
        "incremental equals cold"
    );
    let (largest, mean) = difference(
        cold(&core, &records).pixels(),
        cold(&core, &patches).pixels(),
    );
    println!(
        "{} whole-pixel moves: largest {largest}, mean {mean:.4}",
        lifts.len()
    );
    assert!(largest <= 2, "largest {largest}, mean {mean}");
}

#[test]
fn a_selection_nudged_eight_times_shows_what_eight_exact_patches_showed() {
    let lifts: Vec<Option<RegionMask>> = (0..8)
        .map(|nudge| {
            let (x, y) = (40.0 + 3.0 * nudge as f32, 20.0 + nudge as f32);
            let corners = [
                [x, y],
                [x + 190.0, y],
                [x + 170.0, y + 150.0],
                [x, y + 130.0],
            ];
            RegionMask::polygon(canvas(), &corners)
        })
        .collect();
    moved_by_whole_pixels(&lifts);
}

#[test]
fn interleaved_selections_moved_four_times_show_what_exact_patches_showed() {
    let inside: Vec<bool> = (0..320 * 200).map(|at| (at / 7) % 2 == 0).collect();
    let stripes = RegionMask::from_pixels(canvas(), &inside).ok();
    moved_by_whole_pixels(&[stripes.clone(), None, stripes.clone(), stripes]);
}

/// A layer moved whole, a selection of it moved, the whole layer turned, then a selection of
/// that moved again.
fn nested(core: &crate::fixture::Shared, records: bool) -> Artwork {
    let small = RegionMask::polygon(
        canvas(),
        &[[120.0, 40.0], [230.0, 60.0], [210.0, 140.0], [110.0, 120.0]],
    )
    .expect("a quad");
    let steps = [
        (None, Placement::moved([-20.0, 6.0]).expect("move")),
        (Some(lasso()), turned()),
        (
            None,
            Placement::around([160.0, 100.0], 0.8, -0.3, [true, false]).expect("turn"),
        ),
        (
            Some(small),
            Placement::around([170.0, 90.0], 1.6, 0.5, [false; 2]).expect("up"),
        ),
    ];
    let mut artwork = busy();
    for (clip, placement) in steps {
        artwork = if records {
            by_record(&artwork, clip.as_ref(), placement)
        } else {
            by_patch(core, &artwork, clip.as_ref(), placement)
        };
    }
    artwork
}

#[test]
fn nested_records_show_what_nested_patches_showed_and_band_as_one_piece() {
    let core = core();
    let records = nested(&core, true);
    let patches = nested(&core, false);
    let (largest, mean) = difference(
        cold(&core, &records).pixels(),
        cold(&core, &patches).pixels(),
    );
    println!("nested, records against patches at 1x: largest {largest}, mean {mean:.3}");
    // Four resamplings blur the patches; the records evaluate once.
    assert!(mean < 3.0, "largest {largest}, mean {mean}");
    for scale in [2, 4] {
        let bands = Banding {
            rows: 37,
            columns: 100 * scale,
        };
        let banded = export(&core, &records, scale, bands);
        assert_eq!(banded, whole(&core, &records, scale), "{scale}x bands");
    }
    let one = Banding {
        rows: 29,
        columns: 70,
    };
    assert_eq!(export(&core, &records, 1, one), cold(&core, &records), "1x");
}
