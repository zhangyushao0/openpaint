//! The eyedropper on the open canvas: what a picking contact previews, read back from the
//! canvas target, and on a practice sheet what its originals let it read; and a painted practice
//! copy's frame read back for Compare.

use openpaint_app::{Contact, ContactPhase, PictureImage, PracticePick, ToolIntent, Toolbox};
use openpaint_core::Color;
use openpaint_render::Core;

use super::Canvas;

impl Canvas {
    /// The eyedropper: a contact on the canvas (pen or finger) previews the colour the canvas
    /// shows under it, read back from the target (render picking), as it touches down and
    /// moves; the loupe shows it. Lifting picks the last colour previewed, never one at the
    /// release point: it joins the recent colours and the tool used before comes back. A
    /// cancelled contact ends the eyedropper and keeps the colour. On a practice sheet an
    /// original is read only once checked, a swatch as its colour; while a painted copy is
    /// compared, the point compares the original and the copy there (the copy read back).
    pub(super) fn pick(&mut self, core: &Core, contact: &Contact, toolbox: &mut Toolbox) {
        match contact.phase {
            ContactPhase::Down | ContactPhase::Move => {}
            ContactPhase::Up => return toolbox.dispatch_tool(&ToolIntent::Pick),
            ContactPhase::Cancel => return toolbox.dispatch_tool(&ToolIntent::Eyedropper(false)),
            ContactPhase::Hover | ContactPhase::Leave => return,
        }
        let Some(sample) = contact.samples.last() else {
            return;
        };
        let at = match self.session_mut().practice_pick(sample.position) {
            PracticePick::Canvas => sample.position,
            PracticePick::Refused => return,
            PracticePick::Colour(color) => {
                return toolbox.dispatch_tool(&ToolIntent::Preview(color));
            }
            PracticePick::Probe {
                copy,
                original,
                on_original,
            } => {
                let read = self.canvas_colour(core, copy);
                if let Some(rgb) = read {
                    self.session_mut().practice_probed(rgb);
                }
                let shown = if on_original {
                    Some(original)
                } else {
                    read.and_then(color)
                };
                if let Some(shown) = shown {
                    toolbox.dispatch_tool(&ToolIntent::Preview(shown));
                }
                return;
            }
        };
        // Off the canvas: nothing to preview there; the last preview stays.
        if let Some(picked) = self.canvas_colour(core, at).and_then(color) {
            toolbox.dispatch_tool(&ToolIntent::Preview(picked));
        }
    }

    /// The colour the canvas shows at `at` (canvas pixels), every waiting tile evaluated first;
    /// none off the canvas or when the read fails (logged).
    fn canvas_colour(&mut self, core: &Core, at: [f32; 2]) -> Option<[u8; 3]> {
        // Tiles still waiting would read as they were.
        if let Err(error) = self.settle(core) {
            log::error!("evaluating the canvas before picking failed: {error}");
        }
        let [x, y] = at.map(f32::floor);
        if x < 0.0 || y < 0.0 {
            return None;
        }
        match self.target().pick(core, [x as u32, y as u32]) {
            Ok(picked) => picked.map(|[r, g, b, _]| [r, g, b]),
            Err(error) => {
                log::error!("picking failed: {error}");
                None
            }
        }
    }

    /// A painted practice copy's frame asked for by Compare, read back from the target (every
    /// waiting tile evaluated first) and handed to the sheet; a read that fails hands none.
    pub(crate) fn practice_reads(&mut self, core: &Core) {
        while let Some(job) = self.session_mut().practice_read_request() {
            if let Err(error) = self.settle(core) {
                log::error!("evaluating the canvas before reading a copy failed: {error}");
            }
            let read = self.target().read_region(core, job.origin, job.size);
            let pixels = read
                .inspect_err(|error| log::error!("reading a practice copy failed: {error}"))
                .ok()
                .map(|image| PictureImage {
                    size: image.size(),
                    pixels: image.into_pixels(),
                });
            self.session_mut()
                .practice_read(job.sequence, pixels.as_ref());
        }
    }
}

fn color([r, g, b]: [u8; 3]) -> Option<Color> {
    let channel = |value: u8| f32::from(value) / 255.0;
    Color::rgb(channel(r), channel(g), channel(b))
        .inspect_err(|error| log::error!("picked no colour: {error}"))
        .ok()
}
