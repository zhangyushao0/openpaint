//! Which scenes the raster core draws (S59, DECISIONS D86, D88, D89): a scene holding an
//! adjustment, a filter, a group, a transform record, a moved selection or a merged layer is
//! evaluated by the evaluator whole, the routing decided per scene as it is made (`Scene::routing`,
//! S62; a layer of the raster core's composited onto one of the evaluator's would need either to
//! hand the other its float layer); every other scene, smudges included since S61, is drawn by
//! the raster core, as `RasterScene`: its items as the units the raster plan lays out, strokes
//! with the box of all their spans, smudges with the canvas pixels they can change. The reasons
//! are named, so the routing is logged and tested, never silent.

use std::fmt;
use std::ops::Range;

use crate::geom::{Bounds, PixelRect};
use crate::protocol::raster::Edit;
use crate::scene::{Item, Scene};

/// Why a scene goes to the evaluator: what it holds that the raster core does not draw yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Adjustment,
    Filter,
    Group,
    Transform,
    MovedSelection,
    Nested,
}

impl Reason {
    pub const ALL: [Self; 6] = [
        Self::Adjustment,
        Self::Filter,
        Self::Group,
        Self::Transform,
        Self::MovedSelection,
        Self::Nested,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Adjustment => "adjustments",
            Self::Filter => "filters",
            Self::Group => "groups",
            Self::Transform => "transform records",
            Self::MovedSelection => "moved selections",
            Self::Nested => "merged layers",
        }
    }
}

/// The reasons a scene holds, at least one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Routed(u8);

impl Routed {
    fn with(self, reason: Reason) -> Self {
        Self(self.0 | 1 << reason as u8)
    }

    #[must_use]
    pub fn contains(self, reason: Reason) -> bool {
        self.0 & 1 << reason as u8 != 0
    }

    /// The reasons, in `Reason::ALL`'s order.
    #[must_use]
    pub fn reasons(self) -> Vec<Reason> {
        (Reason::ALL.into_iter())
            .filter(|reason| self.contains(*reason))
            .collect()
    }
}

impl fmt::Display for Routed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<&str> = self.reasons().into_iter().map(Reason::name).collect();
        f.write_str(&names.join(", "))
    }
}

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
    /// The layer's paint is set aside, its mask's units follow.
    BeginMask,
    /// A layer's or a clipping chain's end.
    End(usize),
    /// Where the stroke being drawn lays (S63): above its layer's finished content (or the next
    /// stroke's place, where nothing is drawn).
    Active,
}

/// A scene as the raster core draws it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RasterScene {
    pub(crate) units: Vec<Unit>,
}

/// How a scene is drawn, decided once as it is made (S62).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Routing {
    /// By the raster core, as these units.
    Raster(RasterScene),
    /// By the evaluator, whole, for these reasons.
    Evaluated(Routed),
}

impl Default for Routing {
    /// An empty scene's: nothing the raster core does not draw.
    fn default() -> Self {
        Self::Raster(RasterScene::default())
    }
}

impl Routing {
    /// How `scene` is drawn.
    pub(crate) fn of(scene: &Scene) -> Self {
        match RasterScene::of(scene) {
            Ok(raster) => Self::Raster(raster),
            Err(routed) => Self::Evaluated(routed),
        }
    }

    /// `Ok` for the raster core, else why the evaluator draws it.
    pub(crate) const fn kind(&self) -> Result<(), Routed> {
        match self {
            Self::Raster(_) => Ok(()),
            Self::Evaluated(routed) => Err(*routed),
        }
    }
}

impl RasterScene {
    /// The units laying the scene's items `items` (one layer's paint or mask before a smudge: its
    /// strokes, edits and smudges), in order: what a smudge's material draws (S62).
    pub(crate) fn within<'a>(
        &'a self,
        items: &'a Range<usize>,
    ) -> impl Iterator<Item = &'a Unit> + 'a {
        self.units.iter().filter(move |unit| match unit {
            Unit::Stroke { paint, .. } => items.contains(paint),
            Unit::Edit { item, .. } | Unit::Smudge { item, .. } => items.contains(item),
            Unit::BeginMask | Unit::End(_) | Unit::Active => false,
        })
    }

    /// The unit the stroke being drawn lays at (S63), when its layer shows.
    pub(crate) fn active(&self) -> Option<usize> {
        self.units.iter().position(|unit| *unit == Unit::Active)
    }

    /// `scene`'s units; `Err` with every reason when it holds what goes to the evaluator.
    pub(crate) fn of(scene: &Scene) -> Result<Self, Routed> {
        let mut routed = Routed::default();
        let mut moved = false;
        let mut units = Vec::new();
        let mut first = None;
        let mut smudges = 0;
        for (index, item) in scene.items.iter().enumerate() {
            let reason = match item {
                Item::Segment { .. } => {
                    first.get_or_insert(index);
                    continue;
                }
                Item::EndStroke(_) => {
                    let start = first.take().unwrap_or(index);
                    units.push(stroke(scene, start..index, index));
                    continue;
                }
                Item::Fill { op, bounds } | Item::Clear { op, bounds } => match op.edit() {
                    Edit::Fill | Edit::Patch => {
                        let (item, bounds) = (index, *bounds);
                        units.push(Unit::Edit { item, bounds });
                        continue;
                    }
                    Edit::Adjust => Reason::Adjustment,
                    Edit::Filter | Edit::Other => Reason::Filter,
                },
                Item::BeginMask(_) | Item::Active => {
                    let begins = matches!(item, Item::BeginMask(_));
                    units.push(if begins {
                        Unit::BeginMask
                    } else {
                        Unit::Active
                    });
                    continue;
                }
                Item::EndLayer { grouped, .. } | Item::EndChain { grouped, .. } if !grouped => {
                    units.push(Unit::End(index));
                    continue;
                }
                // A smudge's reach boxes bin the evaluator's tiles; its unit has their union.
                Item::Reach(_) => continue,
                Item::Smudge(_) => {
                    units.push(smudge(scene, index, smudges));
                    smudges += 1;
                    continue;
                }
                Item::EndLayer { .. } | Item::EndChain { .. } | Item::EndGroup(_) => Reason::Group,
                Item::EndMove { whole: true, .. } => Reason::Transform,
                Item::EndMove { .. } | Item::MovedOnly(_) => Reason::MovedSelection,
                Item::EndNested(_) | Item::NestedOnly(_) => Reason::Nested,
                Item::Frame { .. } | Item::BeginMove(_) => {
                    moved = true;
                    continue;
                }
            };
            routed = routed.with(reason);
        }
        routed_units(units, routed, moved)
    }
}

/// `units`, or `Err` with `routed` when it names a reason; a frame or a move's start comes with
/// its record's end, but `moved` alone still is a transform.
fn routed_units(units: Vec<Unit>, routed: Routed, moved: bool) -> Result<RasterScene, Routed> {
    let routed = if moved && routed == Routed::default() {
        routed.with(Reason::Transform)
    } else {
        routed
    };
    if routed == Routed::default() {
        Ok(RasterScene { units })
    } else {
        Err(routed)
    }
}

/// The smudge whose op is item `item`, the scene's smudge number `number` (its smudges are its
/// smudge items in order), with the canvas pixels it can change.
fn smudge(scene: &Scene, item: usize, number: usize) -> Unit {
    let found = scene.smudges.get(number);
    debug_assert_eq!(found.map(|found| found.item), Some(item));
    let none = PixelRect {
        min: [0, 0],
        max: [0, 0],
    };
    Unit::Smudge {
        item,
        smudge: number,
        reach: found.map_or(none, |found| found.canvas),
    }
}

/// The stroke whose spans are `spans` and whose paint op is item `paint`, with their box.
fn stroke(scene: &Scene, spans: Range<usize>, paint: usize) -> Unit {
    let bounds = scene.items[spans.clone()]
        .iter()
        .filter_map(Item::bounds)
        .filter(|bounds| !bounds.is_empty())
        .reduce(|a, b| {
            let ([a0, a1, a2, a3], [b0, b1, b2, b3]) = (a.0, b.0);
            Bounds([a0.min(b0), a1.min(b1), a2.max(b2), a3.max(b3)])
        });
    Unit::Stroke {
        spans,
        paint,
        bounds,
    }
}

#[cfg(test)]
#[path = "raster_route_tests.rs"]
mod tests;
