//! `Fresh`: the in-progress stroke's part of one evaluation, and `ActivePlace`: where in a scene
//! it draws (`scene.rs`).

use openpaint_core::{ActiveStroke, Artwork, LayerId, RegionMask, StrokeBlend, StrokeTool};

use crate::geom::Bounds;
use crate::protocol::Op;
use crate::stroke::{Path, paint, spans};

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

    /// The place of `active`, else (S56) where the next stroke on `artwork`'s active layer draws
    /// without a selection: a scene keeps that place while no stroke is drawn, so the pen
    /// touching down keeps the scene (the place lists nothing without a stroke, `plan_bin.rs`).
    pub(crate) fn of_request(artwork: &Artwork, active: Option<&ActiveStroke>) -> Self {
        active.map_or_else(
            || Self {
                layer: artwork.active(),
                on_mask: artwork.on_mask(),
                clip: None,
            },
            Self::of,
        )
    }
}

/// The in-progress stroke's ops for one evaluation: `ops[0]` paints it (`EndActive`), the rest
/// are the spans not yet in its coverage texture, then its tail, with their bounds in `bounds`.
#[derive(Debug, Default)]
pub(crate) struct Fresh {
    pub(crate) ops: Vec<Op>,
    pub(crate) bounds: Vec<Bounds>,
    /// They are a stroke's: the scene's place for it lists them (S56: without one it lists
    /// nothing).
    pub(crate) stroke: bool,
}

impl Fresh {
    /// The settled spans of the stroke so far from span `from` on (all of them from 0, when
    /// its coverage starts over or the target keeps none), then its tail: the spans later
    /// samples may still change (`Path::settled`), which are never kept. `clip` is the stroke's
    /// clip as the scene placed it (`Scene::active_clip`).
    pub(crate) fn new(active: &ActiveStroke, from: usize, clip: u32) -> Self {
        let (style, samples) = (active.style(), active.samples());
        if style.tool() == StrokeTool::Smudge {
            // Its residual so far, applied in every tile (`Smudges::advance` extends it).
            let locked = active.blend() == StrokeBlend::SmudgeLocked;
            return Self {
                ops: vec![Op::smudge(None, clip, locked)],
                bounds: Vec::new(),
                stroke: true,
            };
        }
        let mut paint = paint(style, active.blend());
        paint.clip = clip;
        let mut fresh = Self {
            ops: vec![Op::end_active(&paint)],
            bounds: Vec::new(),
            stroke: true,
        };
        let path = Path::of(samples);
        let settled = path.settled();
        for (index, segment) in
            spans(style, samples, &path, from.min(settled)..path.spans()).enumerate()
        {
            let tail = from.min(settled) + index >= settled;
            // Its paint op is `ops[0]`: as many ops back as it lies on.
            let paint = -(fresh.ops.len() as i32);
            fresh.ops.push(if tail {
                segment.tail_op(paint)
            } else {
                segment.op(paint)
            });
            fresh.bounds.push(segment.bounds());
        }
        fresh
    }

    /// These ops and then `OP_RESUME` (S48), whose index it returns: the stroke's layer before
    /// it from a snapshot. It reaches every tile, so it has no bounds.
    pub(crate) fn resuming(mut self) -> (Self, u32) {
        let index = u32::try_from(self.ops.len()).unwrap_or(u32::MAX);
        self.ops.push(Op::resume());
        (self, index)
    }
}
