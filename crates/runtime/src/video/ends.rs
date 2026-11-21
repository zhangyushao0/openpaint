//! The end a video holds (S163): the finished picture (a segment's too), or with the caption the
//! end card, the line the composition root laid painted by a painter of its own into a
//! `ScreenTarget` of the band's size (`openpaint_ui::video_caption`) below the picture.

use openpaint_app::VideoError;
use openpaint_core::DrawingOrder;
use openpaint_render::{
    Core, FrameRequest, RenderError, RgbaImage, ScreenFrame, ScreenPainter, ScreenTarget, UiFrame,
    VideoTarget,
};
use openpaint_ui::caption_band;
use openpaint_ui::design::SURROUND;

use super::rendering;
use crate::canvas::PAPER;

/// The end held: the finished picture, and the card once made.
pub(super) struct Ends {
    /// The line laid for the card, until it is painted.
    pub(super) caption: Option<UiFrame>,
    pub(super) finished: Option<RgbaImage>,
    pub(super) card: Option<RgbaImage>,
}

impl Ends {
    /// A frame of the end: the card with the line, else the finished picture ("Stop on the
    /// finished picture": a segment's video too ends on the whole drawing, whose line the card
    /// tells). The card holds the picture the target drew last: the finished one.
    pub(super) fn end(
        &mut self,
        core: &Core,
        target: &mut VideoTarget,
        drawing: &DrawingOrder,
    ) -> Result<&RgbaImage, VideoError> {
        if self.finished.is_none() {
            let finished = drawing.frame(drawing.steps().len(), 0);
            let request = FrameRequest {
                artwork: &finished.artwork,
                active: None,
                background: PAPER,
            };
            self.finished = Some(target.frame(core, &request).map_err(rendering)?.clone());
        }
        if let Some(caption) = self.caption.take() {
            let band = band(core, caption, caption_band(target.size())).map_err(rendering)?;
            self.card = Some(target.card(core, (SURROUND, &band)).map_err(rendering)?);
        }
        self.card
            .as_ref()
            .or(self.finished.as_ref())
            .ok_or(VideoError::Render)
    }
}

impl Drop for Ends {
    /// A line never painted (the video cancelled or failed before its end) is let go: its
    /// textures are its own context's, which is gone, and epaint asserts every delta is handled.
    fn drop(&mut self) {
        if let Some(caption) = &mut self.caption {
            caption.textures.clear();
        }
    }
}

/// The caption's band of `size` pixels: the desk with the line painted over it.
fn band(core: &Core, caption: UiFrame, size: [u32; 2]) -> Result<RgbaImage, RenderError> {
    let mut painter = ScreenPainter::offscreen(core);
    let target = ScreenTarget::new(core, size)?;
    let frame = ScreenFrame {
        surround: SURROUND,
        canvas: None,
        ui: caption,
    };
    core.render_screen(&mut painter, &target, frame)?;
    target.read(core)
}
