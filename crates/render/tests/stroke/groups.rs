//! Groups on the GPU (D50): pass-through, so a Multiply layer in a group darkens what lies
//! under the group; the group's opacity mixes between that and what lies under it, exactly the
//! ungrouped result at 1; a hidden group shows nothing of its layers; and incremental equals
//! cold through group, clipping and blend edits and while drawing on a clipping layer in a
//! faded group.

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, CanvasSize, Color, GroupCommand, GroupId, LayerCommand,
    LayerId, RegionMask,
};
use openpaint_render::{CanvasTarget, FrameRequest, PixelRect, RegionEvaluator, changed_regions};

use crate::fixture::{
    SIZE, blank, cold, core, diagonal, pixel, request, wave, with_layer, with_stroke,
};

const BACK: [f32; 3] = [0.2, 0.7, 0.45];
const SOURCE: [f32; 3] = [0.85, 0.3, 0.6];
const RIGHT: [f32; 3] = [0.1, 0.25, 0.9];
const GROUP: GroupId = GroupId::new(4);

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

fn fill(artwork: &Artwork, mask: RegionMask, rgb: [f32; 3]) -> Artwork {
    artwork.fill_region(mask, color(rgb)).expect("fill").artwork
}

/// Layer 1: `BACK` on the left half. In group 4: layer 2, `SOURCE` everywhere by Multiply, and
/// layer 3, `RIGHT` on the right third.
fn grouped() -> Artwork {
    let bottom = fill(&blank(), columns(0, 160), BACK);
    let middle = fill(&with_layer(&bottom, 1.0), columns(0, SIZE[0]), SOURCE);
    let blend = LayerCommand::SetBlend {
        layer: LayerId::new(2),
        blend: BlendMode::Multiply,
    };
    let middle = apply(&middle, ArtworkCommand::Layer(blend));
    let top = fill(&with_layer(&middle, 1.0), columns(214, SIZE[0]), RIGHT);
    let group = GroupCommand::Create {
        layer: LayerId::new(3),
    };
    apply(&top, ArtworkCommand::Group(group))
}

fn faded(artwork: &Artwork, opacity: f32) -> Artwork {
    let command = GroupCommand::SetOpacity {
        group: GROUP,
        opacity,
    };
    apply(artwork, ArtworkCommand::Group(command))
}

fn near(actual: [u8; 4], expected: [f64; 3], what: &str) {
    let expected = expected.map(|v| (v * 255.0).round() as u8);
    let close = actual[..3]
        .iter()
        .zip(expected)
        .all(|(a, e)| a.abs_diff(e) <= 1);
    assert!(close, "{what}: {actual:?}, expected {expected:?}");
}

#[test]
fn a_group_passes_its_layers_through_and_its_opacity_fades_the_result() {
    let core = core();
    let artwork = grouped();
    assert_eq!(artwork.groups().len(), 1);
    let image = cold(&core, &faded(&artwork, 0.4));
    // Left: the Multiply layer darkens the colour under the group (pass-through), and 0.4 of
    // that shows over the colour: 0.6 x 0.2 + 0.4 x (0.2 x 0.85) = 0.188 in red.
    let left = [0, 1, 2].map(|c| {
        let (b, s) = (f64::from(BACK[c]), f64::from(SOURCE[c]));
        0.6 * b + 0.4 * b * s
    });
    near(pixel(&image, [60, 100]), left, "over the colour");
    assert_eq!((left[0] * 255.0).round(), 48.0, "by hand");
    // Right: the top layer covers, 0.4 of it over white paper.
    let right = [0, 1, 2].map(|c| 0.6 + 0.4 * f64::from(RIGHT[c]));
    near(pixel(&image, [300, 100]), right, "over white");
    // Middle: Multiply over white is the source itself.
    let middle = [0, 1, 2].map(|c| 0.6 + 0.4 * f64::from(SOURCE[c]));
    near(pixel(&image, [190, 100]), middle, "the source over white");
}

#[test]
fn a_full_group_equals_its_layers_ungrouped_and_a_hidden_one_shows_nothing_of_them() {
    let core = core();
    let artwork = grouped();
    let ungrouped = apply(
        &artwork,
        ArtworkCommand::Group(GroupCommand::Ungroup(GROUP)),
    );
    assert!(
        cold(&core, &artwork) == cold(&core, &ungrouped),
        "bit for bit"
    );
    let hide = GroupCommand::SetVisible {
        group: GROUP,
        visible: false,
    };
    let hidden = apply(&faded(&artwork, 0.4), ArtworkCommand::Group(hide));
    let bottom = fill(&blank(), columns(0, 160), BACK);
    assert!(
        cold(&core, &hidden) == cold(&core, &bottom),
        "only the layer under it"
    );
}

/// Versions through every new edit, each from the one before.
fn edits() -> Vec<Artwork> {
    let first = with_stroke(&with_stroke(&grouped(), wave()), diagonal());
    let commands = [
        ArtworkCommand::Group(GroupCommand::SetOpacity {
            group: GROUP,
            opacity: 0.55,
        }),
        ArtworkCommand::Layer(LayerCommand::SetClipping {
            layer: LayerId::new(3),
            clipping: true,
        }),
        ArtworkCommand::Layer(LayerCommand::SetBlend {
            layer: LayerId::new(3),
            blend: BlendMode::Overlay,
        }),
        ArtworkCommand::Layer(LayerCommand::Lower(LayerId::new(2))),
        ArtworkCommand::Layer(LayerCommand::Raise(LayerId::new(2))),
        ArtworkCommand::Group(GroupCommand::SetVisible {
            group: GROUP,
            visible: false,
        }),
        ArtworkCommand::Group(GroupCommand::Ungroup(GROUP)),
        ArtworkCommand::Group(GroupCommand::New),
    ];
    let mut versions = vec![first];
    for command in commands {
        let next = apply(versions.last().expect("version"), command);
        versions.push(next);
    }
    versions
}

#[test]
fn group_clipping_and_blend_edits_through_changed_regions_equal_cold() {
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
fn drawing_on_a_clipping_layer_in_a_faded_group_equals_cold_every_frame() {
    let core = core();
    let clip = LayerCommand::SetClipping {
        layer: LayerId::new(3),
        clipping: true,
    };
    let artwork = apply(&faded(&grouped(), 0.6), ArtworkCommand::Layer(clip));
    let mut target = CanvasTarget::new(&core, SIZE).expect("target");
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&artwork), &[PixelRect::full(SIZE)])
        .expect("evaluate");
    let (style, samples) = wave();
    let mut session = artwork.begin_stroke(style, samples[0]).expect("begin");
    for chunk in samples[1..].chunks(5) {
        session.extend(chunk).expect("extend");
        let frame = FrameRequest {
            active: Some(session.active()),
            ..request(&artwork)
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
            "in progress = cold"
        );
    }
    let finished = session.finish().artwork;
    RegionEvaluator::new(&core, &mut target)
        .evaluate(&request(&finished), &[])
        .expect("evaluate");
    assert!(target.read(&core).expect("readback") == cold(&core, &finished));
}
