//! What the canvas showed in the last pass (`Shown`), for routing this frame's contacts and for
//! the pen's outline.

use openpaint_app::{ButtonMap, CanvasView, SessionProjection, TransformFrame};
use openpaint_core::{CanvasSize, Color};

/// What the canvas showed in the last pass, for routing this frame's contacts.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shown {
    pub(crate) canvas: CanvasSize,
    pub(crate) view: CanvasView,
    pub(crate) picking: bool,
    /// The Selection tool is on.
    pub(crate) selecting: bool,
    /// The Transform tool is on, and its frame once the content is lifted.
    pub(crate) transforming: bool,
    /// An adjustment is under way: the pen draws nothing.
    pub(crate) adjusting: bool,
    pub(crate) transform: Option<TransformFrame>,
    pub(crate) drawing: bool,
    /// The colour a drop carries and the threshold it starts with.
    pub(crate) color: Option<Color>,
    pub(crate) threshold: f32,
    /// The current tool's brush diameter in canvas pixels.
    pub(crate) brush_size: Option<f32>,
    /// What the pen's side buttons do.
    pub(crate) buttons: ButtonMap,
    /// A mode's sheet: its chrome holds the band's middle.
    pub(crate) mode: bool,
}

impl Shown {
    /// What `projection`'s canvas shows, while there is one (`mode` on a mode's sheet).
    pub(crate) fn of(projection: &SessionProjection, mode: bool) -> Option<Self> {
        let brush = projection.brush();
        // A mode's pen draws instead of the toolbox's brush: its size.
        let pen = projection.pen();
        projection.canvas().map(|canvas| Self {
            canvas,
            view: projection.view(),
            picking: brush.is_some_and(|brush| brush.picking),
            selecting: projection.selection().tool,
            transforming: projection.transform().tool,
            adjusting: projection.adjust().adjustment.is_some(),
            transform: projection.transform().frame,
            drawing: projection.drawing(),
            color: brush.map(|brush| brush.color),
            threshold: projection.fill().threshold,
            brush_size: pen.or(brush.map(|brush| brush.size)),
            buttons: brush.map(|brush| brush.buttons).unwrap_or_default(),
            mode,
        })
    }
}
