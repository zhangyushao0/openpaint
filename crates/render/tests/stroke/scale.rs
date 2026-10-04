//! Scale-aware evaluation and banded export (S29): at 1x the bands are the canvas bit for bit;
//! bands equal one piece at every scale; a hard stroke edge at 2x and 4x is the analytic edge
//! antialiased over one output pixel; reduced by its scale, every source comes back to its 1x
//! look within a stated tolerance.

use openpaint_core::{
    Adjustment, Artwork, ArtworkCommand, BlendMode, CanvasSize, Color, GroupCommand, Hsb,
    LayerCommand, LayerId, MaskCommand, Patch, Placement, RegionMask, StrokeStyle, StrokeTool,
};
use openpaint_render::{Banding, Core, RgbaImage};

use crate::fixture::{
    SIZE, airbrush, blank, cold, core, diagonal_with, graphite, plain, sample, wave, with_layer,
    with_stroke,
};

/// The whole output of `artwork` at `scale`, cut by `banding`.
pub fn export(core: &Core, artwork: &Artwork, scale: u32, banding: Banding) -> RgbaImage {
    core.export_bands(artwork, Color::WHITE, scale, banding)
        .expect("bands")
        .image()
        .expect("evaluated")
}

/// One piece: every output pixel at once.
pub fn whole(core: &Core, artwork: &Artwork, scale: u32) -> RgbaImage {
    let banding = Banding {
        rows: SIZE[1] * scale,
        columns: SIZE[0] * scale,
    };
    export(core, artwork, scale, banding)
}

/// `image` reduced by `factor` (the mean of each `factor` x `factor` block, rounded), as RGBA.
pub fn reduced(image: &RgbaImage, factor: u32) -> Vec<u8> {
    let [width, height] = image.size();
    let (w, h) = (width / factor, height / factor);
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            for channel in 0..4 {
                let mut sum = 0u32;
                for dy in 0..factor {
                    for dx in 0..factor {
                        let at =
                            (((y * factor + dy) * width + x * factor + dx) * 4 + channel) as usize;
                        sum += u32::from(image.pixels()[at]);
                    }
                }
                let count = factor * factor;
                out.push(((sum + count / 2) / count) as u8);
            }
        }
    }
    out
}

/// Largest and mean channel difference.
pub fn difference(a: &[u8], b: &[u8]) -> (u8, f64) {
    assert_eq!(a.len(), b.len());
    let mut largest = 0;
    let mut total = 0u64;
    for (x, y) in a.iter().zip(b) {
        let d = x.abs_diff(*y);
        largest = largest.max(d);
        total += u64::from(d);
    }
    (largest, total as f64 / a.len().max(1) as f64)
}

fn layer(id: u64) -> LayerId {
    LayerId::new(id)
}

fn apply(artwork: &Artwork, command: ArtworkCommand) -> Artwork {
    artwork.apply(command).expect("command").artwork
}

fn canvas() -> CanvasSize {
    CanvasSize::new(SIZE[0], SIZE[1]).expect("size")
}

fn region(inside: impl Fn(u32, u32) -> bool) -> RegionMask {
    let pixels: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| inside(at % SIZE[0], at / SIZE[0]))
        .collect();
    RegionMask::from_pixels(canvas(), &pixels).expect("mask")
}

fn color(r: f32, g: f32, b: f32) -> Color {
    Color::rgb(r, g, b).expect("colour")
}

/// A 64 x 48 picture: an opaque gradient with a half transparent stripe.
fn picture() -> Patch {
    let mut pixels = Vec::new();
    for y in 0..48u32 {
        for x in 0..64u32 {
            let pixel = if (20..28).contains(&y) {
                [60, 20, 10, 128]
            } else {
                [(x * 4) as u8, (y * 5) as u8, 150, 255]
            };
            pixels.extend(pixel);
        }
    }
    Patch::new(canvas(), [40, 120], [64, 48], pixels).expect("patch")
}

/// The pencil and the ink wave on layer 1, a fill and a smudge across them.
fn painted() -> Artwork {
    let pencil = with_stroke(&with_stroke(&blank(), wave()), diagonal_with(graphite()));
    let disc = region(|x, y| {
        let (dx, dy) = (x as f32 - 230.0, y as f32 - 60.0);
        dx * dx + dy * dy <= 900.0
    });
    let filled = pencil
        .fill_region(disc, color(0.9, 0.3, 0.1))
        .expect("fill")
        .artwork;
    let smudge = StrokeStyle::new(airbrush(), 30.0, 0.8, Color::BLACK)
        .expect("style")
        .with_tool(StrokeTool::Smudge);
    let path = (0..24u16)
        .map(|i| {
            sample(
                150.0 + 4.0 * f32::from(i),
                40.0 + 5.0 * f32::from(i),
                0.9,
                u32::from(i) * 8,
            )
        })
        .collect();
    with_stroke(&filled, (smudge, path))
}

/// Every source at work: `painted` in a faded group; a turned picture on layer 2 with a grey
/// mask; layer 3 clipping to it by Multiply; an adjustment inside a selection on layer 2.
pub fn every_source() -> Artwork {
    let grouped = apply(
        &painted(),
        ArtworkCommand::Group(GroupCommand::Create { layer: layer(1) }),
    );
    let group = grouped.groups()[0].id();
    let fade = GroupCommand::SetOpacity {
        group,
        opacity: 0.8,
    };
    let faded = apply(&grouped, ArtworkCommand::Group(fade));
    let created = apply(&faded, ArtworkCommand::Layer(LayerCommand::Create));
    let turn = Placement::around([72.0, 144.0], 1.3, 0.4, [false, false]).expect("turn");
    let placed = created
        .transform_region(None, picture(), turn)
        .expect("placed")
        .artwork;
    let picture_layer = placed.active();
    let masked = apply(
        &placed,
        ArtworkCommand::Mask(MaskCommand::Add(picture_layer)),
    );
    let grey = masked
        .fill_region(region(|x, _| x < 70), color(0.4, 0.4, 0.4))
        .expect("grey")
        .artwork;
    let paint = apply(
        &grey,
        ArtworkCommand::Layer(LayerCommand::Select(picture_layer)),
    );
    let selection = region(|x, y| x > 60 && y > 130);
    let adjusted = paint
        .adjust_region(Some(selection), Adjustment::Hsb(Hsb::new(0.7, 0.8, 0.55)))
        .expect("adjusted")
        .artwork;
    let top = with_layer(&adjusted, 0.9);
    let top_layer = top.active();
    let clip = LayerCommand::SetClipping {
        layer: top_layer,
        clipping: true,
    };
    let multiply = LayerCommand::SetBlend {
        layer: top_layer,
        blend: BlendMode::Multiply,
    };
    let chained = apply(
        &apply(&top, ArtworkCommand::Layer(clip)),
        ArtworkCommand::Layer(multiply),
    );
    with_stroke(
        &chained,
        (
            plain(30.0, 1.0, color(0.2, 0.6, 0.9)),
            vec![sample(20.0, 150.0, 1.0, 0), sample(200.0, 170.0, 1.0, 8)],
        ),
    )
}

#[test]
fn at_one_x_the_bands_are_the_canvas_bit_for_bit() {
    let core = core();
    let artwork = every_source();
    let canvas = cold(&core, &artwork);
    let banded = export(
        &core,
        &artwork,
        1,
        Banding {
            rows: 37,
            columns: 100,
        },
    );
    assert_eq!(banded, canvas, "1x bands = the canvas");
    assert_eq!(whole(&core, &artwork, 1), canvas);
}

#[test]
fn bands_equal_one_piece_at_two_and_four_x() {
    let core = core();
    let artwork = every_source();
    for scale in [2, 4] {
        let banding = Banding {
            rows: 37 * scale,
            columns: 61 * scale + 5,
        };
        let banded = export(&core, &artwork, scale, banding);
        assert_eq!(banded.size(), [SIZE[0] * scale, SIZE[1] * scale]);
        assert_eq!(banded, whole(&core, &artwork, scale), "{scale}x");
    }
    let refused = core.export_bands(&artwork, Color::WHITE, 3, Banding::DEFAULT);
    assert!(refused.is_err(), "3x is no scale");
}

/// Coverage of a hard edge for an output pixel whose centre lies `outside` canvas pixels out
/// of the shape, at `scale`: one output pixel of box-filtered antialiasing.
fn analytic(outside: f32, scale: f32) -> f32 {
    (0.5 - outside * scale).clamp(0.0, 1.0)
}

#[test]
fn a_hard_edge_at_two_and_four_x_is_one_output_pixel_wide() {
    let core = core();
    // A black bar of radius 10 along y = 100.3: its top edge at y = 90.3.
    let bar = (
        plain(20.0, 1.0, Color::BLACK),
        vec![sample(40.0, 100.3, 1.0, 0), sample(280.0, 100.3, 1.0, 8)],
    );
    let artwork = with_stroke(&blank(), bar);
    for scale in [1u32, 2, 4] {
        let image = whole(&core, &artwork, scale);
        let width = image.size()[0];
        let x = 160 * scale;
        let mut partial = 0;
        for y in 85 * scale..96 * scale {
            let centre = (y as f32 + 0.5) / scale as f32;
            let coverage = analytic(90.3 - centre, scale as f32);
            let expected = (255.0 * (1.0 - coverage)).round() as u8;
            let at = ((y * width + x) * 4) as usize;
            let actual = image.pixels()[at];
            assert!(
                actual.abs_diff(expected) <= 1,
                "{scale}x row {y}: {actual} for {expected}"
            );
            partial += usize::from(actual > 0 && actual < 255);
        }
        assert!(partial <= 1, "{scale}x: {partial} pixels across the edge");
    }
}

#[test]
fn reduced_by_its_scale_ink_comes_back_to_its_one_x_look() {
    let core = core();
    let artwork = with_stroke(&blank(), wave());
    let one = cold(&core, &artwork);
    for scale in [2, 4] {
        let (largest, mean) = difference(
            &reduced(&whole(&core, &artwork, scale), scale),
            one.pixels(),
        );
        eprintln!("ink {scale}x reduced against 1x: largest {largest}, mean {mean:.4}");
        // Measured 2026-10-04: largest 12 and 13, mean 0.030 and 0.031 (edge pixels only).
        assert!(
            mean < 0.1 && largest <= 24,
            "{scale}x: largest {largest}, mean {mean}"
        );
    }
}

#[test]
fn pencil_grain_at_two_and_four_x_keeps_its_tone_without_shimmer() {
    let core = core();
    let artwork = with_stroke(&blank(), diagonal_with(graphite()));
    let one = cold(&core, &artwork);
    let two = reduced(&whole(&core, &artwork, 2), 2);
    let four = reduced(&whole(&core, &artwork, 4), 4);
    let (largest, mean) = difference(&two, one.pixels());
    let (largest_four, mean_four) = difference(&four, one.pixels());
    let (between, between_mean) = difference(&four, &two);
    eprintln!(
        "pencil reduced: 2x against 1x largest {largest} mean {mean:.4}; 4x against 1x largest \
         {largest_four} mean {mean_four:.4}; 4x against 2x largest {between} mean {between_mean:.4}"
    );
    // Measured 2026-10-04: 2x largest 19 mean 0.095, 4x largest 21 mean 0.125, 4x against 2x
    // largest 13 mean 0.059. Finer grain shows at higher scales; its mean tone stays.
    assert!(mean < 0.5 && mean_four < 0.5, "the tone stays");
    assert!(largest <= 40 && largest_four <= 40, "no speck turns over");
    assert!(between_mean < 0.25, "4x and 2x agree: no shimmer");
}

#[test]
fn every_raster_source_at_two_x_comes_back_to_its_one_x_look() {
    let core = core();
    let artwork = every_source();
    let one = cold(&core, &artwork);
    let two = whole(&core, &artwork, 2);
    let (largest, mean) = difference(&reduced(&two, 2), one.pixels());
    eprintln!("every source 2x reduced against 1x: largest {largest}, mean {mean:.4}");
    // Measured 2026-10-04: largest 18, mean 0.107 (stroke and patch edges, the bilinear smudge).
    assert!(
        mean < 0.5 && largest <= 48,
        "largest {largest}, mean {mean}"
    );
    // A fill's inside is nearest at every scale: exactly its colour on all four pixels.
    let at = |image: &RgbaImage, [x, y]: [u32; 2]| {
        let i = ((y * image.size()[0] + x) * 4) as usize;
        image.pixels()[i..i + 4].to_vec()
    };
    let inside = at(&one, [230, 60]);
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(at(&two, [460 + dx, 120 + dy]), inside, "the fill's colour");
    }
}
