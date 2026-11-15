//! What a painted copy's reading brings for the screen beside its comparison
//! (`openpaint_practice::Comparison`, whose marks and card the screen draws, S109): the pictures its
//! marks lay (`Source::Read`, a key shift's frames in grey, S107).

use std::sync::Arc;

use openpaint_practice::{Comparison, Metrics};

use crate::pictures::PictureImage;

/// A painted copy as read: its comparison, the numbers a first Compare keeps and the pictures its
/// reading brought, by their place (`Source::Read`).
#[derive(Debug, Clone, PartialEq)]
pub struct CopyRead {
    pub comparison: Comparison,
    pub metrics: Metrics,
    pub pictures: Arc<Vec<PictureImage>>,
}
