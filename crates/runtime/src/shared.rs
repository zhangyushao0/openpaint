//! `SharedCore`: the renderer's GPU state as the frame loop and the workers hold it (S49). When the
//! device is lost the frame loop puts a new `Core` here; each worker job takes the one current
//! when it starts, so a job running at the loss fails as any GPU failure does and the next one
//! runs on the new device. No worker is restarted.

use std::sync::{Arc, PoisonError, RwLock};

use openpaint_render::Core;

#[derive(Clone)]
pub(crate) struct SharedCore(Arc<RwLock<Arc<Core>>>);

impl SharedCore {
    pub(crate) fn new(core: Arc<Core>) -> Self {
        Self(Arc::new(RwLock::new(core)))
    }

    /// The core now.
    pub(crate) fn get(&self) -> Arc<Core> {
        Arc::clone(&self.0.read().unwrap_or_else(PoisonError::into_inner))
    }

    /// Puts `core` in place of the one there.
    pub(crate) fn replace(&self, core: Arc<Core>) {
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = core;
    }
}
