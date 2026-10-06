//! An adjustment as a scene op (S28, D54): it reaches every pixel of its clip; with no clip on
//! paint, every pixel the layer's ops before it reach (S35: elsewhere the layer is transparent
//! and an adjustment leaves it so), widened by the kernel's longest reach for a filter; on a mask
//! (white where nothing is painted) or under transform records it is unbounded. The curves'
//! tables go into the mask words. A filter (S33, D59) is such an op too, whose result the
//! target's arena holds (`filter.rs`). A preview's next value changes only these ops' values
//! (`retune`), so the scene, its plan and its other words stay.

use std::ops::Range;
use std::sync::Arc;

use openpaint_core::{AdjustEdit, Adjustment, COORD_LIMIT, LayerId};

use crate::affine::Affine;
use crate::edits::mask_bounds;
use crate::error::RenderError;
use crate::filter::{FilterItem, FilterKey, MAX_EXTENT};
use crate::geom::Bounds;
use crate::preview::Retune;
use crate::protocol::{AdjustOn, Op, table_words};
use crate::scene::{Item, Scene};

/// Which adjustment of which content: its layer, whether on the mask, its index in the edits.
pub(crate) type EditPlace = (LayerId, bool, usize);

/// A point adjustment the scene draws: its place, the items of its layer before it, its item,
/// its clip as its op names it and where its curves' tables lie in the mask words.
#[derive(Debug, Clone)]
pub(crate) struct AdjustItem {
    pub(crate) place: EditPlace,
    pub(crate) before: Range<usize>,
    pub(crate) item: usize,
    clip: u32,
    tables: Option<u32>,
}

/// What `Scene::retune` changed: these items' ops, and the mask words in `words`.
#[derive(Debug, Default)]
pub(crate) struct Retuned {
    pub(crate) items: Vec<usize>,
    pub(crate) words: Vec<Range<usize>>,
}

impl Scene {
    /// The point adjustment `adjust` at `place`, whose layer's items start at `start`.
    pub(crate) fn push_adjust(
        &mut self,
        adjust: &AdjustEdit,
        place: EditPlace,
        start: usize,
    ) -> Result<(), RenderError> {
        let clip = adjust.clip().map(|clip| self.clip(clip)).transpose()?;
        let clip = clip.unwrap_or(0);
        let tables = match adjust.adjustment() {
            Adjustment::Curves(curves) => Some(self.push_words(&table_words(&curves.luts()))?),
            _ => None,
        };
        let op = adjust_op(adjust, place.1, clip, tables)?;
        let bounds = self.adjust_bounds(adjust, place.1, (start, 0.0));
        self.adjusts.push(AdjustItem {
            place,
            before: start..self.items.len(),
            item: self.items.len(),
            clip,
            tables,
        });
        self.items.push(Item::Fill { op, bounds });
        Ok(())
    }

    /// The filter `key` names, whose layer's items start at `start`; its op names its block once
    /// `place_bases` has run. Shown again in content a selection's record moved, it takes its
    /// first place's key and items.
    pub(crate) fn push_filter(&mut self, key: FilterKey, start: usize) -> Result<(), RenderError> {
        let Some(adjust) = key.edit().cloned() else {
            return Err(RenderError::Invalid("a filter item holds no filter"));
        };
        let clip = adjust.clip().map(|clip| self.clip(clip)).transpose()?;
        let first = self
            .filters
            .iter()
            .find(|item| item.key.place() == key.place());
        let (key, before) = match first {
            Some(first) => (Arc::clone(&first.key), first.before.clone()),
            None => (Arc::new(key), start..self.items.len()),
        };
        let bounds = self.adjust_bounds(&adjust, key.on_mask, (before.start, MAX_EXTENT as f32));
        let item = FilterItem {
            key,
            before,
            item: self.items.len(),
            clip: clip.unwrap_or(0),
        };
        self.items.push(Item::Fill {
            op: Op::filter(0, item.clip, self.canvas),
            bounds,
        });
        self.filters.push(item);
        Ok(())
    }

    /// Shows `retune`'s values for the adjustment at its place: its ops and tables anew, or for
    /// a filter its key (its result is prepared again). Everything else stays.
    ///
    /// # Errors
    /// `Invalid` for values of another kind than the scene's.
    pub(crate) fn retune(&mut self, retune: &Retune) -> Result<Retuned, RenderError> {
        let mut retuned = Retuned::default();
        for at in 0..self.adjusts.len() {
            let adjust = self.adjusts[at].clone();
            if adjust.place != retune.place {
                continue;
            }
            let op = adjust_op(&retune.edit, retune.place.1, adjust.clip, adjust.tables)?;
            if let (Some(tables), Adjustment::Curves(curves)) =
                (adjust.tables, retune.edit.adjustment())
            {
                let words = table_words(&curves.luts());
                let from = tables as usize;
                self.masks[from..from + words.len()].copy_from_slice(&words);
                retuned.words.push(from..from + words.len());
            }
            if let Item::Fill { op: shown, .. } = &mut self.items[adjust.item] {
                *shown = op;
            }
            retuned.items.push(adjust.item);
        }
        let mut key = None;
        for filter in &mut self.filters {
            if filter.key.place() == retune.place {
                let new = key.get_or_insert_with(|| Arc::new(filter.key.retuned(&retune.edit)));
                filter.key = Arc::clone(new);
            }
        }
        Ok(retuned)
    }

    /// The items of the layer before the adjustment at `place` and its own item, when the scene
    /// draws it once with no transform record among them (what a snapshot can stand for).
    pub(crate) fn prefix_of(&self, place: EditPlace) -> Option<Range<usize>> {
        let points = self.adjusts.iter().filter(|a| a.place == place);
        let filters = self.filters.iter().filter(|f| f.key.place() == place);
        let mut found = points
            .map(|a| a.before.start..a.item)
            .chain(filters.map(|f| f.before.start..f.item));
        let prefix = found.next()?;
        if found.next().is_some() || !self.pure(prefix.clone()) {
            return None;
        }
        Some(prefix)
    }

    /// Whether `items` lie on the canvas itself: no transform record moves
    /// them and the scene places nothing in progress there.
    fn pure(&self, items: Range<usize>) -> bool {
        let moved = |item: &Item| {
            matches!(
                item,
                Item::Frame { .. }
                    | Item::BeginMove(_)
                    | Item::EndMove { .. }
                    | Item::MovedOnly(_)
                    | Item::EndNested(_)
                    | Item::NestedOnly(_)
                    | Item::Active
            )
        };
        let canvas = self.frame == (Affine::IDENTITY, None);
        canvas && !self.items[items].iter().any(moved)
    }

    /// Where the adjustment `adjust` can change pixels: its clip's rectangle; on paint with no
    /// clip, where the items from `start` on reach, widened by `reach`; else unbounded.
    fn adjust_bounds(
        &self,
        adjust: &AdjustEdit,
        on_mask: bool,
        (start, reach): (usize, f32),
    ) -> Bounds {
        const ALL: Bounds = Bounds([-COORD_LIMIT, -COORD_LIMIT, COORD_LIMIT, COORD_LIMIT]);
        if let Some(clip) = adjust.clip() {
            return mask_bounds(clip);
        }
        if on_mask || !self.pure(start..self.items.len()) {
            return ALL;
        }
        let union = self.items[start..]
            .iter()
            .filter_map(Item::bounds)
            .filter(|bounds| !bounds.is_empty())
            .map(|bounds| bounds.0)
            .reduce(|a, b| {
                [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ]
            });
        // Over nothing it changes nothing: bounds that reach no pixel.
        union.map_or(Bounds::NOTHING, |[x0, y0, x1, y1]| {
            Bounds([x0 - reach, y0 - reach, x1 + reach, y1 + reach])
        })
    }

    /// Appends `words` to the mask words; returns where they start.
    fn push_words(&mut self, words: &[u32]) -> Result<u32, RenderError> {
        let at = u32::try_from(self.masks.len())
            .ok()
            .filter(|at| *at < u32::MAX)
            .ok_or(RenderError::Invalid("the masks exceed 2^32 words"))?;
        self.masks.extend_from_slice(words);
        Ok(at)
    }
}

/// The op of the point adjustment `adjust` on paint or (`on_mask`) a mask, inside the clip its
/// op names as `clip`, its curves' tables at `tables`.
fn adjust_op(
    adjust: &AdjustEdit,
    on_mask: bool,
    clip: u32,
    tables: Option<u32>,
) -> Result<Op, RenderError> {
    let on = match (on_mask, adjust.inverted()) {
        (false, _) => AdjustOn::Paint,
        (true, false) => AdjustOn::Mask,
        (true, true) => AdjustOn::InvertedMask,
    };
    match (adjust.adjustment(), tables) {
        (Adjustment::Hsb(hsb), _) => Ok(Op::adjust_hsb(
            [hsb.hue, hsb.saturation, hsb.brightness],
            on,
            clip,
        )),
        (Adjustment::Curves(_), Some(at)) => Ok(Op::adjust_curves(at, on, clip)),
        _ => Err(RenderError::Invalid("a filter is no point operation")),
    }
}
