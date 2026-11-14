//! What is in hand beside the artwork ends by one table: before an intent applies, `Ends::by`
//! says what it ends and `Session::end` ends it, always in one order. A family's own intents end
//! its own draft in its own handler (an adjustment's Apply, a transform's Tool off, a fill's
//! Commit); this table is what every other intent ends (Procreate: any other edit or tool keeps
//! an adjustment as it shows and commits a transform where it shows).

use super::Session;
use crate::adjust::AdjustIntent;
use crate::intent::Intent;
use crate::selection::SelectionIntent;
use crate::stroke::StrokeIntent;
use crate::transform::TransformIntent;

/// What an intent ends before it applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Ends {
    /// The shape snapped last is done with.
    shape: bool,
    /// An adjustment under way is kept as it shows; its notice goes.
    adjustment: bool,
    /// A transform is committed where it shows; its notice goes.
    transform: bool,
    /// A fill being placed is dropped; its notices go.
    fill: bool,
    /// A layer's opacity touch ends: its next value is a new step.
    slide: bool,
    /// The Selection tool is left; its open path closes.
    selecting: bool,
}

/// The table's cells: ended, kept.
const Y: bool = true;
const N: bool = false;

impl Ends {
    /// The copy menu's commands (`Session::clipboard`, after its check), as any edit.
    pub(super) const EDIT: Self = Self::row([Y, Y, Y, Y, Y, N]);

    /// A step of a mode's own (`Session::lend`): the shape, an adjustment, a transform.
    pub(super) const STEP: Self = Self::row([Y, Y, Y, N, N, N]);

    /// One row, in the fields' order: shape, adjustment, transform, fill, slide, selecting.
    const fn row([shape, adjustment, transform, fill, slide, selecting]: [bool; 6]) -> Self {
        Self {
            shape,
            adjustment,
            transform,
            fill,
            slide,
            selecting,
        }
    }

    /// What `intent` ends; `transforming` while the Transform tool is on (read before `end`).
    /// A "kept" under a family's own draft is its handler's to end.
    pub(super) const fn by(intent: &Intent, transforming: bool) -> Self {
        #[rustfmt::skip]
        let cells = match intent {
            //                                    shape adjust transform fill slide selecting
            Intent::View(_) =>                      [N, N, N, N, N, N],
            // The shape stays editable through saving.
            Intent::Storage(_) =>                   [N, N, N, N, Y, N],
            // Undo cancels an adjustment and steps a transform: their own (`Session::step`).
            Intent::History(_) =>                   [Y, N, N, Y, Y, N],
            // The shape's own steps.
            Intent::Stroke(
                StrokeIntent::EditShape(true)
                | StrokeIntent::MoveHandle { .. }
                | StrokeIntent::Hold { .. },
            ) =>                                    [N, Y, Y, Y, Y, N],
            Intent::Stroke(_)
            | Intent::Selection(_)
            | Intent::Gallery(_) =>                 [Y, Y, Y, Y, Y, N],
            Intent::Layer(_) =>                     [Y, Y, Y, Y, N, N],
            Intent::Fill(_) =>                      [Y, Y, Y, N, Y, N],
            Intent::Adjust(AdjustIntent::Begin(_)) => [Y, N, Y, Y, Y, Y],
            Intent::Adjust(_) =>                    [Y, N, Y, N, N, N],
            Intent::Transform(TransformIntent::Tool(true)) if !transforming
                                                 => [Y, Y, N, Y, N, Y],
            Intent::Transform(TransformIntent::Insert(_)) => [Y, Y, N, Y, Y, Y],
            Intent::Transform(_) =>                 [Y, Y, N, N, N, N],
            // The app's own, which the runtime hands elsewhere; never dispatched here.
            Intent::Brush(_)
            | Intent::Tool(_)
            | Intent::Export(_)
            | Intent::Reference(_)
            | Intent::Insert(_)
            | Intent::Clipboard(_) =>               [Y, Y, Y, N, Y, N],
        };
        Self::row(cells)
    }
}

impl Session {
    /// The one transition: what `ends` names ends, in this order.
    pub(super) fn end(&mut self, ends: Ends) {
        if ends.shape {
            self.strokes.end_shape();
        }
        if ends.adjustment {
            self.apply_adjustment();
        }
        if ends.transform {
            self.commit_transform();
            self.transforms.forget_refusal();
        }
        if ends.fill {
            self.fills.interrupt();
        }
        if ends.slide {
            self.layers.interrupt();
        }
        if ends.selecting {
            self.select(SelectionIntent::Tool(false));
        }
    }
}
