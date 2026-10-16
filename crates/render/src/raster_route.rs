//! Which scenes the raster core draws (S59, DECISIONS D86): a scene holding a smudge, an
//! adjustment, a filter, a group, a transform record, a moved selection or a merged layer is
//! evaluated by the evaluator whole, the routing decided per scene (a layer of the raster core's
//! composited onto one of the evaluator's would need either to hand the other its float layer);
//! every other scene is drawn by the raster core, as `RasterScene`: its items as the units the
//! raster plan lays out, strokes with the box of all their spans. The reasons are named, so the
//! routing is logged and tested, never silent.

use std::fmt;
use std::ops::Range;

use crate::geom::Bounds;
use crate::protocol::raster::Edit;
use crate::scene::{Item, Scene};

/// Why a scene goes to the evaluator: what it holds that the raster core does not draw yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Smudge,
    Adjustment,
    Filter,
    Group,
    Transform,
    MovedSelection,
    Nested,
}

impl Reason {
    pub const ALL: [Self; 7] = [
        Self::Smudge,
        Self::Adjustment,
        Self::Filter,
        Self::Group,
        Self::Transform,
        Self::MovedSelection,
        Self::Nested,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Smudge => "smudges",
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
    /// The layer's paint is set aside, its mask's units follow.
    BeginMask,
    /// A layer's or a clipping chain's end.
    End(usize),
}

/// A scene as the raster core draws it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RasterScene {
    pub(crate) units: Vec<Unit>,
}

impl RasterScene {
    /// `scene`'s units; `Err` with every reason when it holds what goes to the evaluator.
    pub(crate) fn of(scene: &Scene) -> Result<Self, Routed> {
        let mut routed = Routed::default();
        let mut moved = false;
        let mut units = Vec::new();
        let mut first = None;
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
                Item::BeginMask(_) => {
                    units.push(Unit::BeginMask);
                    continue;
                }
                Item::EndLayer { grouped, .. } | Item::EndChain { grouped, .. } if !grouped => {
                    units.push(Unit::End(index));
                    continue;
                }
                Item::Active => continue,
                Item::EndLayer { .. } | Item::EndChain { .. } | Item::EndGroup(_) => Reason::Group,
                Item::Reach(_) | Item::Smudge(_) => Reason::Smudge,
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
        // A frame or a move's start comes with its record's end; alone it still is a transform.
        if moved && routed == Routed::default() {
            routed = routed.with(Reason::Transform);
        }
        if routed == Routed::default() {
            Ok(Self { units })
        } else {
            Err(routed)
        }
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
