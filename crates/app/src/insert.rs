//! Insert image… (Procreate's Insert a photo): Actions asks the system's picker for one PNG or
//! JPEG image, the transfer worker reads it, turns it upright and fits it inside the canvas
//! (`InsertJob`), and the picture goes to the open session as `TransformIntent::Insert`, framed
//! on a new layer. A cancelled picker changes nothing; a file that cannot be used is a notice
//! with OK. The picker and the reading are the reference's (`ImagePick`); only where the
//! picture goes differs.

use std::fmt;

use openpaint_core::{CanvasSize, Patch};

use crate::reference::ImagePick;
use crate::transfer::{HostAnswer, ImageError, ImageSource};

/// What the user asks of inserting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertIntent {
    /// Choose a picture with the system's picker; it becomes a new layer.
    Choose,
    /// Close the notice that a picture could not be inserted.
    Dismiss,
}

/// The chosen picture, to read, turn upright and fit inside `canvas` (premultiplied, centred:
/// `picture_place`).
pub struct InsertJob {
    pub source: Box<dyn ImageSource>,
    pub canvas: CanvasSize,
}

impl fmt::Debug for InsertJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InsertJob({:?})", self.canvas)
    }
}

#[derive(Debug, Default)]
pub(crate) struct Insert {
    /// The picker, open for a canvas of this size, and the reading.
    pub(crate) pick: ImagePick<CanvasSize>,
    /// Read and fitted, for the open session to take.
    picture: Option<Patch>,
}

impl Insert {
    /// The platform's answer while the picker is open: the picture is read next; cancelling
    /// changes nothing.
    pub(crate) fn answer(&mut self, answer: HostAnswer) -> Option<InsertJob> {
        let (source, canvas) = self.pick.answer(answer)?;
        Some(InsertJob { source, canvas })
    }

    /// The chosen picture was read and fitted, or could not be.
    pub(crate) fn opened(&mut self, result: Result<Patch, ImageError>) {
        if let Some(picture) = self.pick.opened(result) {
            self.picture = Some(picture);
        }
    }

    pub(crate) const fn take(&mut self) -> Option<Patch> {
        self.picture.take()
    }
}
