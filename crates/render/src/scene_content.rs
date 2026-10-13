//! A layer's content as scene ops: strokes oldest first, each region edit after the strokes it
//! was made over, smudges as residuals, and transform records (S31, DECISIONS D57) as the
//! content before them evaluated again where it moved.
//!
//! A record is evaluated by mapping back: a pixel after it takes the content before it at the
//! point the placement's inverse maps the pixel to, inside what moved. For the whole layer the
//! ops before it are simply listed under a frame that maps their points (`Item::Frame`), the
//! frames of later records composed into one map, so nothing is listed twice; the moved
//! content is cut to the canvas at the record's place (`EndMove`, whole). For a selection the
//! content before it shows twice: once where it stays, the selection cleared as an older
//! transform clears its lift, and once moved (`BeginMove`, the same ops under the moved frame
//! and limited to the selection's rectangle, `EndMove` cutting them to the selection and
//! laying them over what stayed). Pencil grain, fill edges, clips and smudge residuals are read
//! at the mapped point, so they move with the content as its pixels would, and the footprint of
//! an output pixel follows the placement's scale, so scaled strokes stay sharp.
//! Filters (S33) read neighbouring pixels: the scene lists where their results apply
//! (`push_filter`).

use openpaint_core::{LayerId, RegionEdit, StrokeBlend, StrokeList, StrokeTool, TransformEdit};

use crate::affine::Affine;
use crate::edits::{edit_rect, mask_bounds, restart};
use crate::error::RenderError;
use crate::filter::{FilterKey, kernel_of};
use crate::geom::{Bounds, PixelRect};
use crate::protocol::{Op, Paint};
use crate::scene::{Item, Scene, SmudgeItem};
use crate::scene_cleared::{Cleared, extents, intersect};
use crate::smudge::whole_reach;
use crate::stroke::{paint, segments};

/// The content of one layer's paint or mask, and where its first item lies in the scene.
struct Content<'a> {
    layer: LayerId,
    strokes: &'a StrokeList,
    edits: &'a [RegionEdit],
    on_mask: bool,
    start: usize,
    /// Where each selection record's moved content can lie, when selections may nest.
    extents: Option<Vec<Option<Bounds>>>,
}

/// Where one level of content is evaluated: canvas points to its pixels and back, and the part
/// of it that can show (its pixels; `None`: all).
#[derive(Debug, Clone, Copy)]
struct View {
    to_content: Affine,
    to_canvas: Affine,
    clip: Option<Bounds>,
}

impl View {
    const CANVAS: Self = Self {
        to_content: Affine::IDENTITY,
        to_canvas: Affine::IDENTITY,
        clip: None,
    };

    /// The content before a transform of `placement`, seen from this view.
    fn before(self, placement: Affine, clip: Option<Bounds>) -> Self {
        let back = placement.inverse();
        Self {
            to_content: self.to_content.then(back),
            to_canvas: placement.then(self.to_canvas),
            clip,
        }
    }

    /// Whether anything inside `rect` (its pixels) can show.
    fn meets(&self, rect: PixelRect) -> bool {
        self.clip.is_none_or(|clip| {
            let [x0, y0, x1, y1] = clip.0;
            (rect.min[0] as f32) <= x1
                && (rect.max[0] as f32) >= x0
                && (rect.min[1] as f32) <= y1
                && (rect.max[1] as f32) >= y0
        })
    }
}

impl Scene {
    /// Strokes with each edit after the strokes it was made over, then the in-progress
    /// stroke's place when `active`. `on_mask` for a mask's ops.
    pub(crate) fn push_content(
        &mut self,
        layer: LayerId,
        (strokes, edits): (&StrokeList, &[RegionEdit]),
        active: bool,
        on_mask: bool,
    ) -> Result<(), RenderError> {
        let content = Content {
            layer,
            strokes,
            edits,
            on_mask,
            start: self.items.len(),
            extents: level_extents(strokes, edits, self.canvas),
        };
        self.push_level(&content, (strokes.len(), edits.len()), View::CANVAS)?;
        self.set_frame(View::CANVAS);
        if active {
            self.active_before = Some(content.start..self.items.len());
            self.items.push(Item::Active);
        }
        Ok(())
    }

    /// The content's strokes before `strokes` and edits before `edits`, seen in `view`: from
    /// the last transform of the whole layer with pixels on, in segments between the whole
    /// layer's records, each under the frame of the records after it.
    fn push_level(
        &mut self,
        content: &Content<'_>,
        (strokes, edits): (usize, usize),
        view: View,
    ) -> Result<(), RenderError> {
        let (first, first_stroke) = restart(&content.edits[..edits]);
        let views = segment_views(&content.edits[first..edits], view);
        let cleared = content.extents.as_ref().map(|extents| {
            let all = (content.strokes, content.edits);
            let bounds = (strokes, edits);
            let known = (extents.as_slice(), view.clip, self.canvas);
            Cleared::new(all, (first_stroke, first), bounds, known)
        });
        let hidden = |index| cleared.as_ref().is_some_and(|c| c.stroke(index));
        let edit_hidden = |at| cleared.as_ref().map_or([false; 2], |c| c.edit(at));
        let mut segment = 0;
        let base = self.items.len();
        self.set_frame(views[0]);
        let mut next = first;
        for index in first_stroke..strokes {
            while next < edits && content.edits[next].after() == index {
                let at = (next, edit_hidden(next));
                self.push_level_edit(content, at, (&views, &mut segment), base)?;
                next += 1;
            }
            if !hidden(index) {
                self.push_level_stroke(content, index, views[segment])?;
            }
        }
        for at in next..edits {
            let at = (at, edit_hidden(at));
            self.push_level_edit(content, at, (&views, &mut segment), base)?;
        }
        Ok(())
    }

    /// Edit `at` of the content, unless it (`hidden[0]`) or its moved content (`hidden[1]`)
    /// is cleared unseen; the level's items start at `base`.
    fn push_level_edit(
        &mut self,
        content: &Content<'_>,
        (at, hidden): (usize, [bool; 2]),
        (views, segment): (&[View], &mut usize),
        base: usize,
    ) -> Result<(), RenderError> {
        let view = views[*segment];
        let edit = &content.edits[at];
        match edit.transform().filter(|t| t.patch().is_none()) {
            Some(transform) if transform.lift().is_none() => {
                // Everything so far moved: cut to the canvas where it was, then the next
                // segment's frame.
                let op = Op::end_move(0, false, self.canvas);
                self.items.push(Item::EndMove { op, whole: true });
                *segment += 1;
                self.set_frame(views[*segment]);
                Ok(())
            }
            Some(transform) => {
                // Clearing what the level does not hold yet clears nothing.
                let empty = self.items[base..].iter().all(|item| {
                    matches!(
                        item,
                        Item::Frame { .. } | Item::Clear { .. } | Item::EndMove { whole: true, .. }
                    )
                });
                let hidden = [hidden[0] || empty, hidden[1]];
                self.push_moved(content, (at, hidden), transform, view)
            }
            None if !hidden[0] && view.meets(edit_rect(self.canvas, edit)) => {
                match (edit.adjust(), edit.nested()) {
                    (Some(adjust), _) if kernel_of(adjust.adjustment()).is_some() => {
                        let place = (content.layer, content.on_mask);
                        let edits = (content.strokes, &content.edits[..=at]);
                        let key = FilterKey::new(place, edits, view.to_canvas);
                        self.push_filter(key, content.start)
                    }
                    (Some(adjust), _) => {
                        let place = (content.layer, content.on_mask, at);
                        self.push_adjust(adjust, place, content.start)
                    }
                    (None, Some(nested)) => self.push_nested(nested.layer(), view),
                    (None, None) => self.push_edit(edit),
                }
            }
            None => Ok(()),
        }
    }

    /// A selection's record: what stays without the selection, then the content before it
    /// again, moved, cut to the selection and laid over what stayed.
    fn push_moved(
        &mut self,
        content: &Content<'_>,
        (at, hidden): (usize, [bool; 2]),
        transform: &TransformEdit,
        view: View,
    ) -> Result<(), RenderError> {
        let Some(lift) = transform.lift() else {
            return Ok(());
        };
        let lifted = mask_bounds(lift);
        if !hidden[0] && view.meets(lifted.pixels()) {
            self.push_lift(lift)?;
        }
        if hidden[1] {
            return Ok(());
        }
        let placement = Affine::of(transform.placement());
        let clip = match view.clip {
            Some(clip) => intersect(lifted, placement.inverse().bounds(clip)),
            None => Some(lifted),
        };
        let Some(clip) = clip else {
            return Ok(());
        };
        let moved = view.before(placement, Some(clip));
        self.items.push(Item::BeginMove(Op::begin_move()));
        self.push_level(content, (transform.after(), at), moved)?;
        self.set_frame(moved);
        let mask = self.clip(lift)?;
        let op = Op::end_move(mask, true, self.canvas);
        self.items.push(Item::EndMove { op, whole: false });
        let alone = Op::end_move(mask, false, self.canvas);
        self.items.push(Item::MovedOnly(alone));
        self.set_frame(view);
        Ok(())
    }

    fn push_level_stroke(
        &mut self,
        content: &Content<'_>,
        index: usize,
        view: View,
    ) -> Result<(), RenderError> {
        let Some(stroke) = content.strokes.get(index) else {
            return Ok(());
        };
        let (style, samples) = (stroke.style(), stroke.samples());
        if style.tool() == StrokeTool::Smudge {
            let reach: Vec<Bounds> = whole_reach(style, samples);
            if !reach.iter().any(|bounds| view.meets(bounds.pixels())) {
                return Ok(());
            }
            let clip = stroke.clip().map(|clip| self.clip(clip)).transpose()?;
            let canvas = reach
                .iter()
                .map(|bounds| view.to_canvas.bounds(*bounds).pixels())
                .reduce(|a, b| a.union(&b))
                .unwrap_or(PixelRect {
                    min: [0, 0],
                    max: [0, 0],
                });
            let item = SmudgeItem {
                layer: content.layer,
                on_mask: content.on_mask,
                stroke: stroke.clone(),
                before: content.start..self.items.len(),
                newest: content.strokes.len().checked_sub(1) == Some(index),
                to_canvas: view.to_canvas,
                item: 0,
                clip: clip.unwrap_or(0),
                canvas,
            };
            self.push_smudge(item, reach);
            return Ok(());
        }
        // Each span's paint op is the stroke's end, right after its last span.
        let spans: Vec<_> = segments(style, samples).collect();
        let parts: Vec<(Op, Bounds)> = (1..=spans.len() as i32)
            .rev()
            .zip(&spans)
            .map(|(paint, segment)| (segment.op(paint), segment.bounds()))
            .collect();
        if !parts.iter().any(|(_, bounds)| view.meets(bounds.pixels())) {
            return Ok(());
        }
        let mut paint = paint(style, stroke.blend());
        paint.clip = stroke
            .clip()
            .map(|clip| self.clip(clip))
            .transpose()?
            .unwrap_or(0);
        self.push_stroke(&paint, parts);
        Ok(())
    }

    fn push_stroke(&mut self, paint: &Paint, parts: Vec<(Op, Bounds)>) {
        for (op, bounds) in parts {
            self.items.push(Item::Segment { op, bounds });
        }
        self.items.push(Item::EndStroke(Op::end_stroke(paint)));
    }

    /// A finished smudge: its reach, then its op, which `place_bases` points at its residual's
    /// base among the scene's smudges.
    fn push_smudge(&mut self, mut item: SmudgeItem, reach: Vec<Bounds>) {
        for bounds in reach {
            self.items.push(Item::Reach(bounds));
        }
        let locked = item.stroke.blend() == StrokeBlend::SmudgeLocked;
        item.item = self.items.len();
        self.items
            .push(Item::Smudge(Op::smudge(Some(0), item.clip, locked)));
        self.smudges.push(item);
    }

    /// The items from here lie in `view`: a frame item when it differs from the last.
    fn set_frame(&mut self, view: View) {
        let frame = (view.to_content, view.clip);
        if self.frame == frame {
            return;
        }
        self.frame = frame;
        let foot = view.to_canvas.scale();
        let op = Op::frame(view.to_content.rows(), foot as f32, foot.log2() as f32);
        self.items.push(Item::Frame {
            op,
            to_canvas: view.to_canvas,
            clip: view.clip,
        });
    }
}

/// Where each selection record's moved content can lie, when selections moved more than once
/// can nest (what later ones clear is left out then).
fn level_extents(
    strokes: &StrokeList,
    edits: &[RegionEdit],
    canvas: [u32; 2],
) -> Option<Vec<Option<Bounds>>> {
    let records = edits.iter().filter(|edit| {
        edit.transform()
            .is_some_and(|t| t.patch().is_none() && t.lift().is_some())
    });
    (records.count() > 1).then(|| extents(strokes, edits, canvas))
}

/// Per segment of `edits` between the whole layer's records, the view its content lies in:
/// the last one `view`, each before it under the record that ends it.
fn segment_views(edits: &[RegionEdit], view: View) -> Vec<View> {
    let mut views = vec![view];
    let wholes = edits.iter().rev().filter_map(|edit| {
        edit.transform()
            .filter(|t| t.patch().is_none() && t.lift().is_none())
    });
    for transform in wholes {
        let last = views[views.len() - 1];
        let placement = Affine::of(transform.placement());
        let clip = last.clip.map(|clip| placement.inverse().bounds(clip));
        views.push(last.before(placement, clip));
    }
    views.reverse();
    views
}

#[path = "scene_nested.rs"]
mod nested;

#[cfg(test)]
#[path = "scene_content_tests.rs"]
mod tests;
