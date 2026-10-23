//! The eyedropper on the open canvas: what a picking contact previews, read back from the
//! canvas target, and on a practice sheet what its originals let it read.

use openpaint_app::{Contact, ContactPhase, PracticePick, ToolIntent, Toolbox};
use openpaint_core::Color;
use openpaint_render::Core;

use super::Canvas;

impl Canvas {
    /// The eyedropper: a contact on the canvas (pen or finger) previews the colour the canvas
    /// shows under it, read back from the target (render picking), as it touches down and
    /// moves; the loupe shows it. Lifting picks the last colour previewed, never one at the
    /// release point: it joins the recent colours and the tool used before comes back. A
    /// cancelled contact ends the eyedropper and keeps the colour. On a practice sheet an
    /// original is read only once checked, a swatch as its colour.
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
        match self.session_mut().practice_pick(sample.position) {
            PracticePick::Canvas => {}
            PracticePick::Refused => return,
            PracticePick::Colour(color) => {
                return toolbox.dispatch_tool(&ToolIntent::Preview(color));
            }
        }
        // Tiles still waiting would read as they were.
        if let Err(error) = self.settle(core) {
            log::error!("evaluating the canvas before picking failed: {error}");
        }
        let [x, y] = sample.position.map(f32::floor);
        let picked = if x >= 0.0 && y >= 0.0 {
            self.target().pick(core, [x as u32, y as u32])
        } else {
            Ok(None)
        };
        match picked {
            Ok(Some([r, g, b, _])) => {
                let channel = |value: u8| f32::from(value) / 255.0;
                match Color::rgb(channel(r), channel(g), channel(b)) {
                    Ok(color) => toolbox.dispatch_tool(&ToolIntent::Preview(color)),
                    Err(error) => log::error!("picked no colour: {error}"),
                }
            }
            // Off the canvas: nothing to preview there; the last preview stays.
            Ok(None) => {}
            Err(error) => log::error!("picking failed: {error}"),
        }
    }
}
