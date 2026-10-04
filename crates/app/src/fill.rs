//! Fill (Procreate's ColorDrop): the colour dropped on the canvas fills the connected region
//! around that point on the active layer. While the hand holds and slides, each new threshold
//! is a new request; the newest one runs (one at a time, on the runtime's fill worker, which
//! samples the layer and floods it with `openpaint-raster`), and the last finished preview
//! stays on screen until a newer one replaces it. Committing keeps the newest request's fill as
//! one history step; cancelling leaves the artwork as it was. A preview is never in history.

use std::fmt;

use openpaint_core::{Artwork, Color, LayerId, RegionMask};

use crate::error::AppError;
use crate::history::History;
use crate::projection::{Availability, EditorAction};

/// The threshold a fill starts with (draweave's tested default): 60 % of the full 8-bit range.
pub const DEFAULT_THRESHOLD: f32 = 0.6;

#[derive(Debug, Clone, PartialEq)]
pub enum FillIntent {
    /// Fill the region around canvas pixel `point` with `color`, at `threshold` (`0..=1`):
    /// shown as a preview once it is computed. Replaces the request before.
    Request {
        point: [f32; 2],
        threshold: f32,
        color: Color,
    },
    /// Keep the newest request's fill, as soon as it is computed: one history step.
    Commit,
    /// Drop the fill and its notice; the artwork stays as it was.
    Cancel,
}

/// Why a fill was refused, for the notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRefusal {
    /// The active layer is hidden: a fill there would not show.
    Hidden,
    /// The active layer is locked.
    Locked,
    /// A selection exists and the drop landed outside it, where nothing may change.
    OutsideSelection,
}

impl fmt::Display for FillRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Hidden => "the layer is hidden",
            Self::Locked => "the layer is locked",
            Self::OutsideSelection => "the drop is outside the selection",
        })
    }
}

/// A fill request's values.
#[derive(Debug, Clone, Copy)]
struct Request {
    point: [f32; 2],
    threshold: f32,
    color: Color,
}

/// Why a computed fill failed (sampling the layer on the GPU, or the region).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillFailed;

/// One fill for the worker: flood `layer` of `artwork` from `point` at `threshold`. While one
/// fill is adjusted its jobs name the same version and layer, so the worker samples it once.
#[derive(Debug, Clone)]
pub struct FillJob {
    pub sequence: u64,
    pub artwork: Artwork,
    pub layer: LayerId,
    pub point: [u32; 2],
    pub threshold: f32,
}

/// What the UI shows about filling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillProjection {
    /// The threshold of the newest request, or the one the next fill starts with.
    pub threshold: f32,
    /// A fill is being placed or adjusted.
    pub active: bool,
    /// Its newest request is not computed yet.
    pub pending: bool,
    /// The last request was refused.
    pub refused: Option<FillRefusal>,
    /// The last computed fill failed.
    pub failed: bool,
}

#[derive(Debug)]
struct Draft {
    /// The version the fill applies to; the draft ends if the current version changes.
    base: Artwork,
    layer: LayerId,
    point: [u32; 2],
    threshold: f32,
    color: Color,
    sequence: u64,
    /// The newest request was handed to the worker.
    handed: bool,
    /// The last computed fill, shown instead of `base`.
    preview: Option<(u64, Artwork)>,
    /// Commit once the newest request is computed.
    commit: bool,
    /// The selection when the fill was asked for: the fill is the flooded region inside it.
    clip: Option<RegionMask>,
}

#[derive(Debug)]
pub(crate) struct Fills {
    threshold: f32,
    draft: Option<Draft>,
    sequence: u64,
    /// The request on the worker and the colour it fills with.
    running: Option<(u64, Color)>,
    refused: Option<FillRefusal>,
    failed: bool,
}

impl Default for Fills {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_THRESHOLD,
            draft: None,
            sequence: 0,
            running: None,
            refused: None,
            failed: false,
        }
    }
}

impl Fills {
    /// Applies `intent`; while `clip` is selected a fill covers only its pixels, and a drop
    /// outside it is refused.
    pub(crate) fn handle(
        &mut self,
        history: &mut History,
        drawing: bool,
        intent: FillIntent,
        clip: Option<&RegionMask>,
    ) -> Result<(), AppError> {
        match intent {
            FillIntent::Request {
                point,
                threshold,
                color,
            } => {
                let request = Request {
                    point,
                    threshold,
                    color,
                };
                self.request(history.current(), drawing, request, clip)
            }
            FillIntent::Commit => {
                if let Some(draft) = self.draft.as_mut() {
                    draft.commit = true;
                }
                self.commit_ready(history);
                Ok(())
            }
            FillIntent::Cancel => {
                self.interrupt();
                Ok(())
            }
        }
    }

    fn request(
        &mut self,
        artwork: &Artwork,
        drawing: bool,
        Request {
            point,
            threshold,
            color,
        }: Request,
        clip: Option<&RegionMask>,
    ) -> Result<(), AppError> {
        if drawing {
            return Err(AppError::Unavailable {
                action: EditorAction::Fill,
                availability: Availability::StrokeInProgress,
            });
        }
        let canvas = artwork.canvas();
        let inside = |value: f32, side: u32| (0.0..side as f32).contains(&value);
        if !inside(point[0], canvas.width())
            || !inside(point[1], canvas.height())
            || !(0.0..=1.0).contains(&threshold)
        {
            return Err(AppError::Core(openpaint_core::CoreError::InvalidValue {
                what: "fill request",
            }));
        }
        let layer = artwork.active();
        let props = artwork.layer(layer).map(|layer| *layer.props());
        let point = point.map(|value| value as u32);
        let refusal = match props {
            // Hidden by its own eye or its group's (draweave cc51b74).
            Some(_) if !artwork.shown(layer) => Some(FillRefusal::Hidden),
            Some(props) if props.locked() => Some(FillRefusal::Locked),
            _ if clip.is_some_and(|clip| !clip.contains(point)) => {
                Some(FillRefusal::OutsideSelection)
            }
            _ => None,
        };
        self.refused = refusal;
        if let Some(refusal) = refusal {
            self.draft = None;
            return Err(AppError::FillRefused(refusal));
        }
        self.failed = false;
        self.threshold = threshold;
        self.sequence += 1;
        // Adjusting one fill keeps its last preview until the next one is computed.
        let same = self.draft.as_ref().is_some_and(|draft| {
            draft.base.same_version(artwork)
                && draft.layer == layer
                && draft.point == point
                && draft.clip.as_ref() == clip
        });
        let preview = self.draft.take().filter(|_| same).and_then(|d| d.preview);
        self.draft = Some(Draft {
            base: artwork.clone(),
            layer,
            point,
            threshold,
            color,
            sequence: self.sequence,
            handed: false,
            preview,
            commit: false,
            clip: clip.cloned(),
        });
        Ok(())
    }

    /// The newest request for the worker, when it has not been handed out and nothing runs.
    pub(crate) fn job(&mut self) -> Option<FillJob> {
        if self.running.is_some() {
            return None;
        }
        let draft = self.draft.as_mut().filter(|draft| !draft.handed)?;
        draft.handed = true;
        self.running = Some((draft.sequence, draft.color));
        Some(FillJob {
            sequence: draft.sequence,
            artwork: draft.base.clone(),
            layer: draft.layer,
            point: draft.point,
            threshold: draft.threshold,
        })
    }

    /// The worker computed request `sequence`. A fill for the version shown becomes the
    /// preview; the newest one is committed if that was asked for.
    pub(crate) fn finished(
        &mut self,
        history: &mut History,
        sequence: u64,
        result: Result<RegionMask, FillFailed>,
    ) {
        let Some((running, color)) = self.running.take().filter(|(s, _)| *s == sequence) else {
            return;
        };
        let current = history.current().clone();
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        if !draft.base.same_version(&current) {
            self.draft = None;
            return;
        }
        // Inside a selection, the flooded region's pixels that are selected.
        let clip = draft.clip.as_ref();
        let filled = result
            .ok()
            .and_then(|mask| clip.map_or(Some(mask.clone()), |clip| mask.intersect(clip)))
            .and_then(|mask| current.fill_region(mask, color).ok());
        match filled {
            Some(applied) => draft.preview = Some((running, applied.artwork)),
            None => {
                self.failed = true;
                if draft.commit && draft.sequence == running {
                    self.draft = None;
                }
                return;
            }
        }
        self.commit_ready(history);
    }

    /// Records the newest request's fill when it is computed and a commit was asked for.
    fn commit_ready(&mut self, history: &mut History) {
        let Some(draft) = self.draft.as_ref().filter(|draft| draft.commit) else {
            return;
        };
        if !draft.base.same_version(history.current()) {
            self.draft = None;
            return;
        }
        if let Some((sequence, artwork)) = &draft.preview
            && *sequence == draft.sequence
        {
            history.record(artwork.clone());
            self.draft = None;
        }
    }

    /// The preview to show over `current`, if the draft applies to it.
    pub(crate) fn shown<'a>(&'a self, current: &Artwork) -> Option<&'a Artwork> {
        let draft = self.draft.as_ref()?;
        draft.base.same_version(current).then_some(())?;
        draft.preview.as_ref().map(|(_, artwork)| artwork)
    }

    /// Another kind of edit came: a fill being placed is dropped and the notices go.
    pub(crate) fn interrupt(&mut self) {
        self.draft = None;
        self.refused = None;
        self.failed = false;
    }

    pub(crate) fn projection(&self) -> FillProjection {
        FillProjection {
            threshold: self.threshold,
            active: self.draft.is_some(),
            pending: self
                .draft
                .as_ref()
                .is_some_and(|d| d.preview.as_ref().is_none_or(|(s, _)| *s != d.sequence)),
            refused: self.refused,
            failed: self.failed,
        }
    }
}
