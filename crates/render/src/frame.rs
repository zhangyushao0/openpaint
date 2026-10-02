//! `FrameRequest`: what one evaluation draws.

use openpaint_core::{ActiveStroke, Artwork, Color};

/// The artwork version to draw, the in-progress stroke beside it (DECISIONS D08: it never
/// enters the artwork) and the paper under the layers. Target pixels are canvas pixels.
#[derive(Debug, Clone, Copy)]
pub struct FrameRequest<'a> {
    pub artwork: &'a Artwork,
    /// Drawn on its layer above that layer's finished strokes.
    pub active: Option<&'a ActiveStroke>,
    /// Until the artwork carries its own background.
    pub background: Color,
}
