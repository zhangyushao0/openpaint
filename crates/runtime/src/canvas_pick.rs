//! The eyedropper on the open canvas: what a picking contact previews, read back from the
//! canvas target, and on a practice sheet what its originals let it read (a painted copy's frame
//! is read off the frame loop: `practice/compare.rs`).

use openpaint_app::{Contact, ContactPhase, ToolIntent, Toolbox};
use openpaint_core::Color;
use openpaint_practice_app::PracticePick;
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
        let pick = crate::practice::sheet(self).map(|sheet| sheet.pick(sample.position));
        let at = match pick.unwrap_or(PracticePick::Canvas) {
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
                if let (Some(rgb), Some(sheet)) = (read, crate::practice::sheet(self)) {
                    sheet.probed(rgb);
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
}

fn color([r, g, b]: [u8; 3]) -> Option<Color> {
    let channel = |value: u8| f32::from(value) / 255.0;
    Color::rgb(channel(r), channel(g), channel(b))
        .inspect_err(|error| log::error!("picked no colour: {error}"))
        .ok()
}
