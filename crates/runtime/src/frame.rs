//! `Runtime`: the frame loop a platform host drives. The host attaches, resizes and detaches
//! its window, hands over input batches, keyboard text and the back button, and asks for
//! frames; all calls come from one thread.
//!
//! A frame: take what the storage worker finished -> take the input normalised since the last
//! frame, palms removed, and the keyboard's text -> run the UI from the screen's projection ->
//! route the canvas contacts and the chrome's taps as intents to the session or the gallery ->
//! evaluate what changed into the canvas target -> hand the store what it must write or read
//! -> draw the canvas and the UI over it.

use std::time::Duration;

use openpaint_app::{InputBatch, PalmRejection, ProjectStore, ScreenProjection, SessionProjection};
use openpaint_core::Artwork;
use openpaint_render::{
    Core, RenderError, RgbaImage, ScreenFrame, ScreenPainter, ScreenTarget, Surface, SurfaceWindow,
    UiFrame,
};
use openpaint_ui::{Shell, design};

use crate::canvas::{Canvas, Leaving};
use crate::error::RuntimeError;
use crate::input::MotionBatch;
use crate::normalise::Normaliser;
use crate::pointer::pointer_events;
use crate::screen::{Screen, leave_job};
use crate::storage::Storage;
use crate::text::{TextInput, text_events};

/// Longest `leave` waits for the store; a longer save goes on in the background.
const LEAVE_WAIT: Duration = Duration::from_secs(2);

pub struct Runtime {
    core: Core,
    ui: egui::Context,
    shell: Shell,
    /// Created with the first target and kept for the process: it holds the UI's textures.
    painter: Option<ScreenPainter>,
    surface: Option<Surface>,
    /// The harness's frame, in place of a window.
    offscreen: Option<ScreenTarget>,
    pixels_per_point: f32,
    normaliser: Normaliser,
    /// Drops the hand resting on the glass before the UI or the session sees it.
    palms: PalmRejection,
    /// Input normalised since the last frame, palms removed, oldest first.
    pending: Vec<InputBatch>,
    /// Keyboard text for the focused text field since the last frame.
    typed: Vec<egui::Event>,
    /// Whether the last frame had a text field waiting for the keyboard.
    keyboard: bool,
    screen: Screen,
    /// Declared last: dropping it writes what it was handed before the process goes.
    storage: Storage,
}

impl Runtime {
    /// Opens the GPU device, installs the theme and starts opening the library from `store` on
    /// the storage worker. `pixels_per_point` is the display density (Android's
    /// `DisplayMetrics.density`): physical pixels per UI point.
    pub fn start(
        pixels_per_point: f32,
        store: Box<dyn ProjectStore>,
    ) -> Result<Self, RuntimeError> {
        let storage = Storage::start(store)?;
        let core = Core::headless()?;
        log::info!("GPU adapter: {}", core.adapter());
        let ui = egui::Context::default();
        design::install(&ui);
        Ok(Self {
            storage,
            screen: Screen::Opening(None),
            core,
            ui,
            shell: Shell::default(),
            painter: None,
            surface: None,
            offscreen: None,
            pixels_per_point,
            normaliser: Normaliser::default(),
            palms: PalmRejection::default(),
            pending: Vec::new(),
            typed: Vec::new(),
            keyboard: false,
        })
    }

    /// The host's window exists, `size` pixels large. Replaces any previous window.
    pub fn attach(
        &mut self,
        window: impl SurfaceWindow,
        size: [u32; 2],
    ) -> Result<(), RuntimeError> {
        self.surface = None;
        let surface = self.core.create_surface(window, size)?;
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

    /// The window is about to be destroyed; call before the host lets it go.
    pub fn detach(&mut self) {
        self.surface = None;
    }

    /// The user is leaving the app (Android `onPause`): the open artwork's current version (a
    /// failed one included) and its thumbnail are handed to the store, and this waits until
    /// they are written, at most `LEAVE_WAIT`; a longer save goes on in the background. A
    /// stroke still being drawn is not part of it. In the gallery, the calls handed over are
    /// waited for.
    pub fn leave(&mut self) {
        // A save running when the user left may hold an older version: hand over the newest
        // once it is done.
        let mut handed = false;
        for _ in 0..2 {
            if let Some(canvas) = self.screen.canvas_mut()
                && canvas.leaving == Leaving::Staying
                && !canvas.session().saving()
            {
                let version = canvas.session_mut().leave_request();
                if version.is_some() || !handed {
                    self.storage
                        .submit(leave_job(canvas, &self.core, version, false));
                    handed = true;
                }
            }
            if !self.storage.drain(LEAVE_WAIT) {
                log::warn!("leaving while the store is still writing");
            }
            if let Err(error) = self.receive() {
                log::error!("after leaving: {error}");
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
        self.shell.back(&self.screen.projection())
    }

    /// Runs one frame into the window. Returns whether another frame is wanted at once.
    pub fn frame(&mut self) -> Result<bool, RuntimeError> {
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

    /// Runs one frame into an offscreen target of `size` pixels, for the harness, which never
    /// attaches a window. Returns whether another frame is wanted at once.
    pub fn frame_offscreen(&mut self, size: [u32; 2]) -> Result<bool, RuntimeError> {
        if self
            .offscreen
            .as_ref()
            .is_none_or(|target| target.size() != size)
        {
            self.offscreen = Some(ScreenTarget::new(&self.core, size)?);
        }
        let (ui, repaint) = self.update(size)?;
        let painter = self
            .painter
            .get_or_insert_with(|| ScreenPainter::offscreen(&self.core));
        if let Some(target) = &self.offscreen {
            let frame = screen_frame(self.screen.canvas(), &self.shell, ui);
            self.core.render_screen(painter, target, frame)?;
        }
        Ok(repaint)
    }

    /// The last offscreen frame, read back.
    pub fn read_offscreen(&self) -> Result<Option<RgbaImage>, RuntimeError> {
        match &self.offscreen {
            Some(target) => Ok(Some(target.read(&self.core)?)),
            None => Ok(None),
        }
    }

    /// The canvas as evaluated so far, read back: the artwork's pixels without the UI.
    pub fn read_canvas(&self) -> Result<Option<RgbaImage>, RuntimeError> {
        match self.screen.canvas() {
            Some(canvas) => Ok(Some(canvas.target().read(&self.core)?)),
            None => Ok(None),
        }
    }

    /// One offscreen frame of `size` pixels, read back.
    pub fn screenshot(&mut self, size: [u32; 2]) -> Result<RgbaImage, RuntimeError> {
        self.frame_offscreen(size)?;
        let missing = RenderError::Invalid("no offscreen frame was drawn");
        self.read_offscreen()?.ok_or(RuntimeError::Render(missing))
    }

    /// Blocks until the GPU has finished every submitted frame, for timing.
    pub fn wait_idle(&self) -> Result<(), RuntimeError> {
        Ok(self.core.wait_idle()?)
    }

    /// Adapter name and backend, for logs and harness output.
    #[must_use]
    pub fn adapter(&self) -> String {
        self.core.adapter()
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
        self.screen
            .canvas()
            .map(|canvas| canvas.session().projection())
            .unwrap_or_default()
    }

    /// What the whole screen offers: the canvas, the gallery, or nothing yet.
    #[must_use]
    pub fn screen(&self) -> ScreenProjection {
        self.screen.projection()
    }

    /// True while the frame loop waits for something without input: the storage worker, or a
    /// canvas about to open. Frames keep coming meanwhile, so the result shows.
    #[must_use]
    pub fn storing(&self) -> bool {
        self.storage.busy() || matches!(self.screen, Screen::Opening(_))
    }

    /// Applies everything the storage worker finished, then opens a canvas that is ready.
    fn receive(&mut self) -> Result<(), RuntimeError> {
        while let Some(done) = self.storage.take() {
            self.screen.answer(done);
        }
        self.screen.open(&self.core, self.shell.canvas_area())
    }

    /// Everything before drawing: storage results, input, UI, intents, evaluation, storage
    /// calls. Asks for another frame when the screen changed what it offers, so it never
    /// shows a stale state while idle, and while waiting for the storage worker.
    fn update(&mut self, size: [u32; 2]) -> Result<(UiFrame, bool), RuntimeError> {
        self.receive()?;
        let batches = std::mem::take(&mut self.pending);
        let mut events = pointer_events(&batches, self.pixels_per_point);
        events.append(&mut self.typed);
        let shown = self.screen.projection();
        let pass = run_ui(
            &self.ui,
            UiPass {
                shell: &mut self.shell,
                projection: &shown,
                size,
                pixels_per_point: self.pixels_per_point,
                events,
            },
        );
        self.keyboard = pass.keyboard;
        let intents = self.shell.route(&batches);
        self.screen.open(&self.core, self.shell.canvas_area())?;
        match &mut self.screen {
            Screen::Canvas(canvas) => canvas.dispatch(intents),
            Screen::Gallery(gallery) => dispatch_gallery(gallery, intents),
            Screen::Opening(_) => {}
        }
        let area = self.shell.canvas_area();
        self.screen.progress(&self.core, &mut self.storage, area)?;
        let changed = self.screen.projection() != shown;
        Ok((pass.frame, pass.repaint || changed || self.storing()))
    }
}

/// The gallery's intents in order; anything else from a gallery frame is a stray.
fn dispatch_gallery(gallery: &mut openpaint_app::Gallery, intents: Vec<openpaint_app::Intent>) {
    for intent in intents {
        let result = match intent {
            openpaint_app::Intent::Gallery(intent) => gallery.dispatch(intent),
            other => {
                log::debug!("ignored in the gallery: {other:?}");
                Ok(())
            }
        };
        if let Err(error) = result {
            log::warn!("intent refused: {error}");
        }
    }
}

/// What one UI pass needs.
struct UiPass<'a> {
    shell: &'a mut Shell,
    projection: &'a ScreenProjection,
    /// The screen in pixels.
    size: [u32; 2],
    pixels_per_point: f32,
    events: Vec<egui::Event>,
}

/// What one UI pass gives.
struct UiOutput {
    frame: UiFrame,
    /// egui wants the next frame immediately (animations, first-frame layout).
    repaint: bool,
    /// A text field has the focus and waits for typing.
    keyboard: bool,
}

fn screen_frame<'a>(canvas: Option<&'a Canvas>, shell: &Shell, ui: UiFrame) -> ScreenFrame<'a> {
    let placement = canvas.zip(shell.canvas_area());
    ScreenFrame {
        surround: design::SURROUND,
        canvas: placement.map(|(canvas, area)| canvas.placement(&area)),
        ui,
    }
}

/// One UI pass over the screen.
fn run_ui(ctx: &egui::Context, pass: UiPass<'_>) -> UiOutput {
    let UiPass {
        shell,
        projection,
        size,
        pixels_per_point,
        events,
    } = pass;
    let points = egui::vec2(size[0] as f32, size[1] as f32) / pixels_per_point;
    let mut input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, points)),
        events,
        ..Default::default()
    };
    input
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .native_pixels_per_point = Some(pixels_per_point);
    let output = ctx.run_ui(input, |ui| shell.show(ui, projection));
    let repaint = output
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .is_some_and(|viewport| viewport.repaint_delay.is_zero());
    let keyboard = output.platform_output.ime.is_some();
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    UiOutput {
        frame: UiFrame {
            primitives,
            textures: output.textures_delta,
            pixels_per_point: output.pixels_per_point,
        },
        repaint,
        keyboard,
    }
}
