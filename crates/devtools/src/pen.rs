//! Which end of the pen a scenario's pen step uses and which side button it holds (S41): the
//! runner gives the step's batches that button state or the eraser end, as Android reports a
//! pen whose button is down (`BUTTON_STYLUS_PRIMARY`, `BUTTON_STYLUS_SECONDARY`) or turned
//! round (`TOOL_TYPE_ERASER`).

use openpaint_runtime::{MotionBatch, ToolKind};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pen {
    /// The tip, no button held.
    #[default]
    Tip,
    /// The tip with the first side button held.
    Primary,
    /// The tip with the second side button held.
    Secondary,
    /// The eraser end.
    Eraser,
}

/// `batches` (a pen step's) as `pen` makes them.
pub(crate) fn held(mut batches: Vec<MotionBatch>, pen: Pen) -> Vec<MotionBatch> {
    for batch in &mut batches {
        let stylus = batch.pointers.iter().any(|p| p.tool == ToolKind::Stylus);
        if !stylus {
            continue;
        }
        match pen {
            Pen::Tip => {}
            Pen::Primary => batch.buttons.stylus_primary = true,
            Pen::Secondary => batch.buttons.stylus_secondary = true,
            Pen::Eraser => {
                for pointer in &mut batch.pointers {
                    if pointer.tool == ToolKind::Stylus {
                        pointer.tool = ToolKind::Eraser;
                    }
                }
            }
        }
    }
    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use openpaint_runtime::{Buttons, MotionPhase, Pointer};

    fn pen_batch(tool: ToolKind) -> MotionBatch {
        MotionBatch {
            phase: MotionPhase::Move,
            buttons: Buttons::default(),
            pointers: vec![Pointer {
                id: 0,
                tool,
                samples: Vec::new(),
            }],
        }
    }

    #[test]
    fn a_button_or_the_eraser_end_goes_to_the_pens_batches_only() {
        let both = || vec![pen_batch(ToolKind::Stylus), pen_batch(ToolKind::Finger)];
        let primary = held(both(), Pen::Primary);
        assert!(primary[0].buttons.stylus_primary && !primary[0].buttons.stylus_secondary);
        assert_eq!(primary[1].buttons, Buttons::default(), "a palm has none");
        let secondary = held(both(), Pen::Secondary);
        assert!(secondary[0].buttons.stylus_secondary);
        let eraser = held(both(), Pen::Eraser);
        assert_eq!(eraser[0].pointers[0].tool, ToolKind::Eraser);
        assert_eq!(eraser[1].pointers[0].tool, ToolKind::Finger);
        assert_eq!(held(both(), Pen::Tip), both());
    }
}
