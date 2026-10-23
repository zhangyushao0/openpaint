//! Starting the runtime: the GPU device, the theme, the workers, the brush picker's previews,
//! and opening the library on the storage worker.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Clipboard, PalmRejection, PracticeLog, ProjectStore, Toolbox, Transfers,
};
use openpaint_render::Core;
use openpaint_store::DisplayCache;
use openpaint_ui::Shell;
use openpaint_ui::design;

use super::Runtime;
use crate::brush_pictures::BrushPictures;
use crate::budget::Budget;
use crate::clock::{FrameClock, InputClock};
use crate::display::CacheWorker;
use crate::error::RuntimeError;
use crate::fill::FillWorker;
use crate::host::version_line;
use crate::normalise::Normaliser;
use crate::pacing::Pacing;
use crate::pictures::PictureWorker;
use crate::raster::RasterWorker;
use crate::recover::Recoveries;
use crate::screen::Screen;
use crate::seeds::Seeds;
use crate::shared::SharedCore;
use crate::storage::Storage;
use crate::touches::Touches;
use crate::transfer::TransferWorker;

/// How the runtime starts beyond its store and density; `default()`: the harness's and most
/// tests' start.
#[derive(Default)]
pub struct StartOptions {
    /// A GPU device already open: the GPU tests open one per process, as a device per test
    /// once deadlocked a desktop driver while another test's threads exited. `None`: one is
    /// opened.
    pub core: Option<Arc<Core>>,
    /// Where each artwork's canvas is kept as last shown complete (the Android host's
    /// `cacheDir/canvas`): an artwork opens showing it, complete when it is what the artwork's
    /// records evaluate to. `None`: every artwork opens evaluated.
    pub cache: Option<DisplayCache>,
    /// The device's memory in bytes (Android's `ActivityManager.MemoryInfo.totalMem`): the GPU
    /// memory budget follows from it (`Core::memory_budget`).
    pub memory: Option<u64>,
}

impl Runtime {
    /// Opens the GPU device (or takes `options.core`), installs the theme, draws the brush
    /// picker's previews and starts opening the library (and the stored toolbox) from `store` on
    /// the storage worker. `pixels_per_point` is the display density (Android's
    /// `DisplayMetrics.density`): physical pixels per UI point.
    pub fn start(
        pixels_per_point: f32,
        store: Box<dyn ProjectStore>,
        options: StartOptions,
    ) -> Result<Self, RuntimeError> {
        let StartOptions {
            core,
            cache,
            memory,
        } = options;
        let core = match core {
            Some(core) => core,
            None => Arc::new(Core::headless()?),
        };
        let toolbox = Toolbox::new(BrushLibrary::built_in()?);
        let storage = Storage::start(store, cache.clone())?;
        log::info!("GPU adapter: {}", core.adapter());
        core.catch_errors();
        let ui = egui::Context::default();
        design::install(&ui);
        let mut shell = Shell::default();
        shell.set_version(&version_line());
        let library = toolbox.library();
        let shared = SharedCore::new(Arc::clone(&core));
        let brushes =
            BrushPictures::start(shared.clone(), library, (&mut shell, pixels_per_point))?;
        Ok(Self {
            brushes,
            storage,
            cache: CacheWorker::start(cache)?,
            screen: Screen::Opening(None),
            toolbox,
            transfers: Transfers::default(),
            clipboard: Clipboard::default(),
            practice: PracticeLog::default(),
            seeds: Seeds::from_clock(),
            bench_fast: false,
            ui_ns: 0,
            transfer: TransferWorker::start(shared.clone())?,
            fill: FillWorker::start(shared.clone())?,
            pictures: PictureWorker::start(shared.clone())?,
            raster: RasterWorker::start(shared.clone())?,
            budget: Budget::of(&core, memory),
            core,
            shared,
            ui,
            shell,
            painter: None,
            surface: None,
            window: None,
            recoveries: Recoveries::default(),
            offscreen: None,
            pixels_per_point,
            normaliser: Normaliser::default(),
            palms: PalmRejection::default(),
            clock: InputClock::default(),
            frame_clock: FrameClock::default(),
            touches: Touches::default(),
            pacing: Pacing::default(),
            pending: Vec::new(),
            typed: Vec::new(),
            keyboard: false,
        })
    }
}
