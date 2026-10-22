//! The kinds of record the raster core draws, each alone and nested: point
//! adjustments (Hue, Saturation, Brightness; Curves with a channel inside a lasso; Curves on a
//! mask), filters (a Gaussian Blur, a Sharpen inside a selection), a layer's transform record, a
//! moved selection (over a smudge, whose material then lies under the record), a merged layer, a
//! faded group by Multiply, a filter under a record, a record and a merge inside a faded group,
//! an adjustment over a merged layer's moved selection. Each exports at 1x, 2x, 4x and 8x as one
//! picture (each scale reduced comes back to the canvas); draws alike every time, in bands as in
//! one piece, in scattered regions as whole; its tier tiles are its export;
//! changes over it (a stroke, a fill, a hidden layer) and their undo end as drawn cold; a stroke,
//! a smudge and a clipped stroke drawn inside it are cold every frame and at their lift.

use openpaint_core::{
    Adjustment, Artwork, ArtworkCommand, BlendMode, Blur, Color, Curves, GroupCommand, Hsb,
    LayerCommand, MaskCommand, RegionMask, Sharpen, StrokeSample, StrokeStyle, StrokeTool,
    ToneChannel, ToneCurve,
};
use openpaint_render::{Banding, CanvasTarget, PixelRect, changed_regions};

use crate::drawing::{CHUNKS, begun, committed, path, screen, shows};
use crate::fixture::{
    SIZE, apply, cold, core, diagonal, layer, plain, region, request, wave, with_layer, with_stroke,
};
use crate::raster::{assert_scales_agree, color};
use crate::records::{busy, by_record, lasso, turned};
use crate::scale::{export, whole};
use crate::tiers::tiles_are_the_export;

fn adjusted(artwork: &Artwork, clip: Option<RegionMask>, adjustment: Adjustment) -> Artwork {
    (artwork.adjust_region(clip, adjustment))
        .expect("adjusted")
        .artwork
}

fn curves() -> Adjustment {
    let curve = |points: &[[f32; 2]]| ToneCurve::new(points).expect("curve");
    let curves = (Curves::default())
        .with(
            ToneChannel::Composite,
            curve(&[[0.0, 0.0], [0.5, 0.65], [1.0, 1.0]]),
        )
        .with(ToneChannel::Red, curve(&[[0.0, 0.08], [1.0, 0.9]]));
    Adjustment::Curves(curves)
}

fn hsb() -> Adjustment {
    Adjustment::Hsb(Hsb::new(0.6, 0.7, 0.45))
}

/// The busy layer and a translucent layer above it holding the diagonal, active.
fn two() -> Artwork {
    with_stroke(&with_layer(&busy(), 0.8), diagonal())
}

/// `artwork`'s active layer merged down.
fn merged(artwork: &Artwork) -> Artwork {
    layer(artwork, LayerCommand::MergeDown(artwork.active()))
}

/// `artwork`'s active layer by Multiply grouped with the one under it, the group at `opacity`.
fn grouped(artwork: &Artwork, opacity: f32) -> Artwork {
    let top = artwork.active();
    let blend = LayerCommand::SetBlend {
        layer: top,
        blend: BlendMode::Multiply,
    };
    let artwork = layer(artwork, blend);
    let artwork = apply(
        &artwork,
        ArtworkCommand::Group(GroupCommand::Create { layer: top }),
    );
    let group = (artwork.layer(top).and_then(|layer| layer.props().group())).expect("grouped");
    let faded = GroupCommand::SetOpacity { group, opacity };
    apply(&artwork, ArtworkCommand::Group(faded))
}

/// `two` with a mask on its top layer: grey on the left, Curves over it.
fn adjusted_mask() -> Artwork {
    let artwork = two();
    let masked = apply(
        &artwork,
        ArtworkCommand::Mask(MaskCommand::Add(artwork.active())),
    );
    let grey = masked.fill_region(region(|x, _| x < 150), color(0.45, 0.45, 0.45));
    adjusted(&grey.expect("grey").artwork, None, curves())
}

/// Every kind and nesting, named; the layer each one's content lies on active.
pub fn kinds() -> Vec<(&'static str, Artwork)> {
    let base = busy();
    let selection = || region(|x, y| x > 90 && x < 260 && y < 130);
    let blur = |radius| Adjustment::Blur(Blur::new(radius));
    let in_group = merged(&with_stroke(
        &with_layer(&by_record(&two(), Some(&lasso()), turned()), 0.9),
        wave(),
    ));
    vec![
        ("hsb", adjusted(&base, None, hsb())),
        (
            "curves in a lasso",
            adjusted(&base, Some(lasso()), curves()),
        ),
        ("curves on a mask", adjusted_mask()),
        ("blur", adjusted(&base, None, blur(6.0))),
        (
            "sharpen in a selection",
            adjusted(
                &base,
                Some(selection()),
                Adjustment::Sharpen(Sharpen::new(0.7)),
            ),
        ),
        ("a layer record", by_record(&base, None, turned())),
        (
            "a moved selection",
            by_record(&base, Some(&lasso()), turned()),
        ),
        ("a merged layer", merged(&two())),
        ("a faded group", grouped(&two(), 0.7)),
        (
            "a filter under a record",
            by_record(&adjusted(&base, None, blur(4.0)), None, turned()),
        ),
        (
            "a record and a merge in a faded group",
            grouped(&in_group, 0.6),
        ),
        ("an adjustment over a merged layer's moved selection", {
            let moved = by_record(&merged(&two()), Some(&lasso()), turned());
            adjusted(&moved, None, hsb())
        }),
    ]
}

#[test]
fn every_kind_at_every_scale_comes_back_to_its_canvas() {
    let core = core();
    for (name, artwork) in kinds() {
        assert_scales_agree(&core, &artwork, name);
    }
}

#[test]
fn every_kind_draws_alike_in_bands_as_in_one_piece_and_in_regions_as_whole() {
    let core = core();
    let parts = [
        PixelRect {
            min: [0, 0],
            max: [97, 200],
        },
        PixelRect {
            min: [97, 0],
            max: [320, 61],
        },
        PixelRect {
            min: [97, 61],
            max: [320, 200],
        },
    ];
    for (name, artwork) in kinds() {
        let canvas = cold(&core, &artwork);
        assert!(canvas == cold(&core, &artwork), "{name}: alike every time");
        let mut target = CanvasTarget::new(&core, SIZE).expect("target");
        for part in parts {
            target
                .draw(&core, &request(&artwork), &[part])
                .expect("a part");
        }
        assert!(
            target.read(&core).expect("read") == canvas,
            "{name}: regions = whole"
        );
        for scale in [1, 2, 4] {
            let one = whole(&core, &artwork, scale);
            if scale == 1 {
                assert!(one == canvas, "{name}: the canvas is the 1x export");
            }
            let banding = Banding {
                rows: 37 * scale,
                columns: 61 * scale + 5,
            };
            let bands = export(&core, &artwork, scale, banding);
            assert!(bands == one, "{name} at {scale}x: bands = one piece");
        }
    }
}

#[test]
fn every_kinds_tier_tiles_are_its_export() {
    let core = core();
    for (_, artwork) in kinds() {
        tiles_are_the_export(&core, &artwork);
    }
}

/// A rectangle filled over `artwork`'s active layer.
fn filled(artwork: &Artwork) -> Artwork {
    let square = region(|x, y| (130..190).contains(&x) && (70..120).contains(&y));
    (artwork.fill_region(square, color(0.2, 0.7, 0.3)))
        .expect("fill")
        .artwork
}

#[test]
fn changes_over_every_kind_and_their_undo_through_changed_regions_are_cold() {
    let core = core();
    for (name, artwork) in kinds() {
        let hide = LayerCommand::SetVisible {
            layer: artwork.active(),
            visible: false,
        };
        let steps = [
            with_stroke(&artwork, diagonal()),
            artwork.clone(),
            filled(&artwork),
            artwork.clone(),
            layer(&artwork, hide),
            artwork.clone(),
        ];
        let mut target = CanvasTarget::new(&core, SIZE).expect("target");
        target
            .draw(&core, &request(&artwork), &[PixelRect::full(SIZE)])
            .expect("first");
        let mut shown = artwork.clone();
        for (index, next) in steps.iter().enumerate() {
            target
                .draw(&core, &request(next), &changed_regions(&shown, next))
                .expect("step");
            let incremental = target.read(&core).expect("read");
            assert!(incremental == cold(&core, next), "{name}: step {index}");
            shown = next.clone();
        }
    }
}

/// A stroke of `style` through `samples` (inside `clip`) drawn in batches over `base` at `zoom`
/// about its middle: the canvas (and at a zoom, the tier's ready tiles at `scale`) the stroke so
/// far committed drawn cold after every batch, then the commit cold after the lift.
fn drawn_inside(
    base: &Artwork,
    (style, samples, clip): (&StrokeStyle, &[StrokeSample], Option<&RegionMask>),
    (zoom, scale): (f32, Option<u32>),
    name: &str,
) {
    let core = core();
    let middle = samples[samples.len() / 2].position();
    let (mut screen, view) = screen(base, (zoom, [middle.x(), middle.y()]));
    let mut session = begun(base, (style, &samples[..1]), clip);
    let mut end = 1;
    for size in CHUNKS.iter().cycle() {
        screen.frame(&core, (base, Some(session.active())), &[], &view);
        let what = format!("{name} at {zoom}x after {end} samples");
        if style.tool() == StrokeTool::Smudge {
            // Its contacts trail the pen: the same frame on a canvas drawn until then.
            let (mut once, _) = self::screen(base, (zoom, [middle.x(), middle.y()]));
            once.frame(&core, (base, Some(session.active())), &[], &view);
            let read = |target: &CanvasTarget| target.read(&core).expect("read");
            assert!(read(&screen.target) == read(&once.target), "{what}");
        } else {
            let so_far = committed(base, (style, &samples[..end]), clip);
            shows(&screen, &so_far, scale, &what);
        }
        if end == samples.len() {
            break;
        }
        let next = (end + size).min(samples.len());
        session.extend(&samples[end..next]).expect("extend");
        end = next;
    }
    let lifted = session.finish().artwork;
    screen.frame(&core, (&lifted, None), &[], &view);
    shows(&screen, &lifted, scale, &format!("{name} lifted"));
}

#[test]
fn a_stroke_a_smudge_and_a_clipped_stroke_drawn_inside_every_kind_are_cold() {
    let through = path([160.0, 100.0], 110.0, 30);
    let paint = plain(22.0, 0.7, color(0.1, 0.5, 0.8));
    let smudge = plain(26.0, 0.8, Color::BLACK).with_tool(StrokeTool::Smudge);
    let clip = region(|x, y| (x as i32 - 150).abs() + (y as i32 - 100).abs() < 60);
    for (name, artwork) in kinds() {
        drawn_inside(&artwork, (&paint, &through, None), (1.0, None), name);
        drawn_inside(&artwork, (&smudge, &through, None), (1.0, None), name);
        drawn_inside(
            &artwork,
            (&paint, &through, Some(&clip)),
            (3.7, Some(4)),
            name,
        );
    }
}
