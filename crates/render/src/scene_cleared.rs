//! What a later selection record clears (S31). A selection's transform record shows the
//! content before it twice, where it stays and where it moved, so selections moved again and
//! again would list that content twice per record, nested: 2^n times for n records (measured:
//! 13 items, then 30, 64, ... 4,348 after eight nudges). But a selection's record first clears
//! its selection, so an item of one level that lies wholly inside a later selection of the same
//! level shows nowhere, unless a smudge or a filter between reads it or a record of the whole
//! layer between moves that selection's frame: such items are left out, an earlier record's
//! moved content too, judged by where that content can lie (`extents`). Nudging the same
//! selection n times then lists its content about n times.

use openpaint_core::{RegionEdit, RegionMask, Stroke, StrokeList, StrokeTool};

use crate::affine::Affine;
use crate::edits::{edit_rect, mask_bounds, moved_rect, placed_rect};
use crate::filter::{self, kernel_of};
use crate::geom::Bounds;
use crate::smudge::whole_reach;
use crate::stroke::segments;

/// Per edit of a layer's content: for a selection's record, where its moved content can lie
/// just after it, in the layer's pixels then; `None` for other edits and when nothing moved.
/// Followed through the content as rectangles: each stroke's and fill's, moved and cleared by
/// the transforms after them.
pub(crate) fn extents(
    strokes: &StrokeList,
    edits: &[RegionEdit],
    canvas: [u32; 2],
) -> Vec<Option<Bounds>> {
    let mut alive: Vec<Alive> = Vec::new();
    let mut out = vec![None; edits.len()];
    let mut next = 0;
    for index in 0..=strokes.len() {
        while next < edits.len() && edits[next].after() == index {
            out[next] = follow(&mut alive, &edits[next], canvas);
            next += 1;
        }
        if let Some(bounds) = strokes.get(index).and_then(reach) {
            alive.push(Alive::at(bounds));
        }
    }
    out
}

/// What may still show of one stroke or fill: its bounds where it was made, and where the
/// transforms since have moved them (composed, so moving again never widens them twice).
#[derive(Clone, Copy)]
struct Alive {
    bounds: Bounds,
    moved: Affine,
}

impl Alive {
    fn at(bounds: Bounds) -> Self {
        Self {
            bounds,
            moved: Affine::IDENTITY,
        }
    }

    fn now(self) -> Bounds {
        if self.moved.is_identity() {
            self.bounds
        } else {
            self.moved.bounds(self.bounds)
        }
    }
}

/// `alive` after `edit`; for a selection's record, where its moved content lies.
fn follow(alive: &mut Vec<Alive>, edit: &RegionEdit, canvas: [u32; 2]) -> Option<Bounds> {
    let Some(transform) = edit.transform() else {
        if let Some(fill) = edit.fill() {
            alive.push(Alive::at(widened(mask_bounds(fill.mask()))));
        }
        if edit.nested().is_some() {
            // A merged layer's content: anywhere on the canvas, as its edit's rectangle says.
            alive.push(Alive::at(Bounds([
                0.0,
                0.0,
                canvas[0] as f32,
                canvas[1] as f32,
            ])));
        }
        if let Some(reach) = filter_reach(edit) {
            // A filter spreads what is there by its kernel's reach.
            for item in alive.iter_mut() {
                let Bounds([x0, y0, x1, y1]) = item.now();
                *item = Alive::at(Bounds([x0 - reach, y0 - reach, x1 + reach, y1 + reach]));
            }
        }
        return None;
    };
    let placement = Affine::of(transform.placement());
    let lift = transform.lift();
    let clear = |alive: &mut Vec<Alive>| match lift {
        Some(lift) => alive.retain(|item| !covers(lift, item.now(), canvas)),
        None => alive.clear(),
    };
    if transform.patch().is_some() {
        clear(alive);
        let placed = placed_rect(moved_rect(canvas, transform), transform.placement());
        let [x0, y0] = placed.min.map(|v| v as f32);
        let [x1, y1] = placed.max.map(|v| v as f32);
        alive.push(Alive::at(Bounds([x0, y0, x1, y1])));
        return None;
    }
    let within = match lift {
        Some(lift) => mask_bounds(lift),
        None => Bounds([0.0, 0.0, canvas[0] as f32, canvas[1] as f32]),
    };
    let mut extent = None;
    let mut moved = Vec::new();
    for item in alive.iter() {
        if let Some(part) = intersect(item.now(), within) {
            let placed = placement.bounds(part);
            extent = Some(extent.map_or(placed, |all| union(all, placed)));
            moved.push(Alive {
                bounds: item.bounds,
                moved: item.moved.then(placement),
            });
        }
    }
    clear(alive);
    alive.extend(moved);
    lift.and(extent)
}

/// For one level of content (strokes from `strokes.0`, edits from `edits.0`, up to the
/// second of each), of which only what lies in `clip` can show: which items a later selection
/// of the level clears unseen.
pub(crate) struct Cleared {
    strokes: Vec<bool>,
    /// Per edit: the edit itself (for a selection's record, its clearing), and a selection
    /// record's moved content.
    edits: Vec<[bool; 2]>,
}

impl Cleared {
    pub(crate) fn new(
        (all, edits): (&StrokeList, &[RegionEdit]),
        (first_stroke, first): (usize, usize),
        (stroke_end, edit_end): (usize, usize),
        (extents, clip, canvas): (&[Option<Bounds>], Option<Bounds>, [u32; 2]),
    ) -> Self {
        let mut cleared = Self {
            strokes: vec![false; stroke_end],
            edits: vec![[false; 2]; edit_end],
        };
        let mut lifts: Vec<&RegionMask> = Vec::new();
        let mut next = edit_end;
        let hidden = |lifts: &[&RegionMask], bounds: Bounds| {
            let shown = match clip {
                Some(clip) => intersect(bounds, clip),
                None => Some(bounds),
            };
            !lifts.is_empty()
                && shown.is_none_or(|shown| lifts.iter().any(|lift| covers(lift, shown, canvas)))
        };
        for index in (first_stroke..=stroke_end).rev() {
            if let Some(stroke) = all.get(index).filter(|_| index < stroke_end) {
                if stroke.style().tool() == StrokeTool::Smudge {
                    lifts.clear();
                } else if let Some(bounds) = reach(stroke) {
                    cleared.strokes[index] = hidden(&lifts, bounds);
                }
            }
            while next > first && edits[next - 1].after() == index {
                next -= 1;
                let edit = &edits[next];
                match edit.transform().filter(|t| t.patch().is_none()) {
                    Some(transform) => match transform.lift() {
                        Some(lift) => {
                            let moved = extents[next].is_none_or(|e| hidden(&lifts, e));
                            let clearing = hidden(&lifts, mask_bounds(lift));
                            cleared.edits[next] = [clearing, moved];
                            lifts.push(lift);
                        }
                        None => lifts.clear(),
                    },
                    None => {
                        let rect = edit_rect(canvas, edit);
                        let [x0, y0] = rect.min.map(|v| v as f32);
                        let [x1, y1] = rect.max.map(|v| v as f32 - 1.0);
                        cleared.edits[next][0] = hidden(&lifts, Bounds([x0, y0, x1, y1]));
                        if filter_reach(edit).is_some() {
                            // A filter reads its neighbours, as a smudge does.
                            lifts.clear();
                        }
                    }
                }
            }
        }
        cleared
    }

    pub(crate) fn stroke(&self, index: usize) -> bool {
        self.strokes.get(index).copied().unwrap_or(false)
    }

    /// Whether edit `index` itself is cleared unseen, and (a selection's record) its moved
    /// content.
    pub(crate) fn edit(&self, index: usize) -> [bool; 2] {
        self.edits.get(index).copied().unwrap_or([false; 2])
    }
}

/// Where a stroke can paint: its segments' bounds (a smudge's reach); `None` for an eraser,
/// which adds nothing.
fn reach(stroke: &Stroke) -> Option<Bounds> {
    let (style, samples) = (stroke.style(), stroke.samples());
    match style.tool() {
        StrokeTool::Erase => None,
        StrokeTool::Smudge => whole_reach(style, samples).into_iter().reduce(union),
        StrokeTool::Paint => segments(style, samples)
            .map(|segment| segment.bounds())
            .reduce(union),
    }
}

/// Whether every canvas pixel `bounds` meets lies in `mask` (pixels past the canvas never show).
pub(crate) fn covers(mask: &RegionMask, bounds: Bounds, [width, height]: [u32; 2]) -> bool {
    let rect = bounds.pixels();
    let clamp = |value: i32, limit: u32| value.clamp(0, i32::try_from(limit).unwrap_or(i32::MAX));
    let (x0, y0) = (clamp(rect.min[0], width), clamp(rect.min[1], height));
    let (x1, y1) = (clamp(rect.max[0], width), clamp(rect.max[1], height));
    if x1 <= x0 || y1 <= y0 {
        return true;
    }
    let ([ox, oy], [w, h]) = (mask.origin(), mask.size());
    let [x0, y0, x1, y1] = [x0, y0, x1, y1].map(|v| v as u32);
    if x0 < ox || y0 < oy || x1 > ox + w || y1 > oy + h {
        return false;
    }
    let stride = mask.stride() as usize;
    (y0 - oy..y1 - oy).all(|row| {
        let words = &mask.words()[row as usize * stride..][..stride];
        full(words, x0 - ox, x1 - ox)
    })
}

/// Whether bits `from..to` of a row are all set.
fn full(words: &[u32], from: u32, to: u32) -> bool {
    let mut x = from;
    while x < to {
        let start = x % 32;
        let span = (to - x).min(32 - start);
        let bits = if span == 32 {
            u32::MAX
        } else {
            ((1u32 << span) - 1) << start
        };
        if words[(x / 32) as usize] & bits != bits {
            return false;
        }
        x += span;
    }
    true
}

/// How far a filter edit spreads content, in pixels; `None` for other edits.
fn filter_reach(edit: &RegionEdit) -> Option<f32> {
    let adjust = edit.adjust()?;
    kernel_of(adjust.adjustment()).map(|(sigma, _)| filter::reach(sigma) as f32)
}

fn widened(Bounds([x0, y0, x1, y1]): Bounds) -> Bounds {
    Bounds([x0 - 1.0, y0 - 1.0, x1 + 1.0, y1 + 1.0])
}

fn union(Bounds(a): Bounds, Bounds(b): Bounds) -> Bounds {
    Bounds([
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ])
}

/// The overlap of two bounds, `None` when they do not meet.
pub(crate) fn intersect(Bounds(a): Bounds, Bounds(b): Bounds) -> Option<Bounds> {
    let overlap = [
        a[0].max(b[0]),
        a[1].max(b[1]),
        a[2].min(b[2]),
        a[3].min(b[3]),
    ];
    (overlap[0] <= overlap[2] && overlap[1] <= overlap[3]).then_some(Bounds(overlap))
}
