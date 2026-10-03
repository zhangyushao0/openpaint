//! Transform (Procreate's Transform, draweave's behaviour): the Transform button frames the
//! active layer's content, or the selection's, in a box the hand moves, scales by a corner and
//! turns by the handle above it; flips and a 45-degree turn are a tap. While the tool is on,
//! undo and redo walk its own steps (one per lifted hand or tap); turning the tool off, or
//! choosing another tool, commits the content where it shows as one history step. An
//! unchanged placement commits nothing.
//!
//! The pixels are lifted once when the tool turns on: the session hands the runtime a
//! `TransformJob`, the runtime samples the layer through the renderer and lifts its painted
//! pixels (`openpaint-raster`), and hands the patch back. Every preview is the artwork with
//! the transform added (`Artwork::transform_region`), so what shows while dragging is what is
//! committed (DECISIONS D45).

use std::f32::consts::FRAC_PI_4;
use std::fmt;

use openpaint_core::{Artwork, LayerId, Patch, Placement, RegionMask};

use crate::error::AppError;
use crate::history::History;
use crate::projection::{Availability, EditorAction};

#[derive(Debug, Clone, PartialEq)]
pub enum TransformIntent {
    /// The Transform tool on (the active layer's content, or the selection's, is framed) or
    /// off (it is committed where it shows).
    Tool(bool),
    /// While a hand moves, scales or turns the content: where it is placed now, from where it
    /// was.
    Place(Placement),
    /// The hand lifted: the placement now is one step.
    Settle,
    /// Mirrors the content left to right (or top to bottom) about the frame's centre.
    Flip { horizontal: bool },
    /// Turns the content 45 degrees clockwise about the frame's centre.
    Rotate,
    /// Puts the content back where it was.
    Reset,
}

/// Why the tool did not turn on, for the notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformRefusal {
    /// The active layer is hidden.
    Hidden,
    /// The active layer is locked.
    Locked,
    /// Nothing is painted there (inside the selection, if there is one).
    Empty,
    /// Sampling the layer failed.
    Failed,
}

impl fmt::Display for TransformRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Hidden => "the layer is hidden",
            Self::Locked => "the layer is locked",
            Self::Empty => "nothing to transform",
            Self::Failed => "the layer could not be sampled",
        })
    }
}

/// Sampling the layer for a transform failed on the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiftFailed;

/// The pixels to lift: `layer` of `artwork`, inside `clip` when one is selected.
#[derive(Debug, Clone)]
pub struct TransformJob {
    pub artwork: Artwork,
    pub layer: LayerId,
    pub clip: Option<RegionMask>,
}

/// The lifted content's rectangle (canvas pixels) and where it is placed now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformFrame {
    pub origin: [f32; 2],
    pub size: [f32; 2],
    pub placement: Placement,
}

impl TransformFrame {
    /// The rectangle's corners where they show: top left, top right, bottom right, bottom left.
    #[must_use]
    pub fn corners(&self) -> [[f32; 2]; 4] {
        let [x, y] = self.origin;
        let [w, h] = self.size;
        [[x, y], [x + w, y], [x + w, y + h], [x, y + h]].map(|p| self.placement.apply(p))
    }

    /// Its centre where it shows.
    #[must_use]
    pub fn centre(&self) -> [f32; 2] {
        let [x, y] = self.origin;
        let [w, h] = self.size;
        self.placement.apply([x + w / 2.0, y + h / 2.0])
    }
}

/// What the UI shows of the Transform tool.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TransformProjection {
    pub tool: bool,
    /// The frame, once the content is lifted.
    pub frame: Option<TransformFrame>,
    /// The content is placed elsewhere than it was: Reset is offered.
    pub placed: bool,
    pub refused: Option<TransformRefusal>,
}

#[derive(Debug)]
struct Draft {
    /// The version the transform applies to.
    base: Artwork,
    lift: Option<RegionMask>,
    /// The lifted pixels, once the runtime handed them back, and the preview with them.
    lifted: Option<(Patch, Artwork)>,
    handed: bool,
    /// Every settled placement; `at` is current; `preview` may be on its way to the next.
    steps: Vec<Placement>,
    at: usize,
    preview: Placement,
}

#[derive(Debug, Default)]
pub(crate) struct Transforms {
    draft: Option<Draft>,
    refused: Option<TransformRefusal>,
}

impl Transforms {
    pub(crate) const fn active(&self) -> bool {
        self.draft.is_some()
    }

    /// Frames `artwork`'s active layer, inside `clip` when given; the pixels come later.
    pub(crate) fn begin(
        &mut self,
        artwork: &Artwork,
        drawing: bool,
        clip: Option<&RegionMask>,
    ) -> Result<(), AppError> {
        if drawing {
            return Err(AppError::Unavailable {
                action: EditorAction::Transform,
                availability: Availability::StrokeInProgress,
            });
        }
        let props = artwork.layer(artwork.active()).map(|layer| *layer.props());
        self.refused = match props {
            Some(props) if !props.visible() => Some(TransformRefusal::Hidden),
            Some(props) if props.locked() => Some(TransformRefusal::Locked),
            _ => None,
        };
        if let Some(refusal) = self.refused {
            return Err(AppError::TransformRefused(refusal));
        }
        self.draft = Some(Draft {
            base: artwork.clone(),
            lift: clip.cloned(),
            lifted: None,
            handed: false,
            steps: vec![Placement::IDENTITY],
            at: 0,
            preview: Placement::IDENTITY,
        });
        Ok(())
    }

    /// The pixels to lift, once per transform.
    pub(crate) fn job(&mut self) -> Option<TransformJob> {
        let draft = self.draft.as_mut().filter(|draft| !draft.handed)?;
        draft.handed = true;
        Some(TransformJob {
            artwork: draft.base.clone(),
            layer: draft.base.active(),
            clip: draft.lift.clone(),
        })
    }

    /// The runtime lifted `patch` (`None`: nothing painted there) or failed.
    pub(crate) fn lifted(&mut self, patch: Result<Option<Patch>, LiftFailed>) {
        let Some(draft) = self.draft.as_mut().filter(|d| d.lifted.is_none()) else {
            return;
        };
        let patch = match patch {
            Ok(Some(patch)) => patch,
            Ok(None) => return self.refuse(TransformRefusal::Empty),
            Err(LiftFailed) => return self.refuse(TransformRefusal::Failed),
        };
        let lift = draft.lift.clone();
        match draft
            .base
            .transform_region(lift, patch.clone(), draft.preview)
        {
            Ok(applied) => draft.lifted = Some((patch, applied.artwork)),
            Err(_) => self.refuse(TransformRefusal::Failed),
        }
    }

    fn refuse(&mut self, refusal: TransformRefusal) {
        self.refused = Some(refusal);
        self.draft = None;
    }

    /// The content placed by `placement` from now on; ignored before it is lifted or when the
    /// placement leaves what an artwork holds.
    pub(crate) fn place(&mut self, placement: Placement) {
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        let Some((patch, _)) = &draft.lifted else {
            return;
        };
        let lift = draft.lift.clone();
        if let Ok(applied) = draft.base.transform_region(lift, patch.clone(), placement) {
            draft.preview = placement;
            draft.lifted = Some((patch.clone(), applied.artwork));
        }
    }

    /// The placement now becomes a step, unless it is the current one.
    pub(crate) fn settle(&mut self) {
        if let Some(draft) = self.draft.as_mut()
            && draft.preview != draft.steps[draft.at]
        {
            draft.steps.truncate(draft.at + 1);
            draft.steps.push(draft.preview);
            draft.at += 1;
        }
    }

    /// One tap's change about the frame's centre (flip, 45 degrees) or back to the start, as a
    /// step.
    pub(crate) fn step(&mut self, intent: &TransformIntent) {
        let Some(frame) = self.frame() else {
            return;
        };
        let centre = frame.centre();
        let turn = match intent {
            TransformIntent::Flip { horizontal } => {
                Placement::around(centre, 1.0, 0.0, [*horizontal, !*horizontal])
            }
            TransformIntent::Rotate => Placement::around(centre, 1.0, FRAC_PI_4, [false; 2]),
            _ => Some(Placement::IDENTITY),
        };
        let next = match intent {
            TransformIntent::Reset => Some(Placement::IDENTITY),
            _ => turn.and_then(|turn| frame.placement.then(turn)),
        };
        if let Some(next) = next {
            self.settle();
            self.place(next);
            self.settle();
        }
    }

    /// Undo inside the tool: a placement not settled yet goes first, then the step before.
    pub(crate) fn can_undo(&self) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|d| d.preview != d.steps[d.at] || d.at > 0)
    }

    pub(crate) fn can_redo(&self) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|d| d.preview == d.steps[d.at] && d.at + 1 < d.steps.len())
    }

    pub(crate) fn undo(&mut self) {
        let Some(draft) = self.draft.as_ref() else {
            return;
        };
        let back = if draft.preview == draft.steps[draft.at] {
            draft.at.saturating_sub(1)
        } else {
            draft.at
        };
        self.go(back);
    }

    pub(crate) fn redo(&mut self) {
        if self.can_redo()
            && let Some(at) = self.draft.as_ref().map(|d| d.at + 1)
        {
            self.go(at);
        }
    }

    fn go(&mut self, at: usize) {
        let Some(placement) = self.draft.as_ref().map(|d| d.steps[at]) else {
            return;
        };
        self.place(placement);
        if let Some(draft) = self.draft.as_mut() {
            draft.at = at;
        }
    }

    /// Ends the transform: the content where it shows becomes one history step, unless it is
    /// where it was or was never lifted. Returns whether a selection's content was committed.
    pub(crate) fn commit(&mut self, history: &mut History) -> bool {
        let Some(draft) = self.draft.take() else {
            return false;
        };
        let Some((_, shown)) = draft.lifted else {
            return false;
        };
        if draft.preview == Placement::IDENTITY || !draft.base.same_version(history.current()) {
            return false;
        }
        history.record(shown);
        draft.lift.is_some()
    }

    /// The preview to show over `current`.
    pub(crate) fn shown<'a>(&'a self, current: &Artwork) -> Option<&'a Artwork> {
        let draft = self.draft.as_ref()?;
        draft.base.same_version(current).then_some(())?;
        draft.lifted.as_ref().map(|(_, shown)| shown)
    }

    fn frame(&self) -> Option<TransformFrame> {
        let draft = self.draft.as_ref()?;
        let (patch, _) = draft.lifted.as_ref()?;
        let ([x, y], [w, h]) = (patch.origin(), patch.size());
        Some(TransformFrame {
            origin: [x as f32, y as f32],
            size: [w as f32, h as f32],
            placement: draft.preview,
        })
    }

    /// Another kind of edit came: the notice goes.
    pub(crate) const fn forget_refusal(&mut self) {
        self.refused = None;
    }

    pub(crate) fn projection(&self) -> TransformProjection {
        TransformProjection {
            tool: self.active(),
            frame: self.frame(),
            placed: self
                .draft
                .as_ref()
                .is_some_and(|d| d.preview != Placement::IDENTITY),
            refused: self.refused,
        }
    }
}
