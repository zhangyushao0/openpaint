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

    /// True while something on screen moves (D44) or the view tiers fill in (S32); frames are
    /// wanted until it rests.
    #[must_use]
    pub fn moving(&self) -> bool {
        self.shell.moving() || self.screen.canvas().is_some_and(Canvas::view_pending)
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
