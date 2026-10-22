//! A scene as the raster core draws it: its items as the units the raster
//! plan lays out, in scene order. Strokes carry the box of all their spans, smudges the canvas
//! pixels they can change; a point adjustment and a filter rewrite the layer (or its mask) so far
//! within their bounds; a frame names the transform record's content the units after it lie in; a
//! moved selection's or a merged layer's content starts a level of its own (`BeginMove`) which its
//! end lays onto the one under it; a group's end fades what its layers did to the canvas.

use std::ops::Range;

use crate::geom::{Bounds, PixelRect};
use crate::protocol::raster::Edit;
use crate::scene::{Item, Scene};

/// One thing the raster core draws, in scene order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Unit {
    /// A stroke: its span items, its paint op's item, and the box all its spans reach.
    Stroke {
        spans: Range<usize>,
        paint: usize,
        bounds: Option<Bounds>,
    },
    /// A fill or a cleared selection (`Edit::Fill`), or a patch, over the layer.
    Edit { item: usize, bounds: Bounds },
    /// The smudge whose op is item `item`, the scene's smudge number `smudge`, over the layer (or
    /// its mask) in the canvas pixels `reach` it can change.
    Smudge {
        item: usize,
        smudge: usize,
        reach: PixelRect,
    },
    /// A point adjustment: the layer (or its mask) so far through it, within `bounds`.
    Adjust { item: usize, bounds: Bounds },
    /// A filter, the scene's filter number `filter`: the layer (or its mask) so far
    /// becomes its result from the arena, within `bounds`.
    Filter {
        item: usize,
        filter: usize,
        bounds: Bounds,
    },
    /// The layer's paint is set aside, its mask's units follow (item `.0`).
    BeginMask(usize),
    /// A layer's or a clipping chain's end.
    End(usize),
    /// A group's end: what its layers left on the canvas faded by its opacity over what lay
    /// under it.
    EndGroup(usize),
    /// The units from here lie in the frame of item `.0`, until the next frame.
    Frame(usize),
    /// A moved selection's or a merged layer's content starts on a level of its own.
    BeginMove(usize),
    /// A move's end (item `.0`): its content cut to its selection and laid over the level under
    /// it, or (a whole layer's) cut to the canvas.
    EndMove(usize),
    /// A merged layer's end (item `.0`): its content through its mask laid onto the level under
    /// it as its layer's end laid it before the merge.
    EndNested(usize),
    /// Where the stroke being drawn lays (item `.0`, `Item::Active`): above its layer's
    /// finished content (or the next stroke's place, where nothing is drawn).
    Active(usize),
}

impl Unit {
    /// The item it names (a stroke: its paint op's).
    fn item(&self) -> usize {
        match self {
            Self::Stroke { paint: item, .. }
            | Self::Edit { item, .. }
            | Self::Smudge { item, .. }
            | Self::Adjust { item, .. }
            | Self::Filter { item, .. }
            | Self::BeginMask(item)
            | Self::End(item)
            | Self::EndGroup(item)
            | Self::Frame(item)
            | Self::BeginMove(item)
            | Self::EndMove(item)
            | Self::EndNested(item)
            | Self::Active(item) => *item,
        }
    }

    /// Every item index it holds moved by `moved` (strokes taken into a kept scene).
    pub(crate) fn renumber(&mut self, moved: impl Fn(&mut usize)) {
        match self {
            Self::Stroke { spans, paint, .. } => {
                moved(&mut spans.start);
                moved(&mut spans.end);
                moved(paint);
            }
            Self::Edit { item, .. }
            | Self::Smudge { item, .. }
            | Self::Adjust { item, .. }
            | Self::Filter { item, .. }
            | Self::BeginMask(item)
            | Self::End(item)
            | Self::EndGroup(item)
            | Self::Frame(item)
            | Self::BeginMove(item)
            | Self::EndMove(item)
            | Self::EndNested(item)
            | Self::Active(item) => moved(item),
        }
    }
}

/// A scene as the raster core draws it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RasterScene {
    pub(crate) units: Vec<Unit>,
}

impl RasterScene {
    /// The units laying the scene's items `items` (one layer's paint or mask before a smudge or a
    /// filter: its strokes, edits, smudges, records and the merged layers in it), in order: what
    /// a material draws. No layer's end, group's end or stroke's place lies among them.
    pub(crate) fn within<'a>(
        &'a self,
        items: &'a Range<usize>,
    ) -> impl Iterator<Item = &'a Unit> + 'a {
        self.units.iter().filter(move |unit| {
            !matches!(unit, Unit::End(_) | Unit::EndGroup(_) | Unit::Active(_))
                && items.contains(&unit.item())
        })
    }

    /// The unit the stroke being drawn lays at, when its layer shows.
    pub(crate) fn active(&self) -> Option<usize> {
        self.units
            .iter()
            .position(|unit| matches!(unit, Unit::Active(_)))
    }

    /// Whether a group's backdrop is held at unit `at`: a layer or chain of a group ended
    /// onto the canvas before it and the group did not end since.
    pub(crate) fn group_open_at(&self, scene: &Scene, at: usize) -> bool {
        let mut open = false;
        for unit in self.units.iter().take(at) {
            match unit {
                Unit::End(item) => open |= opens_group(&scene.items[*item]),
                Unit::EndGroup(_) => open = false,
                _ => {}
            }
        }
        open
    }

    /// `scene`'s units.
    pub(crate) fn of(scene: &Scene) -> Self {
        let mut units = Vec::new();
        let mut first = None;
        let (mut smudges, mut filters) = (0, 0);
        for (index, item) in scene.items.iter().enumerate() {
            let unit = match item {
                Item::Segment { .. } => {
                    first.get_or_insert(index);
                    continue;
                }
                Item::EndStroke(_) => {
                    let start = first.take().unwrap_or(index);
                    stroke_unit(scene, start..index, index)
                }
                Item::Fill { op, bounds } | Item::Clear { op, bounds } => {
                    let (item, bounds) = (index, *bounds);
                    match op.edit() {
                        Edit::Fill | Edit::Patch => Unit::Edit { item, bounds },
                        Edit::Adjust => Unit::Adjust { item, bounds },
                        Edit::Filter => {
                            filters += 1;
                            let filter = filters - 1;
                            Unit::Filter {
                                item,
                                filter,
                                bounds,
                            }
                        }
                        Edit::Other => continue,
                    }
                }
                Item::Smudge(_) => {
                    smudges += 1;
                    smudge(scene, index, smudges - 1)
                }
                Item::BeginMask(_) => Unit::BeginMask(index),
                Item::Active => Unit::Active(index),
                Item::EndLayer { .. } | Item::EndChain { .. } => Unit::End(index),
                Item::EndGroup(_) => Unit::EndGroup(index),
                Item::Frame { .. } => Unit::Frame(index),
                Item::BeginMove(_) => Unit::BeginMove(index),
                Item::EndMove { .. } => Unit::EndMove(index),
                Item::EndNested(_) => Unit::EndNested(index),
            };
            units.push(unit);
        }
        Self { units }
    }
}

/// Whether `item`, a layer's or a chain's end, composites a group's layer onto the canvas: what
/// opens the group.
pub(crate) fn opens_group(item: &Item) -> bool {
    match item {
        Item::EndLayer { op, grouped, .. } => {
            *grouped && op.layer_end().into == crate::protocol::LayerInto::Canvas
        }
        Item::EndChain { grouped, .. } => *grouped,
        _ => false,
    }
}

/// The smudge whose op is item `item`, the scene's smudge number `number` (its smudges are its
/// smudge items in order), with the canvas pixels it can change.
fn smudge(scene: &Scene, item: usize, number: usize) -> Unit {
    let found = scene.smudges.get(number);
    debug_assert_eq!(found.map(|found| found.item), Some(item));
    Unit::Smudge {
        item,
        smudge: number,
        reach: found.map_or(PixelRect::NONE, |found| found.canvas),
    }
}

/// The stroke whose spans are `spans` and whose paint op is item `paint`, with their box.
pub(crate) fn stroke_unit(scene: &Scene, spans: Range<usize>, paint: usize) -> Unit {
    let bounds = scene.items[spans.clone()]
        .iter()
        .filter_map(Item::bounds)
        .filter(|bounds| !bounds.is_empty())
        .reduce(Bounds::union);
    Unit::Stroke {
        spans,
        paint,
        bounds,
    }
}

#[cfg(test)]
#[path = "raster_scene_tests.rs"]
mod tests;
