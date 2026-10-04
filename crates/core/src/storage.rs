//! `Artwork::from_storage`: the single entry that turns stored data into an artwork, with full
//! validation. Nothing loaded from outside core becomes an `Artwork` any other way.

use std::collections::HashSet;
use std::sync::Arc;

use crate::adjust::AdjustEdit;
use crate::artwork::{Artwork, ArtworkState, MAX_LAYERS};
use crate::brush::BrushRecipe;
use crate::error::CoreError;
use crate::group::{LayerGroup, check_groups};
use crate::id::{GroupId, IdCounters, LayerId, MAX_ID, StrokeId};
use crate::layer::{Layer, LayerProps};
use crate::mask::{LayerMask, is_neutral};
use crate::parts::{
    ArtworkParts, EditParts, FillParts, GroupParts, LayerMaskParts, LayerParts, MaskParts,
    RecipeParts, SampleParts, StrokeParts, TransformParts,
};
use crate::region::{FillEdit, RegionEdit, RegionMask};
use crate::stroke::{
    MAX_STROKE_SAMPLES, Stroke, StrokeBlend, StrokeSample, StrokeStyle, StrokeTool, check_order,
};
use crate::stroke_list::StrokeList;
use crate::transform::{Patch, Placement, TransformEdit, transform_from};
use crate::value::{CanvasPoint, CanvasSize, Color, LayerName, invalid};

/// Ids seen so far, checked against the stored counters, and the recipes built so far: strokes
/// drawn with one brush share one recipe, as they did before they were stored.
struct IdCheck {
    counters: IdCounters,
    layers: HashSet<u64>,
    strokes: HashSet<u64>,
    recipes: Vec<Arc<BrushRecipe>>,
    /// The last clip built: strokes drawn one after another inside one selection share it, as
    /// they did before they were stored.
    clip: Option<RegionMask>,
}

impl IdCheck {
    fn clip(&mut self, canvas: CanvasSize, parts: &MaskParts) -> Result<RegionMask, CoreError> {
        let clip = RegionMask::from_runs(canvas, parts.origin, parts.size, &parts.runs)?;
        if let Some(known) = self.clip.as_ref().filter(|known| **known == clip) {
            return Ok(known.clone());
        }
        self.clip = Some(clip.clone());
        Ok(clip)
    }

    fn recipe(&mut self, parts: &RecipeParts) -> Result<Arc<BrushRecipe>, CoreError> {
        let recipe = BrushRecipe::from_parts(parts)?;
        if let Some(known) = self.recipes.iter().find(|known| ***known == recipe) {
            return Ok(Arc::clone(known));
        }
        let recipe = Arc::new(recipe);
        self.recipes.push(Arc::clone(&recipe));
        Ok(recipe)
    }

    fn layer(&mut self, id: u64) -> Result<LayerId, CoreError> {
        claim(&mut self.layers, id, self.counters.next_layer, "layer id")?;
        Ok(LayerId::new(id))
    }

    fn stroke(&mut self, id: u64) -> Result<StrokeId, CoreError> {
        claim(
            &mut self.strokes,
            id,
            self.counters.next_stroke,
            "stroke id",
        )?;
        Ok(StrokeId::new(id))
    }
}

fn claim(seen: &mut HashSet<u64>, id: u64, next: u64, what: &'static str) -> Result<(), CoreError> {
    if id == 0 || id >= next {
        return Err(CoreError::IdOutOfRange { what, id });
    }
    if !seen.insert(id) {
        return Err(CoreError::DuplicateId { what, id });
    }
    Ok(())
}

impl Artwork {
    /// Rebuilds an artwork from stored data, checking every value, id and invariant.
    ///
    /// # Errors
    /// `InvalidValue` for any out-of-range value; `NoLayers` / `LayerLimit` for the layer count;
    /// `IdOutOfRange` / `DuplicateId` for ids; `UnknownLayer` when `active` names no layer;
    /// `EmptyStroke`, `SampleLimit` and `SampleOrder` for stroke samples; `InvalidValue` for a
    /// fill whose mask leaves the canvas or whose place is out of order or past the strokes.
    pub fn from_storage(parts: ArtworkParts) -> Result<Self, CoreError> {
        let canvas = CanvasSize::new(parts.width, parts.height)?;
        if parts.layers.is_empty() {
            return Err(CoreError::NoLayers);
        }
        if parts.layers.len() > MAX_LAYERS {
            return Err(CoreError::LayerLimit { max: MAX_LAYERS });
        }
        let counters = IdCounters {
            next_layer: counter(parts.next_layer_id)?,
            next_stroke: counter(parts.next_stroke_id)?,
        };
        let mut ids = IdCheck {
            counters,
            layers: HashSet::new(),
            strokes: HashSet::new(),
            recipes: Vec::new(),
            clip: None,
        };
        let layers = parts
            .layers
            .iter()
            .map(|layer| layer_from(layer, canvas, &mut ids))
            .collect::<Result<Vec<_>, _>>()?;
        let groups = parts
            .groups
            .iter()
            .map(|group| group_from(group, &mut ids))
            .collect::<Result<Vec<_>, _>>()?;
        check_groups(&layers, &groups)?;
        let active = LayerId::new(parts.active);
        let Some(on) = layers.iter().find(|layer| layer.id == active) else {
            return Err(CoreError::UnknownLayer(active));
        };
        match &on.mask {
            None if parts.on_mask => return Err(CoreError::NoMask(active)),
            Some(mask) if parts.on_mask && !mask.enabled => {
                return Err(CoreError::MaskOff(active));
            }
            _ => {}
        }
        Ok(Self::from_state(ArtworkState {
            canvas,
            layers,
            groups,
            active,
            on_mask: parts.on_mask,
            ids: counters,
        }))
    }
}

/// A group's id comes from the layer ids' counter and is never a layer's.
fn group_from(parts: &GroupParts, ids: &mut IdCheck) -> Result<LayerGroup, CoreError> {
    claim(
        &mut ids.layers,
        parts.id,
        ids.counters.next_layer,
        "group id",
    )?;
    let name = LayerName::new(&parts.name)?;
    LayerGroup::new(GroupId::new(parts.id), name, parts.visible, parts.opacity)
}

fn counter(next: u64) -> Result<u64, CoreError> {
    if (1..=MAX_ID).contains(&next) {
        Ok(next)
    } else {
        Err(invalid("id counter"))
    }
}

fn layer_from(
    parts: &LayerParts,
    canvas: CanvasSize,
    ids: &mut IdCheck,
) -> Result<Layer, CoreError> {
    let id = ids.layer(parts.id)?;
    let name = LayerName::new(&parts.name)?;
    let props = LayerProps::new(
        (parts.visible, parts.opacity),
        (parts.locked, parts.alpha_locked),
        (parts.clipping, parts.blend),
        parts.group.map(GroupId::new),
    )?;
    let (strokes, edits) = content_from(&parts.strokes, &parts.edits, canvas, ids)?;
    let mask = parts
        .mask
        .as_ref()
        .map(|mask| mask_from(mask, canvas, ids))
        .transpose()?;
    Ok(Layer {
        id,
        name,
        props,
        strokes,
        edits,
        mask,
    })
}

/// A mask's strokes paint neutral grey over it or smudge it, never under Alpha Lock; its edits
/// are neutral fills or adjustments of its grey (S28).
fn mask_from(
    parts: &LayerMaskParts,
    canvas: CanvasSize,
    ids: &mut IdCheck,
) -> Result<LayerMask, CoreError> {
    let (strokes, edits) = content_from(&parts.strokes, &parts.edits, canvas, ids)?;
    let stroke_ok = |stroke: &Stroke| match stroke.blend() {
        StrokeBlend::Over => is_neutral(stroke.style().color()),
        StrokeBlend::Smudge => true,
        _ => false,
    };
    let edit_ok = |edit: &RegionEdit| match edit {
        RegionEdit::Fill(fill) => !fill.alpha_locked && is_neutral(fill.color),
        RegionEdit::Transform(_) => false,
        RegionEdit::Adjust(_) => true,
    };
    if !strokes.iter().all(stroke_ok) || !edits.iter().all(edit_ok) {
        return Err(invalid("mask content"));
    }
    Ok(LayerMask {
        enabled: parts.enabled,
        inverted: parts.inverted,
        strokes,
        edits,
    })
}

/// Strokes and edits, each edit at a place among the strokes that never goes back.
fn content_from(
    strokes: &[StrokeParts],
    edits: &[EditParts],
    canvas: CanvasSize,
    ids: &mut IdCheck,
) -> Result<(StrokeList, Arc<[RegionEdit]>), CoreError> {
    let strokes = strokes
        .iter()
        .map(|stroke| stroke_from(stroke, canvas, ids))
        .collect::<Result<Vec<_>, _>>()?;
    let mut after = 0;
    let mut built = Vec::with_capacity(edits.len());
    for edit in edits {
        let edit = match edit {
            EditParts::Fill(fill) => RegionEdit::Fill(fill_from(fill, canvas)?),
            EditParts::Transform(transform) => {
                RegionEdit::Transform(transform_parts_from(transform, canvas)?)
            }
            EditParts::Adjust(adjust) => {
                RegionEdit::Adjust(AdjustEdit::from_parts(adjust, canvas)?)
            }
        };
        if edit.after() < after || edit.after() > strokes.len() {
            return Err(invalid("edit place"));
        }
        after = edit.after();
        built.push(edit);
    }
    Ok((StrokeList::from_strokes(&strokes), Arc::from(built)))
}

fn fill_from(parts: &FillParts, canvas: CanvasSize) -> Result<FillEdit, CoreError> {
    let [r, g, b] = parts.color;
    let mask = RegionMask::from_runs(canvas, parts.origin, parts.size, &parts.runs)?;
    Ok(FillEdit {
        after: usize::try_from(parts.after).map_err(|_| invalid("fill place"))?,
        mask,
        color: Color::rgb(r, g, b)?,
        alpha_locked: parts.alpha_locked,
    })
}

fn transform_parts_from(
    parts: &TransformParts,
    canvas: CanvasSize,
) -> Result<TransformEdit, CoreError> {
    let after = usize::try_from(parts.after).map_err(|_| invalid("transform place"))?;
    let lift = parts
        .lift
        .as_ref()
        .map(|lift| RegionMask::from_runs(canvas, lift.origin, lift.size, &lift.runs))
        .transpose()?;
    let patch = parts
        .patch
        .as_ref()
        .map(|patch| Patch::new(canvas, patch.origin, patch.size, patch.pixels.clone()))
        .transpose()?;
    let placement =
        Placement::new(parts.matrix, parts.translation).ok_or(invalid("transform placement"))?;
    transform_from(canvas, after, lift, patch, placement)
}

fn stroke_from(
    parts: &StrokeParts,
    canvas: CanvasSize,
    ids: &mut IdCheck,
) -> Result<Stroke, CoreError> {
    let id = ids.stroke(parts.id)?;
    let [r, g, b] = parts.color;
    let recipe = ids.recipe(&parts.recipe)?;
    let style = StrokeStyle::new(recipe, parts.size, parts.opacity, Color::rgb(r, g, b)?)?;
    let (tool, alpha_locked) = match parts.blend {
        StrokeBlend::Over => (StrokeTool::Paint, false),
        StrokeBlend::Atop => (StrokeTool::Paint, true),
        StrokeBlend::Erase => (StrokeTool::Erase, false),
        StrokeBlend::Smudge => (StrokeTool::Smudge, false),
        StrokeBlend::SmudgeLocked => (StrokeTool::Smudge, true),
    };
    if parts.samples.is_empty() {
        return Err(CoreError::EmptyStroke);
    }
    if parts.samples.len() > MAX_STROKE_SAMPLES {
        return Err(CoreError::SampleLimit {
            max: MAX_STROKE_SAMPLES,
        });
    }
    let samples = parts
        .samples
        .iter()
        .map(sample_from)
        .collect::<Result<Vec<_>, _>>()?;
    check_order(None, &samples)?;
    let style = style.with_tool(tool);
    let clip = parts
        .clip
        .as_ref()
        .map(|clip| ids.clip(canvas, clip))
        .transpose()?;
    Ok(Stroke::new(
        id,
        style,
        alpha_locked,
        Arc::from(samples),
        clip,
    ))
}

fn sample_from(parts: &SampleParts) -> Result<StrokeSample, CoreError> {
    let position = CanvasPoint::new(parts.x, parts.y)?;
    StrokeSample::new(
        position,
        parts.pressure,
        parts.tilt,
        parts.azimuth,
        parts.time_ms,
    )
}
