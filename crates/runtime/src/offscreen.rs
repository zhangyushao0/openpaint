//! The harness's side of the frame loop: frames into an offscreen target instead of a window,
//! reading them and the canvas back, and the fixed clock that makes motion deterministic in
//! scenarios (D44).

use openpaint_core::{ActiveStroke, Artwork};
use openpaint_render::{RenderError, RgbaImage, ScreenPainter, ScreenTarget};
use openpaint_ui::ControlMap;

use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::frame::{Runtime, screen_frame};

impl Runtime {
    /// Runs one frame into an offscreen target of `size` pixels, for the harness, which never
    /// attaches a window. Returns whether another frame is wanted at once. A lost GPU device is
    /// made again as for the window (S49).
    pub fn frame_offscreen(&mut self, size: [u32; 2]) -> Result<bool, RuntimeError> {
        self.recover_if_lost()?;
        let result = self.offscreen_once(size);
        self.after_frame(result)
    }

    fn offscreen_once(&mut self, size: [u32; 2]) -> Result<bool, RuntimeError> {
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

    /// Holds the clock motion runs on at `ms` until the next call, instead of the wall clock:
    /// the harness steps it, so every run moves alike.
    pub fn fix_clock(&mut self, ms: i64) {
        self.frame_clock.fix(ms);
    }

    /// Every stroke from now on takes `seed` for its stamps (S45), instead of one of its own:
    /// the harness fixes it, so a scenario's hashes hold.
    pub fn fix_seed(&mut self, seed: u32) {
        self.seeds.fix(seed);
    }

    /// True while something on screen moves (D44), the view tiers fill in (S32) or the canvas
    /// does (S49); frames are wanted until it rests.
    #[must_use]
    pub fn moving(&self) -> bool {
        let filling = |canvas: &Canvas| canvas.view_pending() || canvas.filling();
        self.shell.moving() || self.screen.canvas().is_some_and(filling)
    }

    /// Destroys the GPU device under the running app, as a driver reset loses it, for tests and
    /// the scenario runner (S49); the next frame makes the renderer again. Returns whether wgpu's
    /// lost callback reported the loss (else it was marked by hand, `Core::destroy_device`).
    pub fn lose_device(&self) -> bool {
        self.core.destroy_device()
    }

    /// What the open canvas's derived caches hold on the GPU now (S49), pictures included.
    #[must_use]
    pub fn cache_bytes(&self) -> Option<openpaint_render::CacheBytes> {
        let canvas = self.screen.canvas()?;
        Some(openpaint_render::CacheBytes {
            pictures: self.shell.picture_bytes(),
            ..canvas.bytes()
        })
    }

    /// What the open canvas's history holds beyond its oldest version, in bytes (S50, the
    /// harness's long session; bounded by `HISTORY_BUDGET`).
    #[must_use]
    pub fn history_bytes(&self) -> Option<usize> {
        let canvas = self.screen.canvas()?;
        Some(canvas.session().history().total_bytes())
    }

    /// The GPU memory budget from the next frame on, in place of the device class's (S49), for
    /// tests.
    pub fn set_memory_budget(&mut self, bytes: u64) {
        self.budget.bytes = bytes;
    }

    /// The renderer's GPU state now (a new one after a loss, S49).
    #[must_use]
    pub fn core(&self) -> std::sync::Arc<openpaint_render::Core> {
        std::sync::Arc::clone(&self.core)
    }

    /// Every control the last frame laid out, by name, in screen pixels: what scenario steps
    /// name instead of coordinates.
    #[must_use]
    pub fn controls(&self) -> &ControlMap {
        self.shell.controls()
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

    /// The brush editor's preview strip as the worker made it last, while the editor shows (S46).
    #[must_use]
    pub fn read_strip(&self) -> Option<RgbaImage> {
        self.brushes.strip().cloned()
    }

    /// What the canvas evaluates now: the artwork shown (with a stroke's mirrored copies or a
    /// preview) and the stroke being drawn, for tests to evaluate cold.
    #[must_use]
    pub fn drawn(&self) -> Option<(Artwork, Option<ActiveStroke>)> {
        let session = self.screen.canvas()?.session();
        Some((session.shown().clone(), session.active().cloned()))
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
}
