//! Plain, unvalidated artwork data for the store. `Artwork::to_parts` produces it;
//! `Artwork::from_storage` is the only way back and validates everything.

use crate::adjust::AdjustParts;
use crate::artwork::Artwork;
use crate::blend::BlendMode;
use crate::brush::{BrushRecipe, Curve, Dynamics, Grain};
use crate::deposit::{Accumulation, Deposit, Paper, PaperTexture, Tip};
use crate::error::CoreError;
use crate::id::GroupId;
use crate::layer::Layer;
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
}

/// A transform as plain data: its place among the layer's strokes, what it lifted (`None`: the
/// whole layer), the lifted pixels and where they went.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformParts {
    pub after: u64,
    pub lift: Option<MaskParts>,
    pub origin: [u32; 2],
    pub size: [u32; 2],
    /// Premultiplied 8-bit RGBA, rows top first.
    pub pixels: Vec<u8>,
    /// `[a, b, c, d]` and the translation, as `Placement::new` takes them.
    pub matrix: [f32; 4],
    pub translation: [f32; 2],
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

/// A brush recipe as plain data: curves as `[x, y]` points.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeParts {
    pub softness: f32,
    pub size_pressure: Vec<[f32; 2]>,
    pub size_speed: Vec<[f32; 2]>,
    pub opacity_pressure: Vec<[f32; 2]>,
    pub opacity_speed: Vec<[f32; 2]>,
    pub grain_strength: f32,
    pub grain_scale: u32,
    /// `None` for a covering brush.
    pub deposit: Option<AccumulationParts>,
}

/// An accumulating deposit as plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct AccumulationParts {
    pub tip: Tip,
    pub flow: f32,
    pub max_density: f32,
    pub paper: Option<PaperParts>,
}

/// A paper as plain data; `contact` a curve over pressure.
#[derive(Debug, Clone, PartialEq)]
pub struct PaperParts {
    pub texture: PaperTexture,
    pub contact: Vec<[f32; 2]>,
    pub scale: f32,
    pub depth: f32,
    pub dust: f32,
}

impl BrushRecipe {
    /// Builds a recipe from plain data, checking every value.
    ///
    /// # Errors
    /// `InvalidValue` for any curve, softness, grain or deposit value out of its range.
    pub fn from_parts(parts: &RecipeParts) -> Result<Self, CoreError> {
        let dynamics = |pressure: &[[f32; 2]], speed: &[[f32; 2]]| {
            Dynamics::new(Curve::new(pressure)?, Curve::new(speed)?)
        };
        let recipe = Self::new(
            parts.softness,
            dynamics(&parts.size_pressure, &parts.size_speed)?,
            dynamics(&parts.opacity_pressure, &parts.opacity_speed)?,
            Grain::new(parts.grain_strength, parts.grain_scale)?,
        )?;
        let Some(deposit) = &parts.deposit else {
            return Ok(recipe);
        };
        let paper = deposit
            .paper
            .as_ref()
            .map(|paper| {
                let contact = Curve::new(&paper.contact)?;
                Paper::new(paper.texture, contact, paper.scale, paper.depth, paper.dust)
            })
            .transpose()?;
        let accumulation =
            Accumulation::new(deposit.tip, deposit.flow, deposit.max_density, paper)?;
        recipe.with_deposit(Deposit::Accumulate(accumulation))
    }

    /// The recipe as plain data.
    #[must_use]
    pub fn to_parts(&self) -> RecipeParts {
        let points = |curve: &Curve| curve.points().to_vec();
        let deposit = match self.deposit() {
            Deposit::Cover => None,
            Deposit::Accumulate(accumulation) => Some(AccumulationParts {
                tip: accumulation.tip(),
                flow: accumulation.flow(),
                max_density: accumulation.max_density(),
                paper: accumulation.paper().map(|paper| PaperParts {
                    texture: paper.texture(),
                    contact: points(paper.contact()),
                    scale: paper.scale(),
                    depth: paper.depth(),
                    dust: paper.dust(),
                }),
            }),
        };
        RecipeParts {
            softness: self.softness(),
            size_pressure: points(self.size().pressure()),
            size_speed: points(self.size().speed()),
            opacity_pressure: points(self.opacity().pressure()),
            opacity_speed: points(self.opacity().speed()),
            grain_strength: self.grain().strength(),
            grain_scale: self.grain().scale(),
            deposit,
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
        mask: layer.mask.as_ref().map(|mask| LayerMaskParts {
            enabled: mask.enabled,
            inverted: mask.inverted,
            strokes: mask.strokes.iter().map(stroke_parts).collect(),
            edits: mask.edits.iter().map(edit_parts).collect(),
        }),
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
            let patch = transform.patch();
            let placement = transform.placement();
            EditParts::Transform(TransformParts {
                after: transform.after() as u64,
                lift: transform.lift().map(RegionMask::to_parts),
                origin: patch.origin(),
                size: patch.size(),
                pixels: patch.pixels().to_vec(),
                matrix: placement.matrix(),
                translation: placement.translation(),
            })
        }
        RegionEdit::Adjust(adjust) => EditParts::Adjust(adjust.to_parts()),
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
