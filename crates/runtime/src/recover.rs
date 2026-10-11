//! Device-loss recovery (S49, DECISIONS D76). A GPU reset loses the device: wgpu's lost callback
//! marks it (`Core::lost`), and the window's surface, a GPU wait or any call on the device fails.
//! Before each frame, and after a frame that failed so, the runtime makes the renderer again: a
//! new device and its pipelines (`Core::reopened`), handed to the workers (`SharedCore`); the
//! window's surface made again from the window it was attached with (else at the next attach);
//! the painter with every UI texture uploaded again; the canvas's target and view tiers, which
//! the next frames evaluate anew from the artwork. The artwork, history, toolbox, session and
//! the stroke being drawn are app state and stay as they are: the stroke goes on from its first
//! sample on the new device, nothing the hand did is lost. A quiet notice tells the display was
//! reset. At most three rebuilds a minute; after that the failure is the frame's error.

use std::sync::Arc;
use std::time::{Duration, Instant};

use openpaint_render::{Core, RenderError, Surface};

use crate::error::RuntimeError;
use crate::frame::Runtime;

/// Rebuilds within this window count against the limit.
const WINDOW: Duration = Duration::from_secs(60);
/// The most rebuilds within it.
const MOST: usize = 3;

/// Makes the window's surface again on a new device: kept from `attach` until `detach`.
pub(crate) type Reattach = Box<dyn Fn(&Core, [u32; 2]) -> Result<Surface, RenderError> + Send>;

/// When the renderer was made again lately.
#[derive(Debug, Default)]
pub(crate) struct Recoveries {
    at: Vec<Instant>,
}

impl Recoveries {
    fn allowed(&mut self) -> bool {
        self.at.retain(|at| at.elapsed() < WINDOW);
        self.at.len() < MOST
    }
}

/// Whether `error` says the GPU failed: the device or its surface lost, a wait that never ended.
pub(crate) fn gpu_failed(error: &RuntimeError) -> bool {
    matches!(
        error,
        RuntimeError::Render(RenderError::Device(_) | RenderError::Timeout { .. })
    )
}

impl Runtime {
    /// Before a frame: a device found lost is made again.
    pub(crate) fn recover_if_lost(&mut self) -> Result<(), RuntimeError> {
        match self.core.lost() {
            Some(why) => self.recover(&why),
            None => Ok(()),
        }
    }

    /// After a frame: when it failed for the GPU (or the device was lost meanwhile), the renderer
    /// is made again and another frame is wanted; any other result stands.
    pub(crate) fn after_frame(
        &mut self,
        result: Result<bool, RuntimeError>,
    ) -> Result<bool, RuntimeError> {
        match result {
            Err(error) if gpu_failed(&error) || self.core.lost().is_some() => {
                self.recover(&error.to_string())?;
                Ok(true)
            }
            other => other,
        }
    }

    /// Makes the renderer again on a new device; see the module.
    fn recover(&mut self, why: &str) -> Result<(), RuntimeError> {
        if !self.recoveries.allowed() {
            let message = format!("the GPU keeps failing ({why}); no more rebuilds this minute");
            return Err(RuntimeError::Render(RenderError::Device(message)));
        }
        let started = Instant::now();
        log::error!("the GPU failed ({why}); making the renderer again");
        let size = self.surface.as_ref().map(Surface::size);
        // The old swapchain goes before the window takes a new one.
        self.surface = None;
        self.offscreen = None;
        let core = Arc::new(self.core.reopened()?);
        core.catch_errors();
        self.shared.replace(Arc::clone(&core));
        self.core = core;
        if let (Some(window), Some(size)) = (&self.window, size) {
            match window(&self.core, size) {
                Ok(surface) => self.surface = Some(surface),
                Err(error) => log::error!("no surface on the new device yet: {error}"),
            }
        }
        self.painter = self
            .painter
            .take()
            .map(|painter| painter.moved(&self.core, self.surface.as_ref()));
        if let Some(canvas) = self.screen.canvas_mut() {
            canvas.rebuild(&self.core)?;
        }
        self.brushes.redo();
        self.shell.display_reset();
        self.recoveries.at.push(Instant::now());
        log::info!(
            "the renderer was made again in {} ms on {}",
            started.elapsed().as_millis(),
            self.core.adapter()
        );
        Ok(())
    }
}
