//! `Artwork::from_storage`: the single entry that turns stored data into an artwork, with full
//! validation. Nothing loaded from outside core becomes an `Artwork` any other way.

use std::collections::HashSet;
use std::sync::Arc;

use crate::artwork::{Artwork, ArtworkState, MAX_LAYERS};
use crate::brush::BrushRecipe;
use crate::error::CoreError;
use crate::id::{IdCounters, LayerId, MAX_ID, StrokeId};
use crate::layer::{Layer, LayerProps};
use crate::parts::{ArtworkParts, LayerParts, RecipeParts, SampleParts, StrokeParts};
use crate::stroke::{MAX_STROKE_SAMPLES, Stroke, StrokeSample, StrokeStyle, check_order};
use crate::stroke_list::StrokeList;
use crate::value::{CanvasPoint, CanvasSize, Color, LayerName, invalid};

/// Ids seen so far, checked against the stored counters, and the recipes built so far: strokes
/// drawn with one brush share one recipe, as they did before they were stored.
struct IdCheck {
    counters: IdCounters,
    layers: HashSet<u64>,
    strokes: HashSet<u64>,
    recipes: Vec<Arc<BrushRecipe>>,
}

impl IdCheck {
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
    /// `EmptyStroke`, `SampleLimit` and `SampleOrder` for stroke samples.
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
        };
        let layers = parts
            .layers
            .iter()
            .map(|layer| layer_from(layer, &mut ids))
            .collect::<Result<Vec<_>, _>>()?;
        let active = LayerId::new(parts.active);
        if !layers.iter().any(|layer| layer.id == active) {
            return Err(CoreError::UnknownLayer(active));
        }
        Ok(Self::from_state(ArtworkState {
            canvas,
            layers,
            active,
            ids: counters,
        }))
    }
}

fn counter(next: u64) -> Result<u64, CoreError> {
    if (1..=MAX_ID).contains(&next) {
        Ok(next)
    } else {
        Err(invalid("id counter"))
    }
}

fn layer_from(parts: &LayerParts, ids: &mut IdCheck) -> Result<Layer, CoreError> {
    let id = ids.layer(parts.id)?;
    let name = LayerName::new(&parts.name)?;
    let props = LayerProps::new(parts.visible, parts.opacity, parts.locked)?;
    let strokes = parts
        .strokes
        .iter()
        .map(|stroke| stroke_from(stroke, ids))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Layer {
        id,
        name,
        props,
        strokes: StrokeList::from_strokes(&strokes),
    })
}

fn stroke_from(parts: &StrokeParts, ids: &mut IdCheck) -> Result<Stroke, CoreError> {
    let id = ids.stroke(parts.id)?;
    let [r, g, b] = parts.color;
    let recipe = ids.recipe(&parts.recipe)?;
    let style = StrokeStyle::new(recipe, parts.size, parts.opacity, Color::rgb(r, g, b)?)?;
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
    Ok(Stroke::new(id, style, Arc::from(samples)))
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
