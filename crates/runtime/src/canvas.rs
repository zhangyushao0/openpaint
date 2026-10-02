//! The open canvas: the app session, the target its artwork is evaluated into, and where that
//! target shows on screen.

use openpaint_app::{Intent, Session};
use openpaint_core::{Artwork, CanvasSize, Color};
use openpaint_render::{
    CanvasPlacement, CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, changed_regions,
};
use openpaint_ui::ScreenArea;

use crate::error::RuntimeError;
use crate::storage::Opened;

/// The paper under the layers until artworks carry their own background.
const PAPER: Color = Color::WHITE;

pub(crate) struct Canvas {
    session: Session,
    target: CanvasTarget,
    /// False until the whole canvas has been evaluated once.
    evaluated: bool,
    /// The version shown before this frame's first undo or redo, until it is evaluated.
    jumped_from: Option<Artwork>,
}

impl Canvas {
    /// The artwork the library `opened`, or a blank one the size of `area`, shown at one screen
    /// pixel per canvas pixel from the area's corner. `None` while the area is empty. Later
    /// area changes (rotation) move the canvas, never resize it.
    pub(crate) fn open(
        core: &Core,
        area: &ScreenArea,
        opened: &Opened,
    ) -> Result<Option<Self>, RuntimeError> {
        if area.size.contains(&0) {
            return Ok(None);
        }
        let blank = || CanvasSize::new(area.size[0], area.size[1]);
        let session = match opened {
            Opened::Last(artwork) => Session::open(artwork.clone())?,
            Opened::Empty => Session::new(blank()?)?,
            Opened::Failed(_) => Session::after_failed_open(blank()?)?,
        };
        let canvas = session.artwork().canvas();
        let size = [canvas.width(), canvas.height()];
        log::info!("canvas opened at {} x {}", size[0], size[1]);
        Ok(Some(Self {
            session,
            target: CanvasTarget::new(core, size)?,
            evaluated: false,
            jumped_from: None,
        }))
    }

    /// Applies the frame's intents in order. A refused intent is logged and skipped: one bad
    /// sample must not stop the frame.
    pub(crate) fn dispatch(&mut self, intents: Vec<Intent>) {
        for intent in intents {
            let jump = matches!(intent, Intent::History(_));
            let before = self.session.artwork().clone();
            match self.session.dispatch(intent) {
                Ok(()) if jump => {
                    self.jumped_from.get_or_insert(before);
                }
                Ok(()) => {}
                Err(error) => log::warn!("intent refused: {error}"),
            }
        }
    }

    /// Brings the target up to date. Committing the stroke being drawn is followed by the
    /// evaluator itself (D17); a history jump passes the pixels that differ between the version
    /// shown before it and the current one; the first evaluation covers the whole canvas.
    pub(crate) fn evaluate(&mut self, core: &Core) -> Result<(), RuntimeError> {
        let mut regions = if self.evaluated {
            Vec::new()
        } else {
            vec![PixelRect::full(self.target.size())]
        };
        if let Some(before) = self.jumped_from.take() {
            regions.extend(changed_regions(&before, self.session.artwork()));
        }
        let request = FrameRequest {
            artwork: self.session.artwork(),
            active: self.session.active(),
            background: PAPER,
        };
        RegionEvaluator::new(core, &mut self.target).evaluate(&request, &regions)?;
        self.evaluated = true;
        Ok(())
    }

    /// The target with its top-left corner at the canvas area's.
    pub(crate) fn placement(&self, area: &ScreenArea) -> CanvasPlacement<'_> {
        CanvasPlacement {
            target: &self.target,
            origin: area.origin,
        }
    }

    pub(crate) fn session(&self) -> &Session {
        &self.session
    }

    pub(crate) fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    pub(crate) fn target(&self) -> &CanvasTarget {
        &self.target
    }
}
