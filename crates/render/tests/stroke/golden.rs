//! A fixed stroke evaluates to fixed pixels.

use openpaint_core::{ArtworkCommand, LayerCommand};

use crate::fixture::{SIZE, blank, cold, core, pixel, rich, wave, wave_with, with_stroke};

/// `cargo run -p openpaint-harness -- render-stroke` renders the same wave and prints this hash.
/// Same on Vulkan and DX12.
const GOLDEN: u64 = 0xecd7_fdc1_3320_2509;
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

/// The wave drawn with every recipe stage at work (`fixture::rich`); same on Vulkan and DX12.
const RICH_GOLDEN: u64 = 0x420e_f5b6_3fc6_c3b5;

#[test]
fn a_rich_recipe_matches_its_golden_hash() {
    let core = core();
    let image = cold(&core, &with_stroke(&blank(), wave_with(rich())));
    assert_eq!(
        image.content_hash(),
        RICH_GOLDEN,
        "hash {:016x} on {}",
        image.content_hash(),
        core.adapter()
    );
    // Grain takes paint away in places: the body is no longer one flat colour.
    let row: Vec<[u8; 4]> = (140..180).map(|x| pixel(&image, [x, 100])).collect();
    assert!(row.windows(2).any(|pair| pair[0] != pair[1]), "{row:?}");
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
