//! Merge Down on the GPU (S39, D66): the upper layer becomes a nested record in the lower one
//! and the canvas shows what it showed. Where the upper layer clipped to the lower one (its
//! base) the nested record is the chain's own composite, bit for bit, at every mode, opacity and
//! mask; merged over by a shared mode that composes (Normal, Multiply, Screen) the order of
//! float operations changes, so a pixel may move by one 8-bit level, rarely (counted). Nested
//! merges (to the nesting limit), faded pass-through groups and clipping chains alike. What
//! comes after a merge: `merge_after.rs`.

use openpaint_core::{
    Artwork, ArtworkCommand, BlendMode, CanvasSize, Color, GroupCommand, LayerCommand, LayerId,
    MaskCommand, RegionMask,
};
use openpaint_render::RgbaImage;

use crate::fixture::{
    SIZE, blank, cold, core, diagonal, plain, sample, wave, with_layer, with_stroke,
};

const BACK: [f32; 3] = [0.2, 0.7, 0.45];
pub const SOURCE: [f32; 3] = [0.85, 0.3, 0.6];
const BASE: [f32; 3] = [0.95, 0.85, 0.2];

/// How one of the two layers composites.
#[derive(Debug, Clone, Copy)]
pub struct Look {
    pub opacity: f32,
    pub blend: BlendMode,
    pub clip: bool,
    pub mask: bool,
}

pub const PLAIN: Look = Look {
    opacity: 1.0,
    blend: BlendMode::Normal,
    clip: false,
    mask: false,
};

pub fn color([r, g, b]: [f32; 3]) -> Color {
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

fn layer(artwork: &Artwork, command: LayerCommand) -> Artwork {
    apply(artwork, ArtworkCommand::Layer(command))
}

fn fill(artwork: &Artwork, mask: RegionMask, rgb: [f32; 3]) -> Artwork {
    artwork.fill_region(mask, color(rgb)).expect("fill").artwork
}

/// The active layer given `look`, a grey mask over `masked` columns when it has one.
fn styled(artwork: &Artwork, look: Look, masked: (u32, u32)) -> Artwork {
    let id = artwork.active();
    let mut out = layer(
        artwork,
        LayerCommand::SetOpacity {
            layer: id,
            opacity: look.opacity,
        },
    );
    out = layer(
        &out,
        LayerCommand::SetBlend {
            layer: id,
            blend: look.blend,
        },
    );
    out = layer(
        &out,
        LayerCommand::SetClipping {
            layer: id,
            clipping: look.clip,
        },
    );
    if look.mask {
        out = apply(&out, ArtworkCommand::Mask(MaskCommand::Add(id)));
        out = fill(&out, columns(masked.0, masked.1), [0.4; 3]);
        out = layer(&out, LayerCommand::Select(id));
    }
    out
}

/// On `under`, a lower layer (`BACK` over columns 20..260 and the wave) and an upper one
/// (`SOURCE` over columns 100..300 and the diagonal), active.
fn pair(under: &Artwork, lower: Look, upper: Look) -> Artwork {
    let low = fill(&with_layer(under, 1.0), columns(20, 260), BACK);
    let low = styled(&with_stroke(&low, wave()), lower, (0, 120));
    let up = fill(&with_layer(&low, 1.0), columns(100, 300), SOURCE);
    styled(&with_stroke(&up, diagonal()), upper, (150, 250))
}

/// The two layers alone on the canvas (the first, empty layer under them).
pub fn alone(lower: Look, upper: Look) -> Artwork {
    pair(&blank(), lower, upper)
}

/// Under the two layers, a base they can clip to: `BASE` over columns 0..200.
fn on_base(lower: Look, upper: Look) -> Artwork {
    pair(&fill(&blank(), columns(0, 200), BASE), lower, upper)
}

pub fn merged(artwork: &Artwork) -> Artwork {
    layer(artwork, LayerCommand::MergeDown(artwork.active()))
}

/// The largest difference of any channel and how many channels differ.
fn difference(a: &RgbaImage, b: &RgbaImage) -> (u8, usize) {
    let pairs = a.pixels().iter().zip(b.pixels());
    let diffs: Vec<u8> = pairs.map(|(x, y)| x.abs_diff(*y)).collect();
    let largest = diffs.iter().copied().max().unwrap_or(0);
    (largest, diffs.iter().filter(|d| **d > 0).count())
}

/// Merging the active layer of `artwork` shows exactly what it showed.
fn exact(artwork: &Artwork, what: &str) {
    let core = core();
    let after = merged(artwork);
    assert_eq!(after.layers().len(), artwork.layers().len() - 1, "{what}");
    assert!(cold(&core, &after) == cold(&core, artwork), "{what}");
}

/// Merging the active layer of `artwork` moves no channel by more than one level; the count.
fn close(artwork: &Artwork, what: &str) -> usize {
    let core = core();
    let (largest, count) = difference(&cold(&core, &merged(artwork)), &cold(&core, artwork));
    assert!(largest <= 1, "{what}: {largest} levels");
    // Rare: well under one channel in a thousand of the 320 x 200 x 4.
    assert!(count < 256, "{what}: {count} channels");
    count
}

#[test]
fn a_layer_clipped_to_the_one_below_merges_bit_for_bit_at_every_mode_opacity_and_mask() {
    for blend in BlendMode::ALL {
        for lower_blend in [BlendMode::Normal, BlendMode::Multiply] {
            for (lower_mask, upper_mask) in
                [(false, false), (true, false), (false, true), (true, true)]
            {
                let lower = Look {
                    opacity: 0.6,
                    blend: lower_blend,
                    mask: lower_mask,
                    ..PLAIN
                };
                let upper = Look {
                    opacity: 0.7,
                    blend,
                    clip: true,
                    mask: upper_mask,
                };
                let what = format!("{blend:?} on {lower_blend:?}, masks {lower_mask} {upper_mask}");
                exact(&alone(lower, upper), &what);
            }
        }
    }
}

#[test]
fn layers_of_one_composing_mode_merge_within_a_level() {
    let mut differing = 0;
    for blend in [BlendMode::Normal, BlendMode::Multiply, BlendMode::Screen] {
        for (opacity, masks) in [(1.0, false), (0.6, false), (1.0, true), (0.6, true)] {
            let lower = Look {
                opacity,
                blend,
                mask: masks,
                ..PLAIN
            };
            let upper = Look {
                opacity: 0.8,
                ..lower
            };
            let what = format!("{blend:?} at {opacity}, masks {masks}");
            differing += close(&alone(lower, upper), &what);
            // Both clipping to one base: the merged layer clips to it.
            let clipped = |look: Look| Look { clip: true, ..look };
            differing += close(&on_base(clipped(lower), clipped(upper)), &what);
        }
    }
    eprintln!("over merges: {differing} channels one level off of 24 x 256000");
}

#[test]
fn a_normal_pair_at_full_strength_without_masks_merges_bit_for_bit() {
    // The lower layer opaque where the upper paints: the same floats either way.
    let lower = fill(&with_layer(&blank(), 1.0), columns(0, SIZE[0]), BACK);
    let upper = with_stroke(&with_layer(&lower, 0.7), diagonal());
    exact(&upper, "opaque lower layer");
}

#[test]
fn merges_in_a_faded_pass_through_group_and_merges_of_merges_show_the_same() {
    let lower = Look {
        blend: BlendMode::Multiply,
        ..PLAIN
    };
    let upper = Look {
        opacity: 0.7,
        clip: true,
        blend: BlendMode::Screen,
        mask: true,
    };
    let artwork = on_base(lower, upper);
    let grouped = apply(
        &artwork,
        ArtworkCommand::Group(GroupCommand::Create {
            layer: artwork.active(),
        }),
    );
    let group = grouped.groups()[0].id();
    let fade = GroupCommand::SetOpacity {
        group,
        opacity: 0.6,
    };
    exact(&apply(&grouped, ArtworkCommand::Group(fade)), "faded group");
    // A third layer clipping to the same base, merged after the second: nested twice over.
    let third = with_stroke(&with_layer(&artwork, 0.5), wave());
    let third = styled(
        &third,
        Look {
            clip: true,
            ..upper
        },
        (40, 90),
    );
    let once = merged(&layer(&third, LayerCommand::Select(LayerId::new(3))));
    let once = layer(&once, LayerCommand::Select(LayerId::new(4)));
    let core = core();
    assert!(
        cold(&core, &once) == cold(&core, &third),
        "the middle one first"
    );
    exact(&once, "then the merged layer under the third");
    // A merged layer merged again into the layer below it.
    let twice = merged(&merged(&third));
    assert_eq!(twice.layers().len(), 2);
    assert!(cold(&core, &twice) == cold(&core, &third), "both");
}

#[test]
fn records_nested_to_the_limit_evaluate_as_the_layers_did() {
    let mut artwork = with_stroke(&blank(), wave());
    let bottom = artwork.active();
    for round in 0..openpaint_core::MAX_NESTING {
        // Faded again before each merge, so the merged content nests one level deeper.
        let faded = layer(
            &artwork,
            LayerCommand::SetOpacity {
                layer: bottom,
                opacity: 0.8,
            },
        );
        let line = (0..12u16).map(|i| {
            let t = f32::from(i);
            let y = 20.0 + 20.0 * round as f32;
            sample(20.0 + 24.0 * t, y, 0.9, u32::from(i) * 8)
        });
        let style = plain(14.0, 0.7, color(SOURCE));
        let upper = with_stroke(&with_layer(&faded, 1.0), (style, line.collect()));
        close(&upper, &format!("round {round}"));
        artwork = merged(&upper);
    }
    let depth = artwork.layers()[0].edits()[0]
        .nested()
        .map(|n| n.layer().depth());
    assert_eq!(depth, Some(openpaint_core::MAX_NESTING));
}
