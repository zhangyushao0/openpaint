//! Starting the runtime: the GPU device, the theme, the workers, the brush picker's previews,
//! and opening the library on the storage worker.

use std::sync::Arc;

use openpaint_app::{BrushLibrary, Clipboard, PalmRejection, ProjectStore, Toolbox, Transfers};
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
use crate::pictures::PictureWorker;
use crate::recover::Recoveries;
use crate::screen::Screen;
use crate::seeds::Seeds;
use crate::shared::SharedCore;
use crate::storage::Storage;
use crate::touches::Touches;
use crate::transfer::TransferWorker;

impl Runtime {
    /// Opens the GPU device, installs the theme, draws the brush picker's previews and starts
    /// opening the library (and the stored toolbox) from `store` on the storage worker.
    /// `pixels_per_point` is the display density (Android's `DisplayMetrics.density`):
    /// physical pixels per UI point. No display cache: every artwork opens evaluated (the
    /// harness, the tests).
    pub fn start(
        pixels_per_point: f32,
        store: Box<dyn ProjectStore>,
    ) -> Result<Self, RuntimeError> {
        let core = Arc::new(Core::headless()?);
        Self::assemble(core, (store, None), pixels_per_point)
    }

    /// As `start`, keeping the display cache of each artwork's canvas in `cache` (S54): an
    /// artwork opens showing it, complete when it is what the artwork's records evaluate to.
    pub fn start_cached(
        pixels_per_point: f32,
        store: Box<dyn ProjectStore>,
        cache: DisplayCache,
    ) -> Result<Self, RuntimeError> {
        let core = Arc::new(Core::headless()?);
        Self::assemble(core, (store, Some(cache)), pixels_per_point)
    }

    /// As `start`, on a GPU device already open: the GPU tests open one per process, as a device
    /// per test once deadlocked a desktop driver while another test's threads exited.
    pub fn start_on(
        core: Arc<Core>,
        pixels_per_point: f32,
        store: Box<dyn ProjectStore>,
    ) -> Result<Self, RuntimeError> {
        Self::assemble(core, (store, None), pixels_per_point)
    }

    /// As `start_cached`, on a GPU device already open (the GPU tests').
    pub fn start_on_cached(
        core: Arc<Core>,
        pixels_per_point: f32,
        (store, cache): (Box<dyn ProjectStore>, DisplayCache),
    ) -> Result<Self, RuntimeError> {
        Self::assemble(core, (store, Some(cache)), pixels_per_point)
    }

    fn assemble(
        core: Arc<Core>,
        (store, cache): (Box<dyn ProjectStore>, Option<DisplayCache>),
        pixels_per_point: f32,
    ) -> Result<Self, RuntimeError> {
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
            seeds: Seeds::from_clock(),
            transfer: TransferWorker::start(shared.clone())?,
            fill: FillWorker::start(shared.clone())?,
            pictures: PictureWorker::start(shared.clone())?,
            budget: Budget::of(&core),
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
            pending: Vec::new(),
            typed: Vec::new(),
            keyboard: false,
        })
    }
}
