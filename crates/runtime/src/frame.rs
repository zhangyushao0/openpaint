//! `Runtime`: the frame loop a platform host drives. The host attaches, resizes and detaches
//! its window, hands over input batches and asks for frames; all calls come from one thread.
//!
//! A frame: take the input normalised since the last frame -> run the UI from the session's
//! projection -> route the canvas contacts and the chrome's taps to the session as intents ->
//! evaluate what changed into the canvas target -> draw the canvas and the UI over it.

use openpaint_app::{InputBatch, SessionProjection};
use openpaint_core::Artwork;
use openpaint_render::{
    Core, RenderError, RgbaImage, ScreenFrame, ScreenPainter, ScreenTarget, Surface, SurfaceWindow,
    UiFrame,
};
use openpaint_ui::{Shell, design};

use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::input::MotionBatch;
use crate::normalise::Normaliser;
use crate::pointer::pointer_events;

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
    /// Input normalised since the last frame, oldest first.
    pending: Vec<InputBatch>,
    /// Opened by the first frame that knows the canvas area.
    canvas: Option<Canvas>,
}

impl Runtime {
    /// Opens the GPU device and installs the theme. `pixels_per_point` is the display density
    /// (Android's `DisplayMetrics.density`): physical pixels per UI point.
    pub fn start(pixels_per_point: f32) -> Result<Self, RuntimeError> {
        let core = Core::headless()?;
        log::info!("GPU adapter: {}", core.adapter());
        let ui = egui::Context::default();
        design::install(&ui);
        Ok(Self {
            core,
            ui,
            shell: Shell::default(),
            painter: None,
            surface: None,
            offscreen: None,
            pixels_per_point,
            normaliser: Normaliser::default(),
            pending: Vec::new(),
            canvas: None,
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

    /// Takes one platform input batch for the next frame. Returns whether it holds anything,
    /// in which case the host should ask for a frame.
    pub fn motion(&mut self, batch: &MotionBatch) -> bool {
        log::debug!(target: "openpaint::input", "{batch}");
        match self.normaliser.normalise(batch) {
            Some(input) => {
                self.pending.push(input);
                true
            }
            None => false,
        }
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
        let frame = screen_frame(self.canvas.as_ref(), &self.shell, ui);
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
            let frame = screen_frame(self.canvas.as_ref(), &self.shell, ui);
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
        match &self.canvas {
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

    /// The open artwork, once a frame has opened the canvas.
    #[must_use]
    pub fn artwork(&self) -> Option<&Artwork> {
        self.canvas
            .as_ref()
            .map(|canvas| canvas.session().artwork())
    }

    /// The open session's projection; before the canvas opens, nothing is available.
    fn projection(&self) -> SessionProjection {
        self.canvas
            .as_ref()
            .map(|canvas| canvas.session().projection())
            .unwrap_or_default()
    }

    /// Everything before drawing: input, UI, intents, evaluation. Asks for another frame when
    /// the intents changed what the UI offers, so it never shows a stale state while idle.
    fn update(&mut self, size: [u32; 2]) -> Result<(UiFrame, bool), RuntimeError> {
        let batches = std::mem::take(&mut self.pending);
        let events = pointer_events(&batches, self.pixels_per_point);
        let shown = self.projection();
        let (ui, repaint) = run_ui(
            &self.ui,
            UiPass {
                shell: &mut self.shell,
                projection: &shown,
                size,
                pixels_per_point: self.pixels_per_point,
                events,
            },
        );
        let intents = self.shell.route(&batches);
        if self.canvas.is_none()
            && let Some(area) = self.shell.canvas_area()
        {
            self.canvas = Canvas::open(&self.core, &area)?;
        }
        if let Some(canvas) = &mut self.canvas {
            canvas.dispatch(intents);
            canvas.evaluate(&self.core)?;
        }
        Ok((ui, repaint || self.projection() != shown))
    }
}

/// What one UI pass needs.
struct UiPass<'a> {
    shell: &'a mut Shell,
    projection: &'a SessionProjection,
    /// The screen in pixels.
    size: [u32; 2],
    pixels_per_point: f32,
    events: Vec<egui::Event>,
}

fn screen_frame<'a>(canvas: Option<&'a Canvas>, shell: &Shell, ui: UiFrame) -> ScreenFrame<'a> {
    let placement = canvas.zip(shell.canvas_area());
    ScreenFrame {
        surround: design::SURROUND,
        canvas: placement.map(|(canvas, area)| canvas.placement(&area)),
        ui,
    }
}

/// One UI pass over the screen. Returns the tessellated frame and whether egui wants the next
/// frame immediately (animations, first-frame layout).
fn run_ui(ctx: &egui::Context, pass: UiPass<'_>) -> (UiFrame, bool) {
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
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    let frame = UiFrame {
        primitives,
        textures: output.textures_delta,
        pixels_per_point: output.pixels_per_point,
    };
    (frame, repaint)
}
