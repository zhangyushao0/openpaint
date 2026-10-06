//! Copy and paste (S40, Procreate's): what is copied is records, never pixels, so strokes stay
//! the truth. A copy is the content of a new layer that shows exactly what was copied: one
//! nested record (S39, `nested.rs`) per part, then, for a copy made inside a selection, the
//! pixels outside it cleared (`clear.rs`). Copy takes the active layer's paint as it shows
//! through its mask, at full opacity and by Normal; Copy All takes what the visible layers show
//! together over nothing (no paper): each layer a record by its own opacity and mode, a
//! clipping chain a record of its base with its clips atop, a faded group a record by the
//! group's opacity (a pass-through group at full opacity is its layers). Pasting renumbers
//! every record and stroke, so one copy pastes any number of times.

use std::ops::Range;
use std::sync::Arc;

use crate::artwork::Artwork;
use crate::blend::BlendMode;
use crate::clear::ClearEdit;
use crate::command::Applied;
use crate::error::CoreError;
use crate::group::members;
use crate::layer::Layer;
use crate::layer_command::content;
use crate::nested::{MAX_NESTING, NestedEdit, NestedLayer};
use crate::region::{RegionEdit, RegionMask};
use crate::stroke_list::StrokeList;
use crate::value::CanvasSize;

/// What was copied: the edits of a new layer holding no strokes, and the canvas it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Copied {
    canvas: CanvasSize,
    edits: Arc<[RegionEdit]>,
}

impl Copied {
    /// The size of the canvas it was copied from; it pastes onto canvases of this size.
    #[must_use]
    pub const fn canvas(&self) -> CanvasSize {
        self.canvas
    }

    /// Nested records, then perhaps the pixels outside the selection cleared.
    #[must_use]
    pub fn edits(&self) -> &[RegionEdit] {
        &self.edits
    }
}

impl Artwork {
    /// The active layer's paint (`all`: the visible layers together) as a copy, inside `clip`.
    ///
    /// # Errors
    /// `NothingToCopy` when it holds nothing; `TooDeep` when its records would nest more than
    /// `MAX_NESTING` deep; `InvalidValue` when the clip leaves the canvas.
    pub fn copied(&self, all: bool, clip: Option<&RegionMask>) -> Result<Copied, CoreError> {
        let mut edits = self.copy_parts(all)?;
        if let Some(clip) = clip {
            crate::region::check_mask(self.canvas(), clip)?;
            if let Some(outside) = clip.inverted(self.canvas()) {
                edits.push(RegionEdit::Clear(ClearEdit {
                    after: 0,
                    mask: outside,
                }));
            }
        }
        Ok(Copied {
            canvas: self.canvas(),
            edits: Arc::from(edits),
        })
    }

    /// Whether `copied(all, _)` has something to copy.
    #[must_use]
    pub fn copyable(&self, all: bool) -> bool {
        self.copy_parts(all).is_ok()
    }

    /// `copied` on the active layer's paint, above its strokes, its records under new ids.
    ///
    /// # Errors
    /// `InvalidValue` when it was copied from a canvas of another size; `LayerLocked` for a
    /// locked layer.
    pub fn paste(&self, copied: &Copied) -> Result<Applied, CoreError> {
        if copied.canvas != self.canvas() {
            return Err(crate::value::invalid("pasted canvas size"));
        }
        let index = self.editable_layer(self.active())?;
        let mut next = self.state().clone();
        let after = next.layers[index].strokes.len();
        let mut edits = next.layers[index].edits.to_vec();
        for edit in copied.edits.iter() {
            edits.push(match edit {
                RegionEdit::Nested(nested) => RegionEdit::Nested(NestedEdit {
                    after,
                    layer: Arc::new(nested.layer.renumbered(&mut next.ids)),
                }),
                RegionEdit::Clear(clear) => RegionEdit::Clear(ClearEdit {
                    after,
                    mask: clear.mask.clone(),
                }),
                other => other.clone(),
            });
        }
        next.layers[index].edits = Arc::from(edits);
        Ok(content(next))
    }

    /// The records a copy holds before any clip.
    fn copy_parts(&self, all: bool) -> Result<Vec<RegionEdit>, CoreError> {
        let parts = if all {
            self.composite()
        } else {
            let layer = &self.layers()[self.position(self.active())?];
            let part = (!is_empty(layer)).then(|| part(layer, (1.0, BlendMode::Normal), false));
            part.into_iter().map(nested).collect()
        };
        if parts.is_empty() {
            return Err(CoreError::NothingToCopy);
        }
        let deep = parts
            .iter()
            .filter_map(RegionEdit::nested)
            .any(|nested| nested.layer.depth() > MAX_NESTING);
        if deep {
            return Err(CoreError::TooDeep);
        }
        Ok(parts)
    }

    /// The visible layers bottom to top as records composing what they show.
    fn composite(&self) -> Vec<RegionEdit> {
        let layers = self.layers();
        let mut parts = Vec::new();
        let mut at = 0;
        while at < layers.len() {
            let Some(id) = layers[at].props.group else {
                let chain = self.chain(at);
                parts.extend(self.chain_part(chain.clone()));
                at = chain.end;
                continue;
            };
            let range = members(layers, id);
            let inner = self.chains(range.clone());
            match self.group(id).filter(|group| group.visible) {
                Some(group) if group.opacity >= 1.0 => parts.extend(inner),
                Some(group) if !inner.is_empty() => {
                    let first = &layers[range.start];
                    parts.push(nested(NestedLayer {
                        edits: Arc::from(inner),
                        ..part(first, (group.opacity, BlendMode::Normal), false).emptied()
                    }));
                }
                _ => {}
            }
            at = range.end.max(at + 1);
        }
        parts
    }

    /// The clipping chains of the layers in `range`, each a record.
    fn chains(&self, range: Range<usize>) -> Vec<RegionEdit> {
        let mut parts = Vec::new();
        let mut at = range.start;
        while at < range.end {
            let chain = self.chain(at);
            parts.extend(self.chain_part(chain.clone()));
            at = chain.end.max(at + 1);
        }
        parts
    }

    /// One clipping chain (a base alone, or with layers clipping to it) as a record; nothing
    /// for a hidden or empty base, which shows nothing.
    fn chain_part(&self, chain: Range<usize>) -> Option<RegionEdit> {
        let layers = &self.layers()[chain];
        let (base, clips) = layers.split_first()?;
        if !base.props.visible || is_empty(base) {
            return None;
        }
        let shown = |layer: &&Layer| layer.props.visible && !is_empty(layer);
        let clips: Vec<RegionEdit> = clips
            .iter()
            .filter(shown)
            .map(|clip| nested(part(clip, (clip.props.opacity, clip.props.blend), true)))
            .collect();
        let look = (base.props.opacity, base.props.blend);
        if clips.is_empty() {
            return Some(nested(part(base, look, false)));
        }
        let mut edits = vec![nested(part(base, (1.0, BlendMode::Normal), false))];
        edits.extend(clips);
        Some(nested(NestedLayer {
            edits: Arc::from(edits),
            ..part(base, look, false).emptied()
        }))
    }
}

/// The layer's paint and enabled mask as a record compositing by `(opacity, blend)`, atop
/// what lies before it when `clipped`. Its id is the layer's until a paste renumbers it.
fn part(layer: &Layer, (opacity, blend): (f32, BlendMode), clipped: bool) -> NestedLayer {
    NestedLayer {
        id: layer.id,
        opacity,
        blend,
        clipped,
        strokes: layer.strokes.clone(),
        edits: Arc::clone(&layer.edits),
        mask: layer.mask.clone().filter(|mask| mask.enabled),
    }
}

impl NestedLayer {
    /// The same compositing with no content of its own.
    fn emptied(self) -> Self {
        Self {
            strokes: StrokeList::default(),
            edits: Arc::from([]),
            mask: None,
            ..self
        }
    }
}

fn nested(layer: NestedLayer) -> RegionEdit {
    RegionEdit::Nested(NestedEdit {
        after: 0,
        layer: Arc::new(layer),
    })
}

fn is_empty(layer: &Layer) -> bool {
    layer.strokes.is_empty() && layer.edits.is_empty()
}
