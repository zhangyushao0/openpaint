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
    Clipboard, InputBatch, Intent, Language, PalmRejection, PracticeLog, ScreenProjection,
    SessionProjection, Toolbox, Transfers, TransformIntent,
};
use openpaint_core::Artwork;
use openpaint_render::{Core, ScreenPainter, ScreenTarget, Surface, SurfaceWindow, UiFrame};
use openpaint_ui::design::{self, TextFonts};
use openpaint_ui::{CanvasArea, Shell};

use crate::brush_pictures::BrushPictures;
use crate::budget::Budget;
use crate::canvas::Canvas;
use crate::clock::{FrameClock, InputClock};
use crate::display::CacheWorker;
use crate::error::RuntimeError;
use crate::fill::FillWorker;
use crate::input::MotionBatch;
use crate::normalise::Normaliser;
use crate::pacing::{Paced, Pacing, Wanted};
use crate::pictures::PictureWorker;
use crate::pointer::pointer_events;
use crate::raster::RasterWorker;
use crate::recover::{Reattach, Recoveries};
use crate::screen::Screen;
use crate::seeds::Seeds;
use crate::shared::SharedCore;
use crate::storage::{Job, Storage};
use crate::text::{TextInput, text_events};
use crate::touches::Touches;
use crate::transfer::{TransferWorker, route};
pub(crate) use crate::ui_pass::screen_frame;
use crate::ui_pass::{UiPass, run_ui};

pub struct Runtime {
    /// The renderer's GPU state now; replaced whole when its device is lost (`recover.rs`).
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
    /// The GPU memory budget of the device's class (`budget.rs`).
    pub(crate) budget: Budget,
    /// The harness's frame, in place of a window.
    pub(crate) offscreen: Option<ScreenTarget>,
    pub(crate) pixels_per_point: f32,
    normaliser: Normaliser,
    /// Drops the hand resting on the glass before the UI or the session sees it.
    palms: PalmRejection,
    /// The input samples' clock, carried on between batches for gestures.
    pub(crate) clock: InputClock,
    /// The clock motion runs on.
    pub(crate) frame_clock: FrameClock,
    /// When a finger or the pen last touched: background work's budget follows.
    pub(crate) touches: Touches,
    /// Frames wait while the GPU is behind.
    pub(crate) pacing: Pacing,
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
    /// The host's language, which shows until the toolbox holds one chosen in Actions.
    system_language: Language,
    /// What was copied last: the app's, kept across artworks, never stored.
    pub(crate) clipboard: Clipboard,
    /// Every practice session: the app's, kept across artworks, as the store holds them.
    pub(crate) practice: PracticeLog,
    /// Where every new stroke's seed comes from.
    pub(crate) seeds: Seeds,
    /// Debug builds' scenario benchmark runs its frames as fast as the GPU takes them.
    pub(crate) bench_fast: bool,
    /// The last UI pass's time (egui's layout and tessellation), ns, for the benchmarks.
    pub(crate) ui_ns: u64,
    /// Export and the reference image: the app's, kept across artworks.
    pub(crate) transfers: Transfers,
    pub(crate) transfer: TransferWorker,
    fill: FillWorker,
    fit: crate::fit::FitWorker,
    pub(crate) pictures: PictureWorker,
    /// Makes the raster core's pipelines and draws the view tiers' tiles.
    pub(crate) raster: RasterWorker,
    /// The brush panel's pictures and the editor's preview strip.
    pub(crate) brushes: BrushPictures,
    /// Writes the display cache; dropping it writes what it was handed.
    pub(crate) cache: CacheWorker,
    /// Declared last: dropping it writes what it was handed before the process goes.
    pub(crate) storage: Storage,
}

#[path = "frame_receive.rs"]
mod receive;
#[path = "frame_start.rs"]
mod start;
pub use start::StartOptions;

impl Runtime {
    /// The host's text faces: its system's sans and a face with the Chinese characters, read
    /// from the system's font files. Call before the first frame.
    pub fn set_text_fonts(&mut self, fonts: TextFonts) {
        design::install_text_fonts(&self.ui, fonts);
    }

    /// The host's window exists, `size` pixels large. Replaces any previous window. The window is
    /// kept until `detach`, so a new device can present into it again.
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
    /// drawn is committed as drawn so far and handed to the store: the pen cannot lift it
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
        // A hand arriving ends the work handed over while nothing touched, at once.
        if self.touches.saw(&kept, self.frame_clock.seconds()) {
            self.raster.hand_arrived();
        }
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
    /// GPU device is made again first, or after a frame that failed for it. While the GPU
    /// is behind, the call draws nothing and wants the next (`pacing.rs`); while nothing
    /// touches the screen it gives the work waiting what the GPU's queue has room for.
    pub fn frame(&mut self) -> Result<bool, RuntimeError> {
        self.recover_if_lost()?;
        let drawing = self.screen.canvas().is_some_and(Canvas::drawing);
        // Input waiting is a hand's (a lift, the last of a drag) even once nothing touches.
        let input = !self.pending.is_empty() || !self.typed.is_empty();
        let resting = self.touches.resting(drawing) && !input;
        let wanted = Wanted::of(drawing, (resting, self.shell.animating()));
        if self.pacing.skips(&self.core, (true, wanted)) {
            let result = self.ahead().map(|()| true);
            return self.after_frame(result);
        }
        let result = self.frame_once();
        self.core.frame_submitted();
        self.after_frame(result)
    }

    /// What the last frame call did: skipped for the GPU, or drawn, and the frames in flight as
    /// it began (for the host's log).
    #[must_use]
    pub const fn paced(&self) -> Paced {
        self.pacing.last()
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
        self.core.frame_ended();
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

    /// The language the interface shows: the one chosen in Actions, else the system's (D100).
    #[must_use]
    pub fn language(&self) -> Language {
        self.toolbox.language().unwrap_or(self.system_language)
    }

    /// What the whole screen offers: the canvas, the gallery, or nothing yet.
    #[must_use]
    pub fn screen(&self) -> ScreenProjection {
        let app = (&self.transfers, &self.clipboard, &self.practice);
        self.screen.projection(&self.toolbox, app)
    }

    /// True while the frame loop waits for something without input: the storage, transfer,
    /// fill or picture worker, or a canvas about to open. Frames keep coming meanwhile.
    #[must_use]
    pub fn storing(&self) -> bool {
        let working = self.storage.busy() || self.transfer.busy() || self.fill.busy();
        let working = working || self.fit.busy();
        let drawing = self.pictures.busy() || self.brushes.busy() || self.cache.busy();
        working || drawing || matches!(self.screen, Screen::Opening(_))
    }

    /// Everything before drawing: storage results, input, UI, intents, evaluation, storage
    /// calls. Asks for another frame when the screen changed what it offers, so it never
    /// shows a stale state while idle, while waiting for the storage worker and while
    /// something moves (D44).
    pub(crate) fn update(&mut self, size: [u32; 2]) -> Result<(UiFrame, bool), RuntimeError> {
        // This frame's budget for background work, corrected by the GPU's measured times;
        // while nothing touches the screen, what the GPU's queue has room for.
        let idle = self.idle();
        self.core.begin_frame_idle(idle);
        // A benchmark times the frame's GPU work from its first submission on.
        self.core.frame_begun();
        self.receive()?;
        let batches = std::mem::take(&mut self.pending);
        let mut events = pointer_events(&batches, self.pixels_per_point);
        events.append(&mut self.typed);
        self.shell.set_now(self.clock.now_ms());
        let language = self.language();
        self.shell.set_language(language);
        let shown = self.screen();
        let laid = std::time::Instant::now();
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
        self.ui_ns = u64::try_from(laid.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.keyboard = pass.keyboard;
        let intents = self.shell.route(&batches);
        let mut intents = route(&mut self.transfers, self.screen.canvas(), intents);
        // A picture read for inserting goes to the canvas open now, before this frame's taps.
        if let Some(picture) = self.transfers.take_picture() {
            intents.insert(0, Intent::Transform(TransformIntent::Insert(picture)));
        }
        let area = self.shell.canvas_area();
        self.open_canvas()?;
        let arrived = self.apply(intents);
        if let Some(state) = self.toolbox.save_request() {
            self.storage.submit(Job::SaveEditor(Box::new(state)));
        }
        self.keep_within_budget()?;
        self.answer_raster();
        self.evaluate_screen(area)?;
        self.core.end_background();
        self.send_raster(idle);
        let changed = self.screen() != shown || self.language() != language;
        // Also until the next tier's atlas is asked for, made ahead once nothing touches.
        let viewing = (self.screen.canvas())
            .is_some_and(|canvas| canvas.waiting() || canvas.view().wants_spare());
        let shows = changed || arrived || viewing;
        let again = pass.repaint || shows || self.storing() || self.shell.animating();
        Ok((pass.frame, again))
    }

    /// The screen's evaluations this frame: where it looks on the canvas (its waiting tiles there
    /// go first), what changed and storage's steps, then the view tiers through the view.
    fn evaluate_screen(&mut self, area: Option<CanvasArea>) -> Result<(), RuntimeError> {
        let now = self.frame_clock.wall_seconds();
        let shell = &self.shell;
        let frame = match (self.screen.canvas_mut(), area) {
            (Some(canvas), Some(area)) => {
                let frame = canvas.view_frame((&area, self.pixels_per_point), |mapping| {
                    shell.displayed(mapping)
                });
                canvas.look_at(&frame);
                Some(frame)
            }
            _ => None,
        };
        let workers = (&mut self.storage, &mut self.cache);
        let practice = (&mut self.practice, &mut self.seeds);
        self.screen
            .progress(&self.core, workers, (area, now), practice)?;
        if let (Some(canvas), Some(frame)) = (self.screen.canvas_mut(), frame) {
            canvas.update_view(&self.core, (&frame, self.frame_clock.seconds()))?;
        }
        Ok(())
    }
}
