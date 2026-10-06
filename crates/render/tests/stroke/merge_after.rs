//! After Merge Down on the GPU (S39, D66): strokes, adjustments, filters, smudges and transform
//! records on the merged layer act on the whole merged content (equal to one layer that held it
//! all, its pixels alone too); incremental equals cold through the merge, drawing on the merged
//! layer and undo; a merged layer exports at 2x and 4x as the two layers did.

use openpaint_core::{Adjustment, Artwork, BlendMode, Blur, Color, Hsb, Placement, StrokeTool};
use openpaint_render::{
    Banding, CanvasTarget, FrameRequest, PixelRect, RegionEvaluator, RgbaImage, changed_regions,
};

use crate::fixture::{
    SIZE, blank, cold, core, diagonal, plain, request, sample, wave, with_layer, with_stroke,
};
use crate::merge::{Look, PLAIN, SOURCE, alone, color, merged};
use crate::scale::export;

/// An edit after the merge, by name.
type Step<'a> = (&'a str, &'a dyn Fn(&Artwork) -> Artwork);

/// One layer holding the wave and then the diagonal, and the same strokes on two layers merged:
/// what each shows after `then`, and its layer's pixels alone.
fn both_ways(then: &dyn Fn(&Artwork) -> Artwork) -> [(RgbaImage, RgbaImage); 2] {
    let core = core();
    let one = with_stroke(&with_stroke(&blank(), wave()), diagonal());
    let two = with_stroke(&with_layer(&with_stroke(&blank(), wave()), 1.0), diagonal());
    [then(&one), then(&merged(&two))].map(|artwork| {
        let id = artwork.active();
        let alone = core.layer_pixels(&artwork, id).expect("layer pixels");
        (cold(&core, &artwork), alone)
    })
}

#[test]
fn everything_after_a_merge_acts_on_the_whole_merged_content() {
    let stroke = |a: &Artwork| {
        let style = plain(18.0, 0.9, color(SOURCE));
        let line = (0..30u16).map(|i| {
            let t = f32::from(i);
            sample(20.0 + 9.0 * t, 60.0 + 3.0 * t, 0.8, u32::from(i) * 8)
        });
        with_stroke(a, (style, line.collect()))
    };
    let hue = |a: &Artwork| {
        let shift = Adjustment::Hsb(Hsb {
            hue: 0.8,
            ..Hsb::NONE
        });
        a.adjust_region(None, shift).expect("adjust").artwork
    };
    let blur = |a: &Artwork| {
        let blur = Adjustment::Blur(Blur { radius: 2.5 });
        a.adjust_region(None, blur).expect("blur").artwork
    };
    let smudge = |a: &Artwork| {
        let style = plain(30.0, 0.8, Color::BLACK).with_tool(StrokeTool::Smudge);
        let across = (0..30u16).map(|i| {
            let t = f32::from(i);
            sample(280.0 - 8.0 * t, 40.0 + 4.0 * t, 1.0, u32::from(i) * 8)
        });
        with_stroke(a, (style, across.collect()))
    };
    let turned = |a: &Artwork| {
        let placement = Placement::around([160.0, 100.0], 1.4, 0.3, [false, false]);
        let placement = placement.expect("placement");
        a.place_region(None, placement).expect("transform").artwork
    };
    let steps: [Step<'_>; 5] = [
        ("stroke", &stroke),
        ("hue", &hue),
        ("blur", &blur),
        ("smudge", &smudge),
        ("transform record", &turned),
    ];
    for (what, step) in steps {
        let [one, two] = both_ways(step);
        assert!(one.0 == two.0, "{what}: the canvas");
        assert!(one.1 == two.1, "{what}: the layer alone");
    }
}

/// The wave drawn on `artwork`'s active layer a few samples a frame into `target`, every frame
/// equal to a fresh target's; the finished artwork.
fn draw_frames(target: &mut CanvasTarget, artwork: &Artwork) -> Artwork {
    let core = core();
    let (style, samples) = wave();
    let mut session = artwork.begin_stroke(style, samples[0]).expect("begin");
    for chunk in samples[1..].chunks(7) {
        session.extend(chunk).expect("extend");
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(artwork)
        };
        RegionEvaluator::new(&core, target)
            .evaluate(&frame, &[])
            .expect("evaluate");
        let mut fresh = CanvasTarget::new(&core, SIZE).expect("target");
        RegionEvaluator::new(&core, &mut fresh)
            .evaluate(&frame, &[PixelRect::full(SIZE)])
            .expect("evaluate");
        let shown = target.read(&core).expect("readback");
        assert!(shown == fresh.read(&core).expect("readback"), "in progress");
    }
    session.finish().artwork
}

#[test]
fn the_merge_drawing_on_the_merged_layer_and_undo_through_changed_regions_equal_cold() {
    let core = core();
    let lower = Look {
        opacity: 0.6,
        mask: true,
        ..PLAIN
    };
    let upper = Look {
        clip: true,
        blend: BlendMode::Overlay,
        ..PLAIN
    };
    let before = alone(lower, upper);
    let after = merged(&before);
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&before), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&after), &changed_regions(&before, &after))
        .expect("evaluate");
    assert!(
        target.read(&core).expect("readback") == cold(&core, &after),
        "merged"
    );
    let drawn = draw_frames(&mut target, &after);
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&drawn), &[])
        .expect("evaluate");
    assert!(
        target.read(&core).expect("readback") == cold(&core, &drawn),
        "committed"
    );
    for (from, to) in [(&drawn, &after), (&after, &before)] {
        RegionEvaluator::new(&core, &mut target)
            .evaluate(&request(to), &changed_regions(from, to))
            .expect("evaluate");
        assert!(
            target.read(&core).expect("readback") == cold(&core, to),
            "undo"
        );
    }
    let reloaded = Artwork::from_storage(drawn.to_parts()).expect("reload");
    assert!(cold(&core, &reloaded) == cold(&core, &drawn), "reopened");
}

#[test]
fn a_merged_layer_exports_at_two_and_four_x_as_the_two_did() {
    let core = core();
    let lower = Look {
        mask: true,
        ..PLAIN
    };
    let upper = Look {
        opacity: 0.7,
        clip: true,
        blend: BlendMode::Multiply,
        mask: true,
    };
    let before = alone(lower, upper);
    let after = merged(&before);
    for scale in [2, 4] {
        let banding = Banding {
            rows: 96,
            columns: 4096,
        };
        let one = export(&core, &before, scale, banding);
        assert!(one == export(&core, &after, scale, banding), "{scale}x");
    }
}
