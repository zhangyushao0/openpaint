//! `Scene`: an artwork flattened into ops in drawing order, with the bounds each segment can
//! reach, and `Fresh`: the in-progress stroke's part of one evaluation. The geometry of one
//! stroke is in `stroke.rs`.

use std::ops::Range;
use std::sync::Arc;

use openpaint_core::{
    ActiveStroke, Artwork, Layer, LayerGroup, LayerId, Patch, RegionMask, Stroke,
};

use crate::affine::Affine;
use crate::error::RenderError;
use crate::filter::FilterItem;
use crate::geom::Bounds;
use crate::protocol::{LayerInto, MASK_HEADER, Op};
use crate::scene_adjust::AdjustItem;
use crate::scene_layer::LayerOps;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Item {
    Segment {
        op: Op,
        bounds: Bounds,
    },
    EndStroke(Op),
    /// A region edit's op over its layer within `bounds`: a fill, a transform's lift (an
    /// erasing fill) or its patch.
    Fill {
        op: Op,
        bounds: Bounds,
    },
    /// A layer's end: onto the canvas, or (`chain`) starting or joining a clipping chain;
    /// `grouped` inside a group.
    EndLayer {
        op: Op,
        chain: bool,
        grouped: bool,
    },
    /// A clipping chain's end, onto the canvas.
    EndChain {
        op: Op,
        grouped: bool,
    },
    /// A group's end.
    EndGroup(Op),
    /// A masked layer's paint is set aside; its mask's ops follow (S24).
    BeginMask(Op),
    /// Where the in-progress stroke draws: above its layer's finished strokes. Its ops are the
    /// evaluation's `Fresh` ops, which change every frame while the scene stays.
    Active,
    /// Pixels a smudge can change, one box per pair of samples; listed nowhere itself.
    Reach(Bounds),
    /// A smudge's residual applied to its layer, in every tile its reach touched.
    Smudge(Op),
    /// The items after it, up to the next frame, lie in a transform record's earlier content
    /// (S31): their bounds are its pixels, `to_canvas` maps them to the canvas, and only what
    /// lies in `clip` (its pixels; `None`: all) can show. Its op moves where they are evaluated.
    Frame {
        op: Op,
        to_canvas: Affine,
        clip: Option<Bounds>,
    },
    /// A moved selection's content starts; the layer so far is set aside. Listed only where
    /// the layer had something to set aside: elsewhere the moved content starts from nothing
    /// on its own (`MovedOnly`).
    BeginMove(Op),
    /// A moved content's end: cut to its selection and laid over what was set aside, or
    /// (`whole`) cut to the canvas, for a whole layer's.
    EndMove {
        op: Op,
        whole: bool,
    },
    /// The same selection's end where nothing was set aside: only cut to the selection.
    MovedOnly(Op),
    /// What a transform lifted cleared from the layer: an erasing fill, which never adds
    /// anything to set aside.
    Clear {
        op: Op,
        bounds: Bounds,
    },
}

impl Item {
    /// For `Active` and `Reach`, an op no plan refers to, so item and op indices stay equal.
    pub(crate) fn op(&self) -> Op {
        match self {
            Self::Segment { op, .. }
            | Self::Fill { op, .. }
            | Self::EndStroke(op)
            | Self::EndLayer { op, .. }
            | Self::EndChain { op, .. }
            | Self::EndGroup(op)
            | Self::BeginMask(op)
            | Self::Smudge(op)
            | Self::Frame { op, .. }
            | Self::BeginMove(op)
            | Self::EndMove { op, .. }
            | Self::MovedOnly(op)
            | Self::Clear { op, .. } => *op,
            Self::Active | Self::Reach(_) => Op::unused(),
        }
    }

    /// Where it can reach, for the items that carry bounds.
    pub(crate) const fn bounds(&self) -> Option<Bounds> {
        match self {
            Self::Segment { bounds, .. }
            | Self::Fill { bounds, .. }
            | Self::Clear { bounds, .. }
            | Self::Reach(bounds) => Some(*bounds),
            _ => None,
        }
    }
}

/// A finished smudge the scene draws, in scene order: its residual must be in the target's
/// arena before the scene is evaluated.
#[derive(Debug, Clone)]
pub(crate) struct SmudgeItem {
    pub(crate) layer: LayerId,
    /// It smudges the layer's mask, not its paint.
    pub(crate) on_mask: bool,
    pub(crate) stroke: Stroke,
    /// The items of its layer before it: the pixels it carries. A smudge a transform record
    /// shows again, in the content it moved, takes the residual of its first place.
    pub(crate) before: Range<usize>,
    /// It is its layer's newest stroke.
    pub(crate) newest: bool,
    /// Where its pixels lie on the canvas: the identity, but under transform records.
    pub(crate) to_canvas: Affine,
    /// Its item and its clip as its op names it.
    pub(crate) item: usize,
    pub(crate) clip: u32,
}

/// Bottom layer first; inside a layer, finished strokes oldest first with each fill after the
/// strokes it was made over, then the in-progress stroke's place. `masks` holds the fills'
/// masks and the clips of strokes drawn inside a selection (`MASK_HEADER` words each, then
/// their rows), which the fill and paint ops point into; strokes in a row sharing one clip
/// share its words.
#[derive(Debug, Default)]
pub(crate) struct Scene {
    /// The canvas's width and height: a fill's smoothed edge continues past them (S30).
    pub(crate) canvas: [u32; 2],
    pub(crate) items: Vec<Item>,
    pub(crate) masks: Vec<u32>,
    /// The in-progress stroke's clip as its paint op names it (0: none).
    pub(crate) active_clip: u32,
    /// The clip pushed last and its paint op value.
    pub(crate) last_clip: Option<(RegionMask, u32)>,
    /// The transforms' patches in the patch words, in order, and how many words they take.
    pub(crate) patches: Vec<Patch>,
    pub(crate) patch_words: usize,
    /// The finished smudges and the filters, and where in the mask words their arena blocks'
    /// bases are written (one word each, smudges then filters, in order, after every other
    /// word; filled in after the scene is uploaded).
    pub(crate) smudges: Vec<SmudgeItem>,
    pub(crate) filters: Vec<FilterItem>,
    /// The point adjustments, in scene order (S35: a preview changes their values in place).
    pub(crate) adjusts: Vec<AdjustItem>,
    pub(crate) bases_at: u32,
    /// While building: the frame the items pushed now lie in (`Item::Frame`).
    pub(crate) frame: (Affine, Option<Bounds>),
    /// The items of the in-progress stroke's layer before its place.
    pub(crate) active_before: Option<Range<usize>>,
    /// The fills' edge values, in order; and while building, an earlier scene's, which a fill
    /// of the same mask takes instead of deriving them again.
    pub(crate) edges: Vec<EdgeValues>,
    pub(crate) known: Vec<EdgeValues>,
}

/// A fill's smoothed edge values (`fill_edge::edge_words`), derived once for its mask and kept
/// with the scenes that draw it.
#[derive(Debug, Clone)]
pub(crate) struct EdgeValues {
    pub(crate) mask: RegionMask,
    pub(crate) words: Arc<[u32]>,
}

/// Where the in-progress stroke draws: its layer (its paint or its mask), and the selection it
/// is drawn inside.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ActivePlace {
    pub(crate) layer: LayerId,
    pub(crate) on_mask: bool,
    pub(crate) clip: Option<RegionMask>,
}

impl ActivePlace {
    pub(crate) fn of(active: &ActiveStroke) -> Self {
        Self {
            layer: active.layer(),
            on_mask: active.on_mask(),
            clip: active.clip().cloned(),
        }
    }
}

impl Scene {
    /// `build_knowing` with no earlier scene.
    #[cfg(test)]
    pub(crate) fn build(
        artwork: &Artwork,
        active: Option<&ActivePlace>,
    ) -> Result<Self, RenderError> {
        Self::build_knowing(artwork, active, Vec::new())
    }

    /// The finished content of `artwork`, with the place of an in-progress stroke `active`.
    /// Layers that do not show (hidden, in a hidden group, clipping to a hidden base), and
    /// layers with neither content nor that place, contribute no ops. A layer ends onto the
    /// canvas, or into its clipping chain, which ends onto the canvas by its base's opacity and
    /// mode; a group's layers end onto the canvas too, and its own end fades them (D50). A fill
    /// whose mask an earlier scene drew takes its edge values from `known`.
    pub(crate) fn build_knowing(
        artwork: &Artwork,
        active: Option<&ActivePlace>,
        known: Vec<EdgeValues>,
    ) -> Result<Self, RenderError> {
        if let Some(place) = active
            && artwork.layer(place.layer).is_none()
        {
            return Err(RenderError::Invalid(
                "the active stroke's layer is not in the artwork",
            ));
        }
        let mut scene = Self::on(artwork);
        scene.known = known;
        let layers = artwork.layers();
        let drawn = drawn(artwork);
        let ops = shown_ops;
        let mut base = 0;
        for (index, layer) in layers.iter().enumerate() {
            let (props, clipped) = (layer.props(), artwork.clips_at(index));
            let chained = clipped || artwork.clips_at(index + 1);
            base = if clipped { base } else { index };
            let grouped = props.group().is_some();
            if drawn[index] {
                let place = active.filter(|place| place.layer == layer.id());
                if let Some(clip) = place.and_then(|place| place.clip.as_ref()) {
                    scene.active_clip = scene.clip(clip)?;
                }
                let into = match (clipped, chained) {
                    (true, _) => LayerInto::Clipped,
                    (false, true) => LayerInto::Base,
                    (false, false) => LayerInto::Canvas,
                };
                let op = Op::end_layer_into(props.opacity(), into, props.blend().into(), grouped);
                let end = Item::EndLayer {
                    op,
                    chain: chained,
                    grouped,
                };
                scene.push_layer(&ops(layer), place.map(|place| place.on_mask), end)?;
            }
            if chained && !artwork.clips_at(index + 1) && drawn[base] {
                let base = layers[base].props();
                let op = Op::end_chain(base.opacity(), base.blend().into(), grouped);
                scene.items.push(Item::EndChain { op, grouped });
            }
            let next = layers.get(index + 1).and_then(|next| next.props().group());
            let group = props.group().filter(|group| next != Some(*group));
            if let Some(group) = group.and_then(|group| artwork.group(group))
                && group.visible()
            {
                scene
                    .items
                    .push(Item::EndGroup(Op::end_group(group.opacity())));
            }
        }
        scene.known.clear();
        scene.place_bases()?;
        Ok(scene)
    }

    /// `layer` of `artwork` alone, its paint at full opacity without its mask, whether it is
    /// shown or not: what a fill samples and a transform lifts. With `mask`, its mask's
    /// strokes and fills alone instead, which over white show its grey (not inverted): what a
    /// fill on the mask samples.
    pub(crate) fn layer(
        artwork: &Artwork,
        layer: LayerId,
        mask: bool,
    ) -> Result<Self, RenderError> {
        let layer = artwork
            .layer(layer)
            .ok_or(RenderError::Invalid("the layer is not in the artwork"))?;
        let mut scene = Self::on(artwork);
        let end = Item::EndLayer {
            op: Op::end_layer(1.0),
            chain: false,
            grouped: false,
        };
        match layer.mask().filter(|_| mask) {
            Some(mask) => {
                let content = (mask.strokes(), mask.edits());
                scene.push_content(layer.id(), content, false, true)?;
                if !scene.items.is_empty() {
                    scene.items.push(end);
                }
            }
            None => scene.push_layer(&LayerOps::of(layer, None), None, end)?,
        }
        scene.place_bases()?;
        Ok(scene)
    }

    /// An empty scene on `artwork`'s canvas.
    fn on(artwork: &Artwork) -> Self {
        let canvas = artwork.canvas();
        Self {
            canvas: [canvas.width(), canvas.height()],
            ..Self::default()
        }
    }

    /// Appends `mask` to the mask words; returns where it starts.
    pub(crate) fn push_mask(&mut self, mask: &RegionMask) -> Result<u32, RenderError> {
        // Below 2^32 - 1, so a clip's value (one more) fits too.
        let at = u32::try_from(self.masks.len())
            .ok()
            .filter(|at| *at < u32::MAX)
            .ok_or(RenderError::Invalid("the masks exceed 2^32 words"))?;
        let ([x, y], [width, height]) = (mask.origin(), mask.size());
        self.masks.extend([x, y, width, height]);
        self.masks.extend_from_slice(mask.words());
        debug_assert_eq!(MASK_HEADER, 4);
        Ok(at)
    }

    /// The paint op value of the clip `mask`: one more than where its words start, shared with
    /// the clip pushed last when that is the same mask.
    pub(crate) fn clip(&mut self, mask: &RegionMask) -> Result<u32, RenderError> {
        if let Some((last, value)) = &self.last_clip
            && last.shares(mask)
        {
            return Ok(*value);
        }
        let value = self.push_mask(mask)? + 1;
        self.last_clip = Some((mask.clone(), value));
        Ok(value)
    }

    /// Whether the in-progress stroke has a place, that is, its layer is visible.
    pub(crate) fn has_active(&self) -> bool {
        self.items.contains(&Item::Active)
    }
}

/// What a shown layer draws: its paint, and its mask while that is enabled.
fn shown_ops(layer: &Layer) -> LayerOps<'_> {
    LayerOps::of(layer, layer.mask().filter(|mask| mask.enabled()))
}

/// Per layer bottom to top, whether it shows: its eye and its group's are open, and a layer
/// clipping to a base needs that base to show.
fn drawn(artwork: &Artwork) -> Vec<bool> {
    let mut drawn: Vec<bool> = Vec::with_capacity(artwork.layers().len());
    let mut base = false;
    for (index, layer) in artwork.layers().iter().enumerate() {
        let group = layer.props().group().and_then(|group| artwork.group(group));
        let shown = layer.props().visible() && group.is_none_or(LayerGroup::visible);
        if artwork.clips_at(index) {
            drawn.push(shown && base);
        } else {
            base = shown;
            drawn.push(shown);
        }
    }
    drawn
}

#[path = "scene_fresh.rs"]
mod fresh;
pub(crate) use fresh::Fresh;

#[cfg(test)]
#[path = "scene_tests.rs"]
mod tests;
