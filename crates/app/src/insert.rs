//! Insert image… (Procreate's Insert a photo, S26): Actions asks the system's picker for one PNG
//! or JPEG image, the transfer worker reads it, turns it upright and fits it inside the canvas
//! (`InsertJob`), and the picture goes to the open session as `TransformIntent::Insert`, framed
//! on a new layer. A cancelled picker changes nothing; a file that cannot be used is a notice
//! with OK. The picker and the reading are the reference's (`reference.rs`); only where the
//! picture goes differs.

use std::fmt;

use openpaint_core::{CanvasSize, Patch};

use crate::reference::{ReferenceNotice, picked};
use crate::transfer::{HostAnswer, HostRequest, ImageError, ImageSource};

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
enum State {
    #[default]
    Idle,
    /// The picker is open for a canvas of this size.
    Choosing(CanvasSize),
    Opening,
    Failed(ImageError),
}

#[derive(Debug, Default)]
pub(crate) struct Insert {
    state: State,
    /// Read and fitted, for the open session to take.
    picture: Option<Patch>,
}

impl Insert {
    /// Choosing a picture for a canvas of `canvas`; only called when no picker or transfer runs
    /// (`Transfers` checks).
    pub(crate) fn choose(&mut self, canvas: CanvasSize) -> HostRequest {
        self.state = State::Choosing(canvas);
        HostRequest::OpenImage
    }

    pub(crate) fn dismiss(&mut self) {
        if matches!(self.state, State::Failed(_)) {
            self.state = State::Idle;
        }
    }

    pub(crate) const fn waiting(&self) -> bool {
        matches!(self.state, State::Choosing(_))
    }

    pub(crate) const fn busy(&self) -> bool {
        matches!(self.state, State::Choosing(_) | State::Opening)
    }

    /// The platform's answer while `waiting`: the picture is read next; cancelling changes
    /// nothing.
    pub(crate) fn answer(&mut self, answer: HostAnswer) -> Option<InsertJob> {
        let State::Choosing(canvas) = self.state else {
            return None;
        };
        self.state = State::Idle;
        match picked(answer) {
            Ok(Some(source)) => {
                self.state = State::Opening;
                Some(InsertJob { source, canvas })
            }
            Ok(None) => None,
            Err(error) => {
                self.state = State::Failed(error);
                None
            }
        }
    }

    /// The chosen picture was read and fitted, or could not be.
    pub(crate) fn opened(&mut self, result: Result<Patch, ImageError>) {
        if !matches!(self.state, State::Opening) {
            return;
        }
        match result {
            Ok(picture) => {
                self.picture = Some(picture);
                self.state = State::Idle;
            }
            Err(error) => self.state = State::Failed(error),
        }
    }

    pub(crate) const fn take(&mut self) -> Option<Patch> {
        self.picture.take()
    }

    /// As the reference's: opening, or why it failed.
    pub(crate) fn notice(&self) -> ReferenceNotice {
        match &self.state {
            State::Idle => ReferenceNotice::None,
            State::Choosing(_) | State::Opening => ReferenceNotice::Opening,
            State::Failed(error) => ReferenceNotice::Failed(error.clone()),
        }
    }
}
