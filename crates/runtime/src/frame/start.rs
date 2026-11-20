//! Starting the runtime: the GPU device, the theme, the workers (the video's among them), the
//! brush picker's previews, and opening the library on the storage worker.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Clipboard, Language, PalmRejection, ProjectStore, Toolbox, Transfers,
};
use openpaint_practice_app::PracticeLog;
use openpaint_practice_store::Packs;
use openpaint_practice_ui::PracticeScreens;
use openpaint_render::Core;
use openpaint_store::DisplayCache;
use openpaint_ui::Shell;
use openpaint_ui::design;

use super::{Replays, Runtime};
use crate::brush_pictures::BrushPictures;
use crate::budget::Budget;
use crate::clock::{FrameClock, InputClock};
use crate::display::CacheWorker;
use crate::error::RuntimeError;
use crate::fill::FillWorker;
use crate::normalise::Normaliser;
use crate::pacing::Pacing;
use crate::pictures::PictureWorker;
use crate::practice::{CompareWorker, PackPictureWorker};
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
    /// The system's language (the host's locale): the interface shows it until one is chosen
    /// in Actions (D100). English by default.
    pub language: Language,
    /// Where practice packs are kept (the Android host's `filesDir/packs`, beside the library).
    /// `None`: there are none, and none can be imported.
    pub packs: Option<Packs>,
    /// A library that lists no artwork opens on a blank canvas instead of the home: the
    /// harness's and the tests' first artwork, their canvases as they always began. The app
    /// leaves it off: a first launch opens on the home, its front door (S122).
    pub blank: bool,
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
        let (cache, memory, language) = (options.cache, options.memory, options.language);
        let core = match options.core {
            Some(core) => core,
            None => Arc::new(Core::headless()?),
        };
        let toolbox = Toolbox::new(BrushLibrary::built_in()?);
        let pack_pictures = PackPictureWorker::start(options.packs.clone())?;
        let storage = Storage::start(store, (cache.clone(), options.packs, options.blank))?;
        log::info!("GPU adapter: {}", core.adapter());
        core.catch_errors();
        let ui = egui::Context::default();
        design::install(&ui);
        let mut shell = shell();
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
            system_language: language,
            transfers: Transfers::default(),
            clipboard: Clipboard::default(),
            practice: PracticeLog::default(),
            utc_offset: 0,
            seeds: Seeds::from_clock(),
            bench_fast: false,
            ui_ns: 0,
            transfer: TransferWorker::start(shared.clone())?,
            fill: FillWorker::start(shared.clone())?,
            compare: CompareWorker::start(shared.clone())?,
            pictures: PictureWorker::start(shared.clone())?,
            pack_pictures,
            raster: RasterWorker::start(shared.clone())?,
            replays: Replays::start(shared.clone())?,
            budget: Budget::of(&core, memory),
            core,
            shared,
            ui,
            shell,
            practice_screens: PracticeScreens::default(),
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

/// The shell with Actions' last line (D79): the workspace's version, which xtask also hands to
/// Android as `versionName` with the same format as build metadata, and the storage format; and
/// the names practice gives its copies, pictures and layers, shown in the language (D112).
fn shell() -> Shell {
    let mut shell = Shell::default();
    shell.set_version(env!("CARGO_PKG_VERSION"), openpaint_store::FORMAT);
    shell.set_names(openpaint_practice_ui::NAMES);
    shell
}
