//! Blend modes and Clipping Masks on the GPU against values worked out by hand from the W3C
//! compositing definitions on straight colours (D50): every mode over a coloured and a white
//! backdrop, a clipping layer on a translucent base, the base's opacity and mode for the chain,
//! a hidden base, and a clip at the bottom of the stack.

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, CanvasSize, Color, LayerCommand, LayerId, RegionMask,
};

use crate::fixture::{SIZE, blank, cold, core, pixel, plain, sample, with_layer, with_stroke};

/// Inside the bottom layer's rectangle, and outside it (white paper under the top layer).
const INSIDE: [u32; 2] = [60, 100];
const OUTSIDE: [u32; 2] = [260, 100];

/// The backdrop and source colours; the top layer is faded to `TOP_OPACITY`.
const BACK: [f32; 3] = [0.2, 0.7, 0.45];
const SOURCE: [f32; 3] = [0.85, 0.3, 0.6];
const TOP_OPACITY: f32 = 0.7;

fn color([r, g, b]: [f32; 3]) -> Color {
    Color::rgb(r, g, b).expect("colour")
}

/// The pixels with `x` in `x0..x1`, every row.
fn columns(x0: u32, x1: u32) -> RegionMask {
    let inside: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| (x0..x1).contains(&(at % SIZE[0])))
        .collect();
    let canvas = CanvasSize::new(SIZE[0], SIZE[1]).expect("size");
    RegionMask::from_pixels(canvas, &inside).expect("mask")
}

fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    let applied = artwork.apply(ArtworkCommand::Layer(command));
    applied.expect("layer command").artwork
}

fn blend(artwork: &Artwork, id: u64, blend: BlendMode) -> Artwork {
    let layer_id = LayerId::new(id);
    layer(
        artwork,
        LayerCommand::SetBlend {
            layer: layer_id,
            blend,
        },
    )
}

/// The blend function `B(Cb, Cs)` on straight colours.
fn mixed(mode: BlendMode, cb: f64, cs: f64) -> f64 {
    match mode {
        BlendMode::Normal => cs,
        BlendMode::Multiply => cb * cs,
        BlendMode::Screen => cb + cs - cb * cs,
        BlendMode::Overlay if cb <= 0.5 => 2.0 * cb * cs,
        BlendMode::Overlay => 1.0 - 2.0 * (1.0 - cb) * (1.0 - cs),
    }
}

/// A source of straight colour `cs` and alpha `sa` over an opaque backdrop `cb`.
fn over_opaque(mode: BlendMode, cb: [f64; 3], cs: [f64; 3], sa: f64) -> [f64; 3] {
    [0, 1, 2].map(|c| (1.0 - sa) * cb[c] + sa * mixed(mode, cb[c], cs[c]))
}

fn bytes(rgb: [f64; 3]) -> [u8; 4] {
    let [r, g, b] = rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
    [r, g, b, 255]
}

/// Equal within one step of 8-bit rounding.
fn near(actual: [u8; 4], expected: [u8; 4], what: &str) {
    let close = actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1);
    assert!(close, "{what}: {actual:?}, expected {expected:?}");
}

fn wide([r, g, b]: [f32; 3]) -> [f64; 3] {
    [f64::from(r), f64::from(g), f64::from(b)]
}

/// Layer 1: `BACK` on the left half. Layer 2 (on top, `TOP_OPACITY`): `SOURCE` everywhere.
fn two_layers() -> Artwork {
    let bottom = blank().fill_region(columns(0, 160), color(BACK));
    let top = with_layer(&bottom.expect("fill").artwork, TOP_OPACITY);
    let whole = columns(0, SIZE[0]);
    top.fill_region(whole, color(SOURCE)).expect("fill").artwork
}

#[test]
fn each_mode_matches_its_formula_over_colour_and_over_white() {
    let core = core();
    let artwork = two_layers();
    let sa = f64::from(TOP_OPACITY);
    for mode in BlendMode::ALL {
        let image = cold(&core, &blend(&artwork, 2, mode));
        let over_back = over_opaque(mode, wide(BACK), wide(SOURCE), sa);
        near(
            pixel(&image, INSIDE),
            bytes(over_back),
            &format!("{mode:?} over colour"),
        );
        let over_white = over_opaque(mode, [1.0; 3], wide(SOURCE), sa);
        near(
            pixel(&image, OUTSIDE),
            bytes(over_white),
            &format!("{mode:?} over white"),
        );
    }
    // Worked by hand for Multiply over the colour: red 0.3 x 0.2 + 0.7 x 0.2 x 0.85 = 0.179,
    // green 0.3 x 0.7 + 0.7 x 0.7 x 0.3 = 0.357, blue 0.3 x 0.45 + 0.7 x 0.45 x 0.6 = 0.324.
    let multiply = cold(&core, &blend(&artwork, 2, BlendMode::Multiply));
    near(
        pixel(&multiply, INSIDE),
        [46, 91, 83, 255],
        "Multiply by hand",
    );
    // Screen over white is white; Multiply over white is the faded source.
    let screen = cold(&core, &blend(&artwork, 2, BlendMode::Screen));
    assert_eq!(pixel(&screen, OUTSIDE), [255; 4]);
    // Overlay picks Multiply where the backdrop is dark (red 0.2) and Screen where it is light
    // (green 0.7): red 0.3 x 0.2 + 0.7 x 2 x 0.2 x 0.85 = 0.298; green 0.3 x 0.7 + 0.7 x
    // (1 - 2 x 0.3 x 0.7) = 0.616.
    let overlay = cold(&core, &blend(&artwork, 2, BlendMode::Overlay));
    let [r, g, ..] = pixel(&overlay, INSIDE);
    near([r, g, 0, 255], [76, 157, 0, 255], "Overlay by hand");
}

/// Layer 1: a red bar of opacity 0.6 (alpha 0.6 at its middle), 3 is clipping to it: `SOURCE`
/// everywhere at `TOP_OPACITY`.
fn clipped_to_a_bar(clip_mode: BlendMode) -> Artwork {
    let style = plain(40.0, 0.6, Color::rgb(1.0, 0.0, 0.0).expect("red"));
    let bar = vec![sample(20.0, 100.0, 1.0, 0), sample(300.0, 100.0, 1.0, 8)];
    let base = with_stroke(&blank(), (style, bar));
    let top = with_layer(&base, TOP_OPACITY);
    let top = top.fill_region(columns(0, SIZE[0]), color(SOURCE));
    let clipping = LayerCommand::SetClipping {
        layer: LayerId::new(2),
        clipping: true,
    };
    blend(&layer(&top.expect("fill").artwork, clipping), 2, clip_mode)
}

/// The chain of a base of straight `base` colour and alpha `ca` with a clipping source drawn
/// by `mode`, composited by the base's opacity `o` and mode `base_mode` over white.
fn chain_over_white(mode: BlendMode, ca: f64, o: f64, base_mode: BlendMode) -> [f64; 3] {
    let base = [1.0, 0.0, 0.0];
    let sa = f64::from(TOP_OPACITY);
    // Atop the base: its colour by the mode, its alpha kept.
    let chain = over_opaque(mode, base, wide(SOURCE), sa);
    let a = ca * o;
    [0, 1, 2].map(|c| (1.0 - a) * 1.0 + a * mixed(base_mode, 1.0, chain[c]))
}

#[test]
fn a_clipping_layer_shows_only_on_its_base_atop_it_by_its_mode() {
    let core = core();
    for mode in BlendMode::ALL {
        let image = cold(&core, &clipped_to_a_bar(mode));
        let expected = chain_over_white(mode, 0.6, 1.0, BlendMode::Normal);
        near(
            pixel(&image, [160, 100]),
            bytes(expected),
            &format!("{mode:?} on the base"),
        );
        assert_eq!(
            pixel(&image, [160, 20]),
            [255; 4],
            "{mode:?}: nothing off the base"
        );
    }
    // By hand, Normal: the chain is red x 0.3 + SOURCE x 0.7 at alpha 0.6, over white:
    // red 0.4 + 0.6 x (0.3 + 0.595) = 0.937, green 0.4 + 0.6 x 0.21 = 0.526.
    let normal = cold(&core, &clipped_to_a_bar(BlendMode::Normal));
    let [r, g, ..] = pixel(&normal, [160, 100]);
    near([r, g, 0, 255], [239, 134, 0, 255], "Normal atop by hand");
}

#[test]
fn the_bases_opacity_and_mode_carry_the_chain_and_a_hidden_base_hides_it() {
    let core = core();
    let artwork = clipped_to_a_bar(BlendMode::Multiply);
    let faded = layer(
        &artwork,
        LayerCommand::SetOpacity {
            layer: LayerId::new(1),
            opacity: 0.5,
        },
    );
    let image = cold(&core, &blend(&faded, 1, BlendMode::Screen));
    let expected = chain_over_white(BlendMode::Multiply, 0.6, 0.5, BlendMode::Screen);
    near(
        pixel(&image, [160, 100]),
        bytes(expected),
        "the chain by the base",
    );
    let hidden = layer(
        &artwork,
        LayerCommand::SetVisible {
            layer: LayerId::new(1),
            visible: false,
        },
    );
    assert!(
        cold(&core, &hidden).pixels().iter().all(|v| *v == 255),
        "all paper"
    );
    // The clipping layer alone at the bottom is a base itself and shows whole.
    let alone = layer(&artwork, LayerCommand::Delete(LayerId::new(1)));
    let expected = over_opaque(BlendMode::Multiply, [1.0; 3], wide(SOURCE), 0.7);
    near(
        pixel(&cold(&core, &alone), [160, 20]),
        bytes(expected),
        "a bottom clip",
    );
}
