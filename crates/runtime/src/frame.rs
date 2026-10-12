//! `Runtime`: the frame loop a platform host drives. The host attaches, resizes and detaches
//! its window, hands over input batches, keyboard text and the back button, and asks for
//! frames; all calls come from one thread.
//!
//! A frame: take what the storage worker finished -> take the input normalised since the last
//! frame, palms removed, and the keyboard's text -> run the UI from the screen's projection ->
//! route the canvas contacts and the chrome's taps as intents to the session or the gallery ->
//! evaluate what changed into the canvas target -> hand the store what it must write or read
//! -> draw the canvas and the UI over it.

use std::sync::Arc;

use openpaint_app::{
    BrushLibrary, Clipboard, InputBatch, Intent, PalmRejection, ProjectStore, ScreenProjection,
    SessionProjection, Toolbox, Transfers, TransformIntent,
};
use openpaint_core::Artwork;
use openpaint_render::{Core, ScreenPainter, ScreenTarget, Surface, SurfaceWindow, UiFrame};
use openpaint_ui::Shell;
use openpaint_ui::design::{self, TextFonts};

use crate::brush_pictures::BrushPictures;
use crate::budget::Budget;
use crate::clock::{FrameClock, InputClock};
use crate::error::RuntimeError;
use crate::fill::FillWorker;
use crate::input::MotionBatch;
use crate::normalise::Normaliser;
use crate::pictures::PictureWorker;
use crate::pointer::pointer_events;
use crate::recover::{Reattach, Recoveries};
use crate::screen::{Screen, dispatch_gallery};
use crate::seeds::Seeds;
use crate::shared::SharedCore;
use crate::storage::{Job, Storage};
use crate::text::{TextInput, text_events};
use crate::transfer::{TransferWorker, route};
pub(crate) use crate::ui_pass::screen_frame;
use crate::ui_pass::{UiPass, run_ui};

pub struct Runtime {
    /// The renderer's GPU state now; replaced whole when its device is lost (S49, `recover.rs`).
    pub(crate) core: Arc<Core>,
    /// The same, as the workers take it for each job.
    pub(crate) shared: SharedCore,
    ui: egui::Context,
    pub(crate) shell: Shell,
    /// Created with the first target and kept for the process: it holds the UI's textures.
    pub(crate) painter: Option<ScreenPainter>,
    pub(crate) surface: Option<Surface>,
    /// Makes the window's surface again on a new device, from `attach` until `detach`.
    pub(crate) window: Option<Reattach>,
    pub(crate) recoveries: Recoveries,
    /// The GPU memory budget of the device's class (S49, `budget.rs`).
    pub(crate) budget: Budget,
    /// The harness's frame, in place of a window.
    pub(crate) offscreen: Option<ScreenTarget>,
    pixels_per_point: f32,
    normaliser: Normaliser,
    /// Drops the hand resting on the glass before the UI or the session sees it.
    palms: PalmRejection,
    /// The input samples' clock, carried on between batches for gestures.
    clock: InputClock,
    /// The clock motion runs on.
    pub(crate) frame_clock: FrameClock,
    /// Input normalised since the last frame, palms removed, oldest first.
    pending: Vec<InputBatch>,
    /// Keyboard text for the focused text field since the last frame.
    typed: Vec<egui::Event>,
    /// Whether the last frame had a text field waiting for the keyboard.
    keyboard: bool,
    pub(crate) screen: Screen,
    /// The brush, size and opacity: the app's, kept across artworks; the open session gets the
    /// style it makes.
    pub(crate) toolbox: Toolbox,
    /// What was copied last (S40): the app's, kept across artworks, never stored.
    clipboard: Clipboard,
    /// Where every new stroke's seed comes from (S45).
    pub(crate) seeds: Seeds,
    /// Export and the reference image: the app's, kept across artworks.
    pub(crate) transfers: Transfers,
    pub(crate) transfer: TransferWorker,
    fill: FillWorker,
    pub(crate) pictures: PictureWorker,
    /// The brush panel's pictures and the editor's preview strip (S46).
    pub(crate) brushes: BrushPictures,
    /// Declared last: dropping it writes what it was handed before the process goes.
    pub(crate) storage: Storage,
}

impl Runtime {
    /// Opens the GPU device, installs the theme, draws the brush picker's previews and starts
    /// opening the library (and the stored toolbox) from `store` on the storage worker.
    /// `pixels_per_point` is the display density (Android's `DisplayMetrics.density`):
    /// physical pixels per UI point.
    pub fn start(
        pixels_per_point: f32,
        store: Box<dyn ProjectStore>,
    ) -> Result<Self, RuntimeError> {
        let toolbox = Toolbox::new(BrushLibrary::built_in()?);
        let storage = Storage::start(store)?;
        let core = Arc::new(Core::headless()?);
        Self::assemble(core, toolbox, storage, pixels_per_point)
    }

    /// As `start`, on a GPU device already open: the GPU tests open one per process, as a device
    /// per test once deadlocked a desktop driver while another test's threads exited.
    pub fn start_on(
        core: Arc<Core>,
        pixels_per_point: f32,
        store: Box<dyn ProjectStore>,
    ) -> Result<Self, RuntimeError> {
        let toolbox = Toolbox::new(BrushLibrary::built_in()?);
        let storage = Storage::start(store)?;
        Self::assemble(core, toolbox, storage, pixels_per_point)
    }

    fn assemble(
        core: Arc<Core>,
        toolbox: Toolbox,
        storage: Storage,
        pixels_per_point: f32,
    ) -> Result<Self, RuntimeError> {
        log::info!("GPU adapter: {}", core.adapter());
        core.catch_errors();
        let ui = egui::Context::default();
        design::install(&ui);
        let mut shell = Shell::default();
        let library = toolbox.library();
        let shared = SharedCore::new(Arc::clone(&core));
        let brushes =
            BrushPictures::start(shared.clone(), library, (&mut shell, pixels_per_point))?;
        Ok(Self {
            brushes,
            storage,
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
            pending: Vec::new(),
            typed: Vec::new(),
            keyboard: false,
        })
    }

    /// The host's text faces: its system's sans and a face with the Chinese characters, read
    /// from the system's font files. Call before the first frame.
    pub fn set_text_fonts(&mut self, fonts: TextFonts) {
        design::install_text_fonts(&self.ui, fonts);
    }

    /// The host's window exists, `size` pixels large. Replaces any previous window. The window is
    /// kept until `detach`, so a new device can present into it again (S49).
    pub fn attach(
        &mut self,
        window: impl SurfaceWindow + Send + Sync,
        size: [u32; 2],
    ) -> Result<(), RuntimeError> {
        self.surface = None;
        let window = Arc::new(window);
        let surface = self.core.create_surface(Arc::clone(&window), size)?;
        self.window = Some(Box::new(move |core: &Core, size| {
            core.create_surface(Arc::clone(&window), size)
        }));
        log::info!("surface presents {}", surface.report());
        if self.painter.is_none() {
            self.painter = Some(ScreenPainter::for_surface(&self.core, &surface));
        }
        self.surface = Some(surface);
        Ok(())
    }

    /// The window changed size or density (rotation, split screen, display change).
    pub fn resize(&mut self, size: [u32; 2], pixels_per_point: f32) {
        self.pixels_per_point = pixels_per_point;
        if let Some(surface) = &mut self.surface {
            surface.resize(&self.core, size);
        }
    }

    /// The window is about to be destroyed; call before the host lets it go. A stroke still being
    /// drawn is committed as drawn so far and handed to the store (S50): the pen cannot lift it
    /// any more. The artwork, its history and the view stay for the next `attach`.
    pub fn detach(&mut self) {
        self.surface = None;
        self.window = None;
        if let Some(canvas) = self.screen.canvas_mut() {
            canvas.end_stroke(&mut self.toolbox);
            if let Some(version) = canvas.session_mut().save_request() {
                self.storage.submit(Job::Save(version));
            }
        }
    }

    /// Takes one platform input batch for the next frame. Returns whether it holds anything,
    /// in which case the host should ask for a frame.
    pub fn motion(&mut self, batch: &MotionBatch) -> bool {
        log::debug!(target: "openpaint::input", "{batch}");
        let Some(input) = self.normaliser.normalise(batch) else {
            return false;
        };
        let kept = self.palms.filter(input);
        let wanted = !kept.is_empty();
        for batch in &kept {
            self.clock.saw(batch);
        }
        self.pending.extend(kept);
        wanted
    }

    /// Takes keyboard text for the focused text field; the host should ask for a frame.
    pub fn text(&mut self, input: TextInput) {
        self.typed.extend(text_events(input));
    }

    /// True while a text field waits for typing: the host shows its soft keyboard then and
    /// hides it otherwise.
    #[must_use]
    pub fn keyboard_wanted(&self) -> bool {
        self.keyboard
    }

    /// The system's back button. Returns whether the app used it (the host then does nothing
    /// more): from the canvas it goes to the gallery, in the gallery it closes an open dialog.
    /// Otherwise the system's own back applies.
    pub fn back(&mut self) -> bool {
        self.shell.back(&self.screen())
    }

    /// Runs one frame into the window. Returns whether another frame is wanted at once. A lost
    /// GPU device is made again first, or after a frame that failed for it (S49).
    pub fn frame(&mut self) -> Result<bool, RuntimeError> {
        self.recover_if_lost()?;
        let result = self.frame_once();
        self.after_frame(result)
    }

    fn frame_once(&mut self) -> Result<bool, RuntimeError> {
        let Some(size) = self.surface.as_ref().map(Surface::size) else {
            return Ok(false);
        };
        let (ui, repaint) = self.update(size)?;
        let (Some(surface), Some(painter)) = (self.surface.as_mut(), self.painter.as_mut()) else {
            return Ok(false);
        };
        let frame = screen_frame(self.screen.canvas(), &self.shell, ui);
        let presented = self.core.present(surface, painter, frame)?;
        Ok(repaint || !presented)
    }

    /// The open artwork, while the canvas shows one.
    #[must_use]
    pub fn artwork(&self) -> Option<&Artwork> {
        self.screen
            .canvas()
            .map(|canvas| canvas.session().artwork())
    }

    /// The open session's projection, what the canvas offers; without a canvas, nothing is
    /// available.
    #[must_use]
    pub fn projection(&self) -> SessionProjection {
        match self.screen() {
            ScreenProjection::Canvas(projection) => *projection,
            _ => SessionProjection::default(),
        }
    }

    /// What the whole screen offers: the canvas, the gallery, or nothing yet.
    #[must_use]
    pub fn screen(&self) -> ScreenProjection {
        let app = (&self.transfers, &self.clipboard);
        self.screen.projection(&self.toolbox, app)
    }

    /// True while the frame loop waits for something without input: the storage, transfer,
    /// fill or picture worker, or a canvas about to open. Frames keep coming meanwhile.
    #[must_use]
    pub fn storing(&self) -> bool {
        let working = self.storage.busy() || self.transfer.busy() || self.fill.busy();
        let drawing = self.pictures.busy() || self.brushes.busy();
        working || drawing || matches!(self.screen, Screen::Opening(_))
    }

    /// Applies everything the workers finished, then opens a canvas that is ready.
    pub(crate) fn receive(&mut self) -> Result<(), RuntimeError> {
        self.transfer.receive(&mut self.transfers);
        while let Some(done) = self.storage.take() {
            self.screen.answer(done, &mut self.toolbox);
        }
        let area = self.shell.canvas_area();
        self.screen.open(&self.core, area, &self.toolbox)
    }

    /// Everything before drawing: storage results, input, UI, intents, evaluation, storage
    /// calls. Asks for another frame when the screen changed what it offers, so it never
    /// shows a stale state while idle, while waiting for the storage worker and while
    /// something moves (D44).
    pub(crate) fn update(&mut self, size: [u32; 2]) -> Result<(UiFrame, bool), RuntimeError> {
        self.receive()?;
        let batches = std::mem::take(&mut self.pending);
        let mut events = pointer_events(&batches, self.pixels_per_point);
        events.append(&mut self.typed);
        self.shell.set_now(self.clock.now_ms());
        let shown = self.screen();
        let pass = run_ui(
            &self.ui,
            UiPass {
                shell: &mut self.shell,
                projection: &shown,
                size,
                pixels_per_point: self.pixels_per_point,
                events,
                time: self.frame_clock.seconds(),
            },
        );
        self.keyboard = pass.keyboard;
        let intents = self.shell.route(&batches);
        let mut intents = route(&mut self.transfers, self.screen.canvas(), intents);
        // A picture read for inserting goes to the canvas open now, before this frame's taps.
        if let Some(picture) = self.transfers.take_picture() {
            intents.insert(0, Intent::Transform(TransformIntent::Insert(picture)));
        }
        let area = self.shell.canvas_area();
        self.screen.open(&self.core, area, &self.toolbox)?;
        let mut arrived = false;
        match &mut self.screen {
            Screen::Canvas(canvas) => {
                let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
                canvas.dispatch(&self.core, intents, app);
                self.fill.step(canvas);
                self.pictures.step(canvas, &mut self.shell);
                arrived = self.brushes.step(&self.toolbox, &mut self.shell);
            }
            Screen::Gallery(gallery) => dispatch_gallery(gallery, intents),
            Screen::Opening(_) => {}
        }
        if let Some(state) = self.toolbox.save_request() {
            self.storage.submit(Job::SaveEditor(Box::new(state)));
        }
        let now = self.frame_clock.wall_seconds();
        self.keep_within_budget()?;
        self.screen
            .progress(&self.core, &mut self.storage, (area, now))?;
        if let (Some(canvas), Some(area)) = (self.screen.canvas_mut(), area) {
            let shell = &self.shell;
            let frame = canvas.view_frame((&area, self.pixels_per_point), |mapping| {
                shell.displayed(mapping)
            });
            canvas.update_view(&self.core, (&frame, self.frame_clock.seconds()))?;
        }
        let changed = self.screen() != shown;
        // The view tiers or the canvas itself still fill in (S32, S49).
        let viewing = self
            .screen
            .canvas()
            .is_some_and(|canvas| canvas.view_pending() || canvas.filling());
        let shows = changed || arrived || viewing;
        let again = pass.repaint || shows || self.storing() || self.shell.animating();
        Ok((pass.frame, again))
    }
}
