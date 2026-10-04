//! Layer masks on the GPU (S24): a mask multiplies its layer's paint before the layer's
//! opacity, mode, clipping and group; a base's mask shapes its chain as its alpha does, a
//! clipping layer's mask only itself; disabled is no mask, bit for bit; inverted shows black for
//! white; the eraser paints white; incremental equals cold through every mask edit and while
//! painting or smudging on a mask. Values are worked by hand on straight sRGB values.

use openpaint_core::{
    Artwork, ArtworkCommand, CanvasSize, Color, GroupCommand, GroupId, LayerCommand, LayerId,
    MaskCommand, RegionMask, StrokeTool,
};
use openpaint_render::{CanvasTarget, FrameRequest, PixelRect, RegionEvaluator, changed_regions};

use crate::fixture::{
    SIZE, blank, cold, core, diagonal, pixel, plain, request, sample, wave, with_layer, with_stroke,
};

const BACK: [f32; 3] = [0.2, 0.7, 0.45];
const SOURCE: [f32; 3] = [0.85, 0.3, 0.6];
/// A grey the 8-bit palette holds exactly: 102 / 255.
const GREY: f32 = 0.4;

fn color([r, g, b]: [f32; 3]) -> Color {
    Color::rgb(r, g, b).expect("colour")
}

fn columns(x0: u32, x1: u32) -> RegionMask {
    let inside: Vec<bool> = (0..SIZE[0] * SIZE[1])
        .map(|at| (x0..x1).contains(&(at % SIZE[0])))
        .collect();
    let canvas = CanvasSize::new(SIZE[0], SIZE[1]).expect("size");
    RegionMask::from_pixels(canvas, &inside).expect("mask")
}

fn apply(artwork: &Artwork, command: ArtworkCommand) -> Artwork {
    artwork.apply(command).expect("command").artwork
}

fn mask(artwork: &Artwork, command: MaskCommand) -> Artwork {
    apply(artwork, ArtworkCommand::Mask(command))
}

/// Fills `region` of the target (the active layer's paint or its mask) with `rgb`.
fn fill(artwork: &Artwork, region: RegionMask, rgb: [f32; 3]) -> Artwork {
    artwork
        .fill_region(region, color(rgb))
        .expect("fill")
        .artwork
}

fn layer(id: u64) -> LayerId {
    LayerId::new(id)
}

fn paint_of(artwork: &Artwork, id: u64) -> Artwork {
    apply(
        artwork,
        ArtworkCommand::Layer(LayerCommand::Select(layer(id))),
    )
}

fn near(actual: [u8; 4], expected: [f64; 3], what: &str) {
    let expected = expected.map(|v| (v * 255.0).round() as u8);
    let close = actual[..3]
        .iter()
        .zip(expected)
        .all(|(a, e)| a.abs_diff(e) <= 1);
    assert!(close, "{what}: {actual:?}, expected {expected:?}");
}

fn each(f: impl Fn(f64) -> f64, rgb: [f32; 3]) -> [f64; 3] {
    rgb.map(|c| f(f64::from(c)))
}

/// One layer of `SOURCE` everywhere at half opacity, its mask grey on the left half.
fn faded_masked() -> (Artwork, Artwork) {
    let painted = fill(&blank(), columns(0, SIZE[0]), SOURCE);
    let half = LayerCommand::SetOpacity {
        layer: layer(1),
        opacity: 0.5,
    };
    let unmasked = apply(&painted, ArtworkCommand::Layer(half));
    let added = mask(&unmasked, MaskCommand::Add(layer(1)));
    let masked = fill(&added, columns(0, 160), [GREY; 3]);
    (unmasked, masked)
}

#[test]
fn a_mask_multiplies_the_paint_before_the_opacity_and_white_changes_nothing() {
    let core = core();
    let (unmasked, masked) = faded_masked();
    let white = mask(&unmasked, MaskCommand::Add(layer(1)));
    let plain_image = cold(&core, &unmasked);
    assert!(
        cold(&core, &white) == plain_image,
        "a white mask, bit for bit"
    );
    let image = cold(&core, &masked);
    // Left: 0.4 of the paint, then half of that over white: 0.2 C + 0.8 (red 0.97).
    let left = each(|c| 0.2 * c + 0.8, SOURCE);
    near(pixel(&image, [60, 100]), left, "grey mask x opacity");
    assert_eq!((left[0] * 255.0).round(), 247.0, "by hand");
    let right = each(|c| 0.5 * c + 0.5, SOURCE);
    near(pixel(&image, [300, 100]), right, "white mask");
    assert_eq!(pixel(&image, [300, 100]), pixel(&plain_image, [300, 100]));
}

#[test]
fn disabled_is_no_mask_and_inverted_shows_black_for_white() {
    let core = core();
    let (unmasked, masked) = faded_masked();
    let off = MaskCommand::SetEnabled {
        layer: layer(1),
        enabled: false,
    };
    let disabled = mask(&masked, off);
    assert!(
        cold(&core, &disabled) == cold(&core, &unmasked),
        "bit for bit"
    );
    let inverted = cold(&core, &mask(&masked, MaskCommand::Invert(layer(1))));
    // Left: 1 - 0.4 of the paint at half opacity; right: the white mask hides everything.
    let left = each(|c| 0.3 * c + 0.7, SOURCE);
    near(pixel(&inverted, [60, 100]), left, "inverted grey");
    assert_eq!(
        pixel(&inverted, [300, 100]),
        [255; 4],
        "inverted white hides"
    );
    let cleared = mask(&masked, MaskCommand::Clear(layer(1)));
    assert!(
        cold(&core, &cleared) == cold(&core, &unmasked),
        "cleared is white"
    );
    let deleted = mask(&masked, MaskCommand::Delete(layer(1)));
    assert!(cold(&core, &deleted) == cold(&core, &unmasked), "deleted");
}

#[test]
fn the_eraser_paints_white_on_a_mask_and_a_black_mask_hides_its_layer() {
    let core = core();
    let painted = fill(&blank(), columns(0, SIZE[0]), SOURCE);
    let black = fill(
        &mask(&painted, MaskCommand::Add(layer(1))),
        columns(0, SIZE[0]),
        [0.0; 3],
    );
    let hidden = cold(&core, &black);
    assert_eq!(pixel(&hidden, [160, 100]), [255; 4], "black hides");
    let eraser = plain(30.0, 1.0, Color::BLACK).with_tool(StrokeTool::Erase);
    let line = (0..20u16).map(|i| sample(40.0 + 12.0 * f32::from(i), 100.0, 1.0, u32::from(i) * 8));
    let erased = with_stroke(&black, (eraser, line.collect()));
    let shown = cold(&core, &erased);
    let full = cold(&core, &painted);
    assert_eq!(
        pixel(&shown, [160, 100]),
        pixel(&full, [160, 100]),
        "white again"
    );
    assert_eq!(pixel(&shown, [160, 40]), [255; 4], "away from the stroke");
}

/// Layer 1: `BACK` everywhere, its mask grey on columns 0..100; layer 2: `SOURCE` on the left
/// half, clipping to layer 1.
fn chain() -> Artwork {
    let base = fill(&blank(), columns(0, SIZE[0]), BACK);
    let base = fill(
        &mask(&base, MaskCommand::Add(layer(1))),
        columns(0, 100),
        [GREY; 3],
    );
    let top = fill(&with_layer(&base, 1.0), columns(0, 160), SOURCE);
    let clip = LayerCommand::SetClipping {
        layer: layer(2),
        clipping: true,
    };
    apply(&top, ArtworkCommand::Layer(clip))
}

#[test]
fn a_bases_mask_shapes_its_chain_and_a_clipping_layers_mask_only_itself() {
    let core = core();
    let artwork = chain();
    let image = cold(&core, &artwork);
    // The base shows 0.4 at x 50, so its chain does: the clipping layer atop it, 0.4 of it
    // over white.
    let thin = each(|c| 0.4 * c + 0.6, SOURCE);
    near(pixel(&image, [50, 100]), thin, "through the base's grey");
    near(
        pixel(&image, [130, 100]),
        each(|c| c, SOURCE),
        "on the whole base",
    );
    near(
        pixel(&image, [200, 100]),
        each(|c| c, BACK),
        "the base alone",
    );
    let own = mask(&artwork, MaskCommand::Add(layer(2)));
    let own = fill(&own, columns(120, 160), [0.0; 3]);
    let image = cold(&core, &own);
    near(
        pixel(&image, [130, 100]),
        each(|c| c, BACK),
        "the clip hidden, the base not",
    );
    near(pixel(&image, [50, 100]), thin, "the base's mask unchanged");
}

#[test]
fn a_mask_inside_a_faded_group_applies_before_the_group() {
    let core = core();
    let (unmasked, masked) = faded_masked();
    let full = LayerCommand::SetOpacity {
        layer: layer(1),
        opacity: 1.0,
    };
    let masked = apply(&masked, ArtworkCommand::Layer(full));
    let grouped = apply(
        &masked,
        ArtworkCommand::Group(GroupCommand::Create { layer: layer(1) }),
    );
    let group = GroupId::new(grouped.next_layer_id() - 1);
    let fade = GroupCommand::SetOpacity {
        group,
        opacity: 0.5,
    };
    let faded = apply(&grouped, ArtworkCommand::Group(fade));
    let image = cold(&core, &faded);
    // 0.4 of the layer over white, then half of that over white: 0.2 C + 0.8.
    near(
        pixel(&image, [60, 100]),
        each(|c| 0.2 * c + 0.8, SOURCE),
        "mask, then group",
    );
    near(
        pixel(&image, [300, 100]),
        each(|c| 0.5 * c + 0.5, SOURCE),
        "white mask",
    );
    drop(unmasked);
}

/// Versions through every mask edit, each from the one before: strokes and fills on the paint
/// and the mask, invert, disable, enable, clear, delete.
fn edits() -> Vec<Artwork> {
    let first = with_stroke(&with_stroke(&chain(), wave()), diagonal());
    let steps: [&dyn Fn(&Artwork) -> Artwork; 10] = [
        &|a| mask(a, MaskCommand::Add(layer(2))),
        &|a| with_stroke(a, wave()),
        &|a| fill(a, columns(10, 40), [0.7; 3]),
        &|a| mask(a, MaskCommand::Invert(layer(2))),
        &|a| with_stroke(a, diagonal()),
        &|a| {
            mask(
                a,
                MaskCommand::SetEnabled {
                    layer: layer(1),
                    enabled: false,
                },
            )
        },
        &|a| {
            mask(
                a,
                MaskCommand::SetEnabled {
                    layer: layer(1),
                    enabled: true,
                },
            )
        },
        &|a| mask(a, MaskCommand::Clear(layer(1))),
        &|a| with_stroke(&paint_of(a, 2), wave()),
        &|a| mask(a, MaskCommand::Delete(layer(2))),
    ];
    let mut versions = vec![first];
    for step in steps {
        let next = step(versions.last().expect("version"));
        versions.push(next);
    }
    versions
}

#[test]
fn mask_edits_through_changed_regions_equal_cold() {
    let core = core();
    let versions = edits();
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    let mut shown = versions[0].clone();
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&shown), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let order = (1..versions.len()).chain((0..versions.len() - 1).rev());
    for index in order {
        let next = &versions[index];
        let regions = changed_regions(&shown, next);
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(next), &regions)
            .expect("evaluate");
        let image = target.read(&core).expect("readback");
        assert!(image == cold(&core, next), "version {index}");
        shown = next.clone();
    }
}

#[test]
fn painting_and_smudging_on_a_mask_equal_cold_every_frame_and_once_committed() {
    let core = core();
    let artwork = mask(
        &with_stroke(&chain(), wave()),
        MaskCommand::Select(layer(1)),
    );
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let mut current = artwork;
    for tool in [StrokeTool::Paint, StrokeTool::Smudge, StrokeTool::Erase] {
        let (style, samples) = diagonal();
        let style = style.with_tool(tool);
        let mut session = current.begin_stroke(style, samples[0]).expect("begin");
        assert!(session.active().on_mask());
        for chunk in samples[1..].chunks(6) {
            session.extend(chunk).expect("extend");
            let frame = FrameRequest {
                active: Some(session.active()),
                ..request(&current)
            };
            RegionEvaluator::new(&core, &mut target)
                .evaluate(&frame, &[])
                .expect("evaluate");
            let mut fresh = CanvasTarget::new(&core, SIZE).expect("target");
            RegionEvaluator::new(&core, &mut fresh)
                .evaluate(&frame, &[PixelRect::full(SIZE)])
                .expect("evaluate");
            let shown = target.read(&core).expect("readback");
            assert!(
                shown == fresh.read(&core).expect("readback"),
                "{tool:?} in progress"
            );
        }
        current = session.finish().artwork;
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(&current), &[])
            .expect("evaluate");
        let shown = target.read(&core).expect("readback");
        assert!(shown == cold(&core, &current), "{tool:?} committed = cold");
    }
    let reloaded = Artwork::from_storage(current.to_parts()).expect("reload");
    assert!(cold(&core, &reloaded) == cold(&core, &current), "reopened");
}

#[test]
fn a_masks_pixels_are_its_grey_on_white() {
    let core = core();
    let (_, masked) = faded_masked();
    let pixels = core.mask_pixels(&masked, layer(1)).expect("mask pixels");
    assert_eq!(pixel(&pixels, [60, 100]), [102, 102, 102, 255]);
    assert_eq!(pixel(&pixels, [300, 100]), [255; 4]);
    let paint = core.layer_pixels(&masked, layer(1)).expect("layer pixels");
    let source = SOURCE.map(|c| (c * 255.0).round() as u8);
    assert_eq!(
        pixel(&paint, [60, 100])[..3],
        source,
        "the paint without its mask"
    );
}
