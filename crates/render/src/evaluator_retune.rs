//! A kept scene given a previewed adjustment's new values in place (S35, `preview.rs`), and the
//! serials naming each scene built or changed.

use openpaint_core::Artwork;

use super::CanvasTarget;
use crate::core::Core;
use crate::error::RenderError;
use crate::preview::Retune;
use crate::protocol::{Op, encode};
use crate::smudge::Wanted;

impl CanvasTarget {
    /// Gives the kept scene `retune`'s values: the ops and tables that changed rewritten on the
    /// GPU, a filter's result prepared anew.
    pub(super) fn retune(
        &mut self,
        core: &Core,
        artwork: &Artwork,
        retune: &Retune,
    ) -> Result<(), RenderError> {
        let cache = self
            .scene
            .as_mut()
            .ok_or(RenderError::Invalid("no scene was built"))?;
        let changed = cache.scene.retune(retune)?;
        let queue = &core.gpu.queue;
        for item in changed.items {
            let op = [cache.scene.items[item].op()];
            queue.write_buffer(
                &cache.ops.ops(),
                (item * size_of::<Op>()) as u64,
                &encode(&op),
            );
        }
        for words in changed.words {
            let at = (words.start * 4) as u64;
            queue.write_buffer(&cache.ops.masks, at, &encode(&cache.scene.masks[words]));
        }
        let (smudges, filters) = (&mut self.smudges, &mut self.filters);
        let scene = (&cache.scene, &cache.ops);
        smudges.prepare(core, scene, artwork, (filters, &Wanted::Near(&[])))?;
        cache.artwork = artwork.clone();
        cache.spread = retune.spread;
        cache.batch = None;
        cache.serial = next_serial();
        if cache.previewed != Some(retune.place) {
            cache.previewed = Some(retune.place);
            cache.snapshot = None;
        }
        Ok(())
    }
}

/// The next `SceneCache::serial`.
pub(super) fn next_serial() -> u64 {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}
