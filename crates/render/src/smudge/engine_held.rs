//! The residuals as a draw reads them: the arena's buffer and the
//! in-progress residual's, and where each of a scene's smudges' and filters' blocks starts in that
//! arena, taken together; each draw names its smudge's or filter's block by its base. The raster
//! core's worker draws a view tier's tile a few frames after the frame loop handed it over, and
//! meanwhile the frame loop may move the arena into a new buffer. Bases taken with their buffer
//! stay right: the arena writes a block over only in a buffer it has no more, or where no scene
//! the canvas shows reads it any more (`Arena::reserve`), and a tile reading one of those was
//! reached by the change that took the smudge or filter away, so it is drawn again.

use std::sync::Arc;

use super::{Arena, Smudges, key};
use crate::filter::Filters;
use crate::scene::Scene;

#[derive(Debug, Clone)]
pub(crate) struct Held {
    /// The arena, and the in-progress residual (an empty one without).
    pub(crate) buffers: [wgpu::Buffer; 2],
    pub(crate) bases: Bases,
}

/// Per smudge and per filter of a scene, in its order: the word its block starts at in the
/// arena; `None` while the arena does not hold it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Bases {
    pub(crate) smudges: Arc<[Option<u32>]>,
    pub(crate) filters: Arc<[Option<u32>]>,
}

impl Smudges {
    /// The residuals of `scene`'s smudges and the results of its filters (as `filters` names
    /// them) as the arena holds them now.
    pub(crate) fn held(&self, scene: &Scene, filters: &Filters) -> Held {
        let [_, active] = self.residuals();
        held_in(&self.arena, active, (scene, filters))
    }
}

/// The residuals of `scene`'s smudges and its filters' results (`filters` naming them) as `arena`
/// holds them now, `active` the in-progress residual (an empty one for a material, which
/// never reads it).
pub(crate) fn held_in(
    arena: &Arena,
    active: &wgpu::Buffer,
    (scene, filters): (&Scene, &Filters),
) -> Held {
    let based = |base: Option<u64>| base.and_then(|base| u32::try_from(base).ok());
    let smudges = (scene.smudges.iter())
        .map(|item| based(arena.base(&key(item.layer, &item.stroke))))
        .collect();
    let filters = (filters.known(&scene.filters).into_iter())
        .map(|key| based(key.and_then(|key| arena.base(&key))))
        .collect();
    Held {
        buffers: [arena.buffer().clone(), active.clone()],
        bases: Bases { smudges, filters },
    }
}
