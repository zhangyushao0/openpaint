//! Copy and paste (S40, Procreate's Copy & Paste menu): Cut, Copy, Copy All, Duplicate,
//! Cut & Paste and Paste act on the selection, or with nothing selected on the whole active
//! layer (Copy All: on what every visible layer shows). What is copied is records (core's
//! `Copied`), held for the app's run by the runtime across artworks: never stored and not the
//! system's clipboard. Paste puts it on a new layer above the active one and frames it with the
//! Transform tool at once, so it can be placed; the layer and where it is placed are one
//! history step when the transform commits, Cut & Paste's cut included.

use openpaint_core::{Artwork, CanvasSize, Copied, MAX_LAYERS};

use crate::projection::Availability;

/// One of the menu's commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardAction {
    /// Takes the selection's content (or the layer's) out of the active layer and holds it.
    Cut,
    /// Holds the selection's content of the active layer (or the whole layer).
    Copy,
    /// Holds what every visible layer shows together inside the selection (or everywhere).
    CopyAll,
    /// Copy, then Paste, in one step.
    Duplicate,
    /// Cut, then Paste, in one step.
    CutPaste,
    /// Puts what is held on a new layer above the active one, framed to be placed.
    Paste,
}

impl ClipboardAction {
    /// Every command in the menu's order.
    pub const ALL: [Self; 6] = [
        Self::Cut,
        Self::Copy,
        Self::CopyAll,
        Self::Duplicate,
        Self::CutPaste,
        Self::Paste,
    ];

    /// The command's name in the menu.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Copy => "Copy",
            Self::CopyAll => "Copy All",
            Self::Duplicate => "Duplicate",
            Self::CutPaste => "Cut & Paste",
            Self::Paste => "Paste",
        }
    }

    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

/// What was copied last, for the app's run (the runtime owns it).
#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    held: Option<Copied>,
}

impl Clipboard {
    #[must_use]
    pub const fn held(&self) -> Option<&Copied> {
        self.held.as_ref()
    }

    pub(crate) fn hold(&mut self, copied: Copied) {
        self.held = Some(copied);
    }

    /// The size of the canvas what is held was copied from.
    #[must_use]
    pub fn canvas(&self) -> Option<CanvasSize> {
        self.held.as_ref().map(Copied::canvas)
    }
}

/// Which commands `artwork` offers now, Paste left out (`with_clipboard` adds it); `drawing`
/// while a stroke is drawn.
pub(crate) fn offered(artwork: &Artwork, drawing: bool) -> [Availability; 6] {
    let active = artwork.layer(artwork.active());
    let cuttable = active.is_some_and(|layer| {
        let paint = !layer.strokes().is_empty() || !layer.edits().is_empty();
        artwork.shown(layer.id()) && !layer.props().locked() && paint
    });
    let room = artwork.layers().len() < MAX_LAYERS;
    let copy = artwork.copyable(false);
    let possible = [
        cuttable,
        copy,
        artwork.copyable(true),
        copy && room,
        cuttable && room,
        false,
    ];
    possible.map(|possible| {
        if !possible {
            Availability::NothingLeft
        } else if drawing {
            Availability::StrokeInProgress
        } else {
            Availability::Available
        }
    })
}

/// Paste's availability onto a canvas of `canvas` with `room` (Add layer's) when `held` was
/// copied from a canvas of that size.
pub(crate) fn paste(
    held: Option<CanvasSize>,
    canvas: Option<CanvasSize>,
    room: Availability,
) -> Availability {
    if held.is_some() && held == canvas {
        room
    } else {
        Availability::NothingLeft
    }
}
