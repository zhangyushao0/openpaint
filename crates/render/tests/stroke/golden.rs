//! A fixed stroke evaluates to fixed pixels.

use openpaint_core::{ArtworkCommand, LayerCommand};

use crate::fixture::{SIZE, blank, cold, core, pixel, wave, with_stroke};

/// `cargo run -p openpaint-harness -- render-stroke` renders the same wave and prints this hash.
/// Same on Vulkan and DX12.
const GOLDEN: u64 = 0xba85_2310_2ad3_8e3a;
const WHITE: [u8; 4] = [255; 4];
/// 80 % of (0.12, 0.25, 0.65) over white, rounded to 8 bits.
const FULL: [u8; 4] = [75, 102, 184, 255];

#[test]
fn wave_matches_the_golden_hash() {
    let core = core();
    let image = cold(&core, &with_stroke(&blank(), wave()));
    assert_eq!(image.size(), SIZE);
    assert_eq!(
        image.content_hash(),
        GOLDEN,
        "hash {:016x} on {}",
        image.content_hash(),
        core.adapter()
    );
}

#[test]
fn the_body_is_one_flat_colour_without_seams() {
    let image = cold(&core(), &with_stroke(&blank(), wave()));
    // Along the middle of the stroke every pixel is fully covered by several overlapping
    // segments; the stroke's opacity applies once, so they all match.
    for i in 10..=37u16 {
        let t = f32::from(i) / 47.0;
        let x = 30.0 + 260.0 * t;
        let y = 100.0 + 55.0 * (std::f32::consts::TAU * t).sin();
        assert_eq!(pixel(&image, [x as u32, y as u32]), FULL, "at t = {t}");
    }
}

#[test]
fn pressure_tapers_the_ends() {
    let image = cold(&core(), &with_stroke(&blank(), wave()));
    // At the ends the radius is 2 px; in the middle it is 20 px.
    assert_eq!(pixel(&image, [30, 100]), FULL);
    assert_eq!(pixel(&image, [30, 106]), WHITE);
    assert_eq!(pixel(&image, [24, 100]), WHITE);
    assert_eq!(pixel(&image, [160, 117]), FULL);
    assert_eq!(pixel(&image, [0, 0]), WHITE);
}

#[test]
fn a_hidden_layer_leaves_the_paper() {
    let drawn = with_stroke(&blank(), wave());
    let layer = drawn.active();
    let hide = LayerCommand::SetVisible {
        layer,
        visible: false,
    };
    let hidden = drawn
        .apply(ArtworkCommand::Layer(hide))
        .expect("hide")
        .artwork;
    let image = cold(&core(), &hidden);
    let (pixels, _) = image.pixels().as_chunks::<4>();
    assert!(pixels.iter().all(|pixel| *pixel == WHITE));
}
