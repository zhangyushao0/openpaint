//! Shared by the runtime's GPU tests: the test binary's one GPU device, which every runtime and
//! cold evaluation of its tests runs on. A device per test once deadlocked NVIDIA's Windows
//! driver: opening one loads a library under the driver's lock while an exiting thread holds
//! the loader lock and waits for the driver's (2026-10-04).

use std::sync::{Arc, OnceLock};

use openpaint_render::Core;

/// The device, opened by the first test that asks.
pub fn core() -> Arc<Core> {
    static CORE: OnceLock<Arc<Core>> = OnceLock::new();
    let core = CORE.get_or_init(|| {
        Arc::new(Core::headless().expect("a GPU adapter is required for runtime tests"))
    });
    Arc::clone(core)
}
