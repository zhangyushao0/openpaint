//! Plain, unvalidated artwork data for the store. `Artwork::to_parts` produces it;
//! `Artwork::from_storage` is the only way back and validates everything.

use crate::adjust::AdjustParts;
use crate::artwork::Artwork;
use crate::blend::BlendMode;
use crate::id::GroupId;
use crate::layer::Layer;
use crate::mask::LayerMask;
use crate::nested::NestedEdit;
pub use crate::parts_recipe::{AccumulationParts, PaperParts, RecipeParts};
use crate::region::{RegionEdit, RegionMask};
use crate::stroke::{Stroke, StrokeBlend, StrokeSample};

/// A whole artwork as plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtworkParts {
    pub width: u32,
    pub height: u32,
    /// Bottom to top.
    pub layers: Vec<LayerParts>,
    /// Oldest first.
    pub groups: Vec<GroupParts>,
    pub active: u64,
    /// The active layer's mask is the target of strokes and fills.
    pub on_mask: bool,
    pub next_layer_id: u64,
    pub next_stroke_id: u64,
}

/// A layer mask as plain data: its strokes and fills are in neutral grey.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerMaskParts {
    pub enabled: bool,
    pub inverted: bool,
    /// Oldest first; paint over or smudge, never under Alpha Lock.
    pub strokes: Vec<StrokeParts>,
    /// Fills only, oldest first.
    pub edits: Vec<EditParts>,
}

/// A layer group as plain data; its layers name its id.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupParts {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerParts {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub locked: bool,
    pub alpha_locked: bool,
    pub clipping: bool,
    pub blend: BlendMode,
    /// The group's id, if the layer lies in one.
    pub group: Option<u64>,
    /// Oldest first.
    pub strokes: Vec<StrokeParts>,
    /// Fills and transforms, oldest first.
    pub edits: Vec<EditParts>,
    pub mask: Option<LayerMaskParts>,
}

/// A region edit as plain data.
#[derive(Debug, Clone, PartialEq)]
pub enum EditParts {
    Fill(FillParts),
    Transform(TransformParts),
    Adjust(AdjustParts),
    Nested(NestedParts),
    /// Pixels cleared (S40, Cut).
    Clear(ClearParts),
}

/// Pixels cleared (S40) as plain data: their place among the layer's strokes and the mask.
#[derive(Debug, Clone, PartialEq)]
pub struct ClearParts {
    pub after: u64,
    pub mask: MaskParts,
}

/// A layer merged into another (S39) as plain data: its place among that layer's strokes, the
/// merged layer's id, how it composited (`clipped`: atop the content before it), its content.
#[derive(Debug, Clone, PartialEq)]
pub struct NestedParts {
    pub after: u64,
    pub id: u64,
    pub opacity: f32,
    pub blend: BlendMode,
    pub clipped: bool,
    pub strokes: Vec<StrokeParts>,
    pub edits: Vec<EditParts>,
    /// Enabled; its strokes and fills in neutral grey.
    pub mask: Option<LayerMaskParts>,
}

/// A transform as plain data: its place among the layer's strokes, what it moved (`None`: the
/// whole layer), where it went, and for an older transform or an inserted picture the pixels
/// it placed.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformParts {
    pub after: u64,
    pub lift: Option<MaskParts>,
    pub patch: Option<PatchParts>,
    /// `[a, b, c, d]` and the translation, as `Placement::new` takes them.
    pub matrix: [f32; 4],
    pub translation: [f32; 2],
}

/// A transform's pixels as plain data (`Patch::new`).
#[derive(Debug, Clone, PartialEq)]
pub struct PatchParts {
    pub origin: [u32; 2],
    pub size: [u32; 2],
    /// Premultiplied 8-bit RGBA, rows top first.
    pub pixels: Vec<u8>,
}

/// A fill as plain data: its place among the layer's strokes, its colour and its mask as runs
/// over a rectangle (`RegionMask::from_runs`).
#[derive(Debug, Clone, PartialEq)]
pub struct FillParts {
    /// How many of the layer's strokes lie under it.
    pub after: u64,
    /// sRGB red, green, blue.
    pub color: [f32; 3],
    pub alpha_locked: bool,
    pub origin: [u32; 2],
    pub size: [u32; 2],
    /// Alternately outside and inside, row by row, starting outside.
    pub runs: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrokeParts {
    pub id: u64,
    /// Paint over, paint atop (Alpha Lock) or erase; gives the style's tool.
    pub blend: StrokeBlend,
    /// The brush recipe the stroke was drawn with.
    pub recipe: RecipeParts,
    pub size: f32,
    pub opacity: f32,
    /// sRGB red, green, blue.
    pub color: [f32; 3],
    pub samples: Vec<SampleParts>,
    /// The selection the stroke was drawn inside.
    pub clip: Option<MaskParts>,
}

/// A region mask as plain data: runs over a rectangle (`RegionMask::from_runs`).
#[derive(Debug, Clone, PartialEq)]
pub struct MaskParts {
    pub origin: [u32; 2],
    pub size: [u32; 2],
    /// Alternately outside and inside, row by row, starting outside.
    pub runs: Vec<u32>,
}

impl RegionMask {
    /// The mask as plain data.
    #[must_use]
    pub fn to_parts(&self) -> MaskParts {
        MaskParts {
            origin: self.origin(),
            size: self.size(),
            runs: self.runs(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleParts {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt: f32,
    pub azimuth: f32,
    pub time_ms: u32,
}

impl Artwork {
    /// The artwork as plain data for storage.
    #[must_use]
    pub fn to_parts(&self) -> ArtworkParts {
        let state = self.state();
        ArtworkParts {
            width: state.canvas.width(),
            height: state.canvas.height(),
            layers: state.layers.iter().map(layer_parts).collect(),
            groups: state
                .groups
                .iter()
                .map(|group| GroupParts {
                    id: group.id.get(),
                    name: group.name.as_str().to_owned(),
                    visible: group.visible,
                    opacity: group.opacity,
                })
                .collect(),
            active: state.active.get(),
            on_mask: state.on_mask,
            next_layer_id: state.ids.next_layer,
            next_stroke_id: state.ids.next_stroke,
        }
    }
}

fn layer_parts(layer: &Layer) -> LayerParts {
    LayerParts {
        id: layer.id.get(),
        name: layer.name.as_str().to_owned(),
        visible: layer.props.visible,
        opacity: layer.props.opacity,
        locked: layer.props.locked,
        alpha_locked: layer.props.alpha_locked,
        clipping: layer.props.clipping,
        blend: layer.props.blend,
        group: layer.props.group.map(GroupId::get),
        strokes: layer.strokes.iter().map(stroke_parts).collect(),
        edits: layer.edits.iter().map(edit_parts).collect(),
        mask: layer.mask.as_ref().map(mask_parts),
    }
}

fn mask_parts(mask: &LayerMask) -> LayerMaskParts {
    LayerMaskParts {
        enabled: mask.enabled,
        inverted: mask.inverted,
        strokes: mask.strokes.iter().map(stroke_parts).collect(),
        edits: mask.edits.iter().map(edit_parts).collect(),
    }
}

fn nested_parts(nested: &NestedEdit) -> NestedParts {
    let layer = nested.layer();
    NestedParts {
        after: nested.after() as u64,
        id: layer.id.get(),
        opacity: layer.opacity,
        blend: layer.blend,
        clipped: layer.clipped,
        strokes: layer.strokes.iter().map(stroke_parts).collect(),
        edits: layer.edits.iter().map(edit_parts).collect(),
        mask: layer.mask.as_ref().map(mask_parts),
    }
}

fn edit_parts(edit: &RegionEdit) -> EditParts {
    match edit {
        RegionEdit::Fill(fill) => {
            let color = fill.color;
            EditParts::Fill(FillParts {
                after: fill.after as u64,
                color: [color.r(), color.g(), color.b()],
                alpha_locked: fill.alpha_locked,
                origin: fill.mask.origin(),
                size: fill.mask.size(),
                runs: fill.mask.runs(),
            })
        }
        RegionEdit::Transform(transform) => {
            let placement = transform.placement();
            EditParts::Transform(TransformParts {
                after: transform.after() as u64,
                lift: transform.lift().map(RegionMask::to_parts),
                patch: transform.patch().map(|patch| PatchParts {
                    origin: patch.origin(),
                    size: patch.size(),
                    pixels: patch.pixels().to_vec(),
                }),
                matrix: placement.matrix(),
                translation: placement.translation(),
            })
        }
        RegionEdit::Adjust(adjust) => EditParts::Adjust(adjust.to_parts()),
        RegionEdit::Nested(nested) => EditParts::Nested(nested_parts(nested)),
        RegionEdit::Clear(clear) => EditParts::Clear(ClearParts {
            after: clear.after() as u64,
            mask: clear.mask().to_parts(),
        }),
    }
}

fn stroke_parts(stroke: &Stroke) -> StrokeParts {
    let style = stroke.style();
    let color = style.color();
    StrokeParts {
        id: stroke.id().get(),
        blend: stroke.blend(),
        recipe: style.recipe().to_parts(),
        size: style.size(),
        opacity: style.opacity(),
        color: [color.r(), color.g(), color.b()],
        samples: stroke.samples().iter().map(sample_parts).collect(),
        clip: stroke.clip().map(RegionMask::to_parts),
    }
}

fn sample_parts(sample: &StrokeSample) -> SampleParts {
    let position = sample.position();
    SampleParts {
        x: position.x(),
        y: position.y(),
        pressure: sample.pressure(),
        tilt: sample.tilt(),
        azimuth: sample.azimuth(),
        time_ms: sample.time_ms(),
    }
}
