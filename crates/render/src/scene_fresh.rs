//! `Fresh`: the in-progress stroke's part of one evaluation (`scene.rs`).

use openpaint_core::{ActiveStroke, StrokeBlend, StrokeTool};

use crate::geom::Bounds;
use crate::protocol::Op;
use crate::stroke::{Path, paint, spans};

/// The in-progress stroke's ops for one evaluation: `ops[0]` paints it (`EndActive`), the rest
/// are the spans not yet in its coverage texture, then its tail, with their bounds in `bounds`.
#[derive(Debug, Default)]
pub(crate) struct Fresh {
    pub(crate) ops: Vec<Op>,
    pub(crate) bounds: Vec<Bounds>,
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
            };
        }
        let mut paint = paint(style, active.blend());
        paint.clip = clip;
        let mut fresh = Self {
            ops: vec![Op::end_active(&paint)],
            bounds: Vec::new(),
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
}
