//! The residuals as a draw on another thread reads them (S61, DECISIONS D88): the arena's buffer
//! and the in-progress residual's, and where each of a scene's smudges' blocks starts in that
//! arena, taken together. The raster core's worker draws a view tier's tile a few frames after
//! the frame loop handed it over, and meanwhile the frame loop may move the arena (into a new
//! buffer, writing the new bases into the scene's mask words): a draw that read the bases from
//! there would read the new buffer's bases in the old buffer. Bases taken with their buffer stay
//! right: the arena writes a block over only in a buffer it has no more, or where no scene the
//! canvas shows reads it any more (`Arena::reserve`), and a tile reading one of those was reached
//! by the change that took the smudge away, so it is drawn again.

use std::sync::Arc;

use super::{Arena, Smudges, key};
use crate::scene::Scene;

#[derive(Debug, Clone)]
pub(crate) struct Held {
    /// The arena, and the in-progress residual (an empty one without).
    pub(crate) buffers: [wgpu::Buffer; 2],
    /// Per smudge of the scene, in its order: the word its block starts at in the arena; `None`
    /// while the arena does not hold it.
    pub(crate) bases: Arc<[Option<u32>]>,
}

impl Smudges {
    /// The residuals of `scene`'s smudges as the arena holds them now.
    pub(crate) fn held(&self, scene: &Scene) -> Held {
        let [_, active] = self.residuals();
        held_in(&self.arena, active, scene)
    }
}

/// The residuals of `scene`'s smudges as `arena` holds them now, `active` the in-progress one's
/// (S62: an empty one for a smudge's material, which never reads it).
pub(crate) fn held_in(arena: &Arena, active: &wgpu::Buffer, scene: &Scene) -> Held {
    let bases = (scene.smudges.iter())
        .map(|item| {
            let base = arena.base(&key(item.layer, &item.stroke))?;
            u32::try_from(base).ok()
        })
        .collect();
    Held {
        buffers: [arena.buffer().clone(), active.clone()],
        bases,
    }
}
