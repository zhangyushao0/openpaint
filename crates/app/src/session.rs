//! `Session`: the open artwork and the editor flow around it. The UI emits intents, the
//! session applies them; the renderer reads the artwork and the in-progress stroke beside it.

use openpaint_core::{ActiveStroke, Artwork, CanvasSize};

use crate::error::AppError;
use crate::intent::Intent;
use crate::stroke::Strokes;

#[derive(Debug)]
pub struct Session {
    artwork: Artwork,
    strokes: Strokes,
}

impl Session {
    /// A blank artwork of `canvas` pixels.
    ///
    /// # Errors
    /// `Core` when the default brush is invalid (a programming error).
    pub fn new(canvas: CanvasSize) -> Result<Self, AppError> {
        Ok(Self {
            artwork: Artwork::new(canvas),
            strokes: Strokes::new()?,
        })
    }

    /// Applies one intent. On error the session is as it was before, except that a stroke
    /// whose lift failed to extend is still committed with the samples it had.
    ///
    /// # Errors
    /// `Core` when the artwork refuses the edit (a locked layer, an invalid sample).
    pub fn dispatch(&mut self, intent: Intent) -> Result<(), AppError> {
        match intent {
            Intent::Stroke(stroke) => self.strokes.handle(&mut self.artwork, stroke),
        }
    }

    /// The current version. While a stroke is drawn it is the version the stroke started on.
    #[must_use]
    pub fn artwork(&self) -> &Artwork {
        &self.artwork
    }

    /// The stroke being drawn, which is not in the artwork until it is committed.
    #[must_use]
    pub fn active(&self) -> Option<&ActiveStroke> {
        self.strokes.active()
    }
}
