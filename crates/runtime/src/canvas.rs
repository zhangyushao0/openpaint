//! The open canvas: the app session, the target its artwork is evaluated into, where that
//! target shows on screen, and whether the user is leaving it for the gallery.

use openpaint_app::{Intent, Session, Thumbnail, Toolbox};
use openpaint_core::{Artwork, CanvasSize, Color, StrokeStyle};
use openpaint_render::{
    CanvasPlacement, CanvasTarget, Core, FrameRequest, PixelRect, RegionEvaluator, changed_regions,
};
use openpaint_ui::ScreenArea;

use crate::error::RuntimeError;

/// The paper under the layers until artworks carry their own background.
const PAPER: Color = Color::WHITE;
/// Largest gallery thumbnail in pixels: a gallery card's picture at density 1.75.
pub(crate) const THUMBNAIL_MAX: [u32; 2] = [448, 336];

/// What a canvas opens with.
#[derive(Debug, Clone)]
pub(crate) enum Content {
    /// An artwork as the library holds it.
    Stored(Artwork),
    /// A blank artwork the size of the canvas area, not in the library yet.
    Blank,
    /// As `Blank`, after the last artwork could not be opened.
    BlankAfterFailedOpen,
}

/// Leaving the canvas for the gallery: the artwork is stored first, and the canvas stays until
/// that succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Leaving {
    Staying,
    /// Asked to leave; waits for a save still running.
    Waiting,
    /// The last version and the thumbnail are on their way to the store.
    Storing,
}

pub(crate) struct Canvas {
    session: Session,
    target: CanvasTarget,
    /// False until the whole canvas has been evaluated once.
    evaluated: bool,
    /// The version shown before this frame's first change other than a stroke (undo, redo, a
    /// layer edit), until it is evaluated.
    changed_from: Option<Artwork>,
    pub(crate) leaving: Leaving,
}

impl Canvas {
    /// `content` shown at one screen pixel per canvas pixel from the area's corner; a blank
    /// artwork is the size of `area`. `None` while the area is empty. Later area changes
    /// (rotation) move the canvas, never resize it.
    pub(crate) fn open(
        core: &Core,
        area: &ScreenArea,
        content: &Content,
        brush: StrokeStyle,
    ) -> Result<Option<Self>, RuntimeError> {
        if area.size.contains(&0) {
            return Ok(None);
        }
        let session = match content {
            Content::Stored(artwork) => Session::open(artwork.clone(), brush),
            Content::Blank => Session::new(blank_size(area)?, brush),
            Content::BlankAfterFailedOpen => Session::after_failed_open(blank_size(area)?, brush),
        };
        let canvas = session.artwork().canvas();
        let size = [canvas.width(), canvas.height()];
        log::info!("canvas opened at {} x {}", size[0], size[1]);
        Ok(Some(Self {
            session,
            target: CanvasTarget::new(core, size)?,
            evaluated: false,
            changed_from: None,
            leaving: Leaving::Staying,
        }))
    }

    /// Applies the frame's intents in order. A refused intent is logged and skipped: one bad
    /// sample must not stop the frame. A gallery intent the session allows starts leaving;
    /// while leaving, nothing else is applied. Brush intents go to `toolbox`, and the session
    /// draws its next stroke with the style that results.
    pub(crate) fn dispatch(&mut self, intents: Vec<Intent>, toolbox: &mut Toolbox) {
        for intent in intents {
            if self.leaving != Leaving::Staying {
                return;
            }
            if let Intent::Brush(brush) = &intent {
                toolbox.dispatch(brush);
                match toolbox.style() {
                    Ok(style) => self.session.set_brush(style),
                    Err(error) => log::error!("no brush style: {error}"),
                }
                continue;
            }
            // Committing a stroke is followed by the evaluator itself (D17).
            let stroke = matches!(intent, Intent::Stroke(_));
            let gallery = matches!(intent, Intent::Gallery(_));
            let before = self.session.artwork().clone();
            match self.session.dispatch(intent) {
                Ok(()) if gallery => self.leaving = Leaving::Waiting,
                Ok(()) if !stroke => {
                    self.changed_from.get_or_insert(before);
                }
                Ok(()) => {}
                Err(error) => log::warn!("intent refused: {error}"),
            }
        }
    }

    /// Brings the target up to date. Committing the stroke being drawn is followed by the
    /// evaluator itself (D17); any other change passes the pixels that differ between the version
    /// shown before it and the current one; the first evaluation covers the whole canvas.
    pub(crate) fn evaluate(&mut self, core: &Core) -> Result<(), RuntimeError> {
        let mut regions = if self.evaluated {
            Vec::new()
        } else {
            vec![PixelRect::full(self.target.size())]
        };
        if let Some(before) = self.changed_from.take() {
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

    /// The gallery picture of what the canvas shows now; call after `evaluate`.
    pub(crate) fn thumbnail(&self, core: &Core) -> Result<Thumbnail, RuntimeError> {
        let image = core.thumbnail(&self.target, THUMBNAIL_MAX)?;
        let size = image.size();
        Thumbnail::new(size, image.into_pixels()).ok_or(RuntimeError::Render(
            openpaint_render::RenderError::Invalid("a thumbnail of the wrong length"),
        ))
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

/// The size of a blank artwork that fills `area`.
pub(crate) fn blank_size(area: &ScreenArea) -> Result<CanvasSize, RuntimeError> {
    Ok(CanvasSize::new(area.size[0], area.size[1])?)
}
