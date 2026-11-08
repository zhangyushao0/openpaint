//! What a painted copy's reading brings for the screen beside its comparison
//! (`openpaint_practice::Comparison`, whose marks and card the screen draws, S109): a key shift's
//! frames in grey for the Grey view (S107), the original's none where its picture was not read.

use std::sync::Arc;

use openpaint_practice::{Comparison, Metrics};

use crate::pictures::PictureImage;

/// Both frames in grey, as Oklab lightness.
#[derive(Debug, Clone, PartialEq)]
pub struct Greys {
    pub copy: Arc<PictureImage>,
    pub original: Option<Arc<PictureImage>>,
}

/// A painted copy as read: its comparison, the numbers a first Compare keeps and, for a key shift,
/// its frames in grey.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyRead {
    pub comparison: Comparison,
    pub metrics: Metrics,
    pub greys: Option<Arc<Greys>>,
}
