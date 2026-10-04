//! Adjustments (Procreate's Adjustments on a layer, S28): choosing one from the Adjustments
//! menu starts it on the active layer (its mask when that is the target), inside the selection
//! when there is one, changing nothing yet. Each value the hand sets makes the preview at once:
//! the artwork with the adjustment recorded (`Artwork::adjust_region`), which the evaluator
//! shows like any edit, so what shows is what Apply keeps. Apply records it as one history step
//! (nothing when it changes nothing); Cancel and undo leave the artwork as it was; any other
//! edit or tool applies it as it shows (D54).

use std::fmt;

use openpaint_core::{Adjustment, AdjustmentKind, Artwork, RegionMask};

use crate::error::AppError;
use crate::history::History;
use crate::projection::{Availability, EditorAction};

#[derive(Debug, Clone, PartialEq)]
pub enum AdjustIntent {
    /// Start adjusting the active layer with `kind`, changing nothing yet.
    Begin(AdjustmentKind),
    /// The values the hand set now (of the kind begun): the preview.
    Set(Adjustment),
    /// Keep the adjustment as it shows: one history step.
    Apply,
    /// Drop the adjustment; the artwork stays as it was.
    Cancel,
    /// Close the notice of a refusal.
    Dismiss,
}

/// Why an adjustment did not start, for the notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustRefusal {
    /// The active layer is hidden (by its eye or its group's).
    Hidden,
    /// The active layer is locked.
    Locked,
}

impl fmt::Display for AdjustRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Hidden => "the layer is hidden",
            Self::Locked => "the layer is locked",
        })
    }
}

/// What the UI shows of adjusting.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AdjustProjection {
    /// The adjustment under way and its values now, until it is applied or cancelled.
    pub adjustment: Option<Adjustment>,
    pub refused: Option<AdjustRefusal>,
}

#[derive(Debug)]
struct Draft {
    /// The version adjusted; the draft ends if the current version changes.
    base: Artwork,
    clip: Option<RegionMask>,
    adjustment: Adjustment,
    /// The artwork with the adjustment, `None` while it changes nothing.
    preview: Option<Artwork>,
}

#[derive(Debug, Default)]
pub(crate) struct Adjusts {
    draft: Option<Draft>,
    refused: Option<AdjustRefusal>,
}

impl Adjusts {
    /// True while an adjustment is under way.
    pub(crate) const fn active(&self) -> bool {
        self.draft.is_some()
    }

    /// Starts `kind` on `artwork`'s active layer, inside `clip` when given.
    pub(crate) fn begin(
        &mut self,
        artwork: &Artwork,
        drawing: bool,
        kind: AdjustmentKind,
        clip: Option<&RegionMask>,
    ) -> Result<(), AppError> {
        if drawing {
            return Err(AppError::Unavailable {
                action: EditorAction::Adjust,
                availability: Availability::StrokeInProgress,
            });
        }
        let layer = artwork.active();
        let props = artwork.layer(layer).map(|layer| *layer.props());
        self.refused = match props {
            Some(_) if !artwork.shown(layer) => Some(AdjustRefusal::Hidden),
            Some(props) if props.locked() => Some(AdjustRefusal::Locked),
            _ => None,
        };
        if let Some(refusal) = self.refused {
            self.draft = None;
            return Err(AppError::AdjustRefused(refusal));
        }
        self.draft = Some(Draft {
            base: artwork.clone(),
            clip: clip.cloned(),
            adjustment: Adjustment::none(kind),
            preview: None,
        });
        Ok(())
    }

    /// New values for the adjustment under way, shown at once; another kind's are ignored.
    pub(crate) fn set(&mut self, adjustment: Adjustment) {
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        if adjustment.kind() != draft.adjustment.kind() || adjustment == draft.adjustment {
            return;
        }
        draft.preview = if adjustment.is_none() {
            None
        } else {
            let applied = draft
                .base
                .adjust_region(draft.clip.clone(), adjustment.clone());
            applied.ok().map(|applied| applied.artwork)
        };
        draft.adjustment = adjustment;
    }

    /// Ends the adjustment keeping what shows: one history step, unless it changes nothing or
    /// the version moved on.
    pub(crate) fn apply(&mut self, history: &mut History) {
        let Some(draft) = self.draft.take() else {
            return;
        };
        if let Some(shown) = draft
            .preview
            .filter(|_| draft.base.same_version(history.current()))
        {
            history.record(shown);
        }
    }

    /// Cancel and undo: the artwork stays as it was.
    pub(crate) fn cancel(&mut self) {
        self.draft = None;
    }

    /// The notice goes.
    pub(crate) const fn dismiss(&mut self) {
        self.refused = None;
    }

    /// The preview to show over `current`.
    pub(crate) fn shown<'a>(&'a self, current: &Artwork) -> Option<&'a Artwork> {
        let draft = self.draft.as_ref()?;
        draft.base.same_version(current).then_some(())?;
        draft.preview.as_ref()
    }

    pub(crate) fn projection(&self) -> AdjustProjection {
        AdjustProjection {
            adjustment: self.draft.as_ref().map(|draft| draft.adjustment.clone()),
            refused: self.refused,
        }
    }
}
