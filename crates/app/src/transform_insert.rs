//! Inserting a picture (Procreate's Insert a photo, S26): the picture becomes a new layer above
//! the active one, in its group, named "Inserted image", and the Transform tool frames it at
//! once. Nothing is lifted: the patch is the picture itself, placed where `picture_place` puts
//! it, so a picture inside the canvas shows pixel for pixel until the hand moves it, and moving
//! it resamples the picture's pixels once. The layer, the picture and where it is placed are one
//! history step when the transform commits; undo at its first step takes both away. A pasted
//! copy (S40) comes the same way on a layer named "Pasted layer", as records: its pixels are
//! lifted as any transform's, and it is placed as a record.

use openpaint_core::{
    Artwork, ArtworkCommand, CanvasSize, Copied, CoreError, LayerCommand, LayerName, Patch,
    Placement,
};

use super::{Draft, TransformRefusal, Transforms};
use crate::error::AppError;
use crate::projection::{Availability, EditorAction};

/// The inserted picture's layer, as the layer panel names it.
const NAME: &str = "Inserted image";
/// A pasted copy's layer (S40).
const PASTED: &str = "Pasted layer";

/// Where a picture of `size` pixels lies on `canvas`: its origin and its size there. A picture
/// larger than the canvas on either side is fitted inside it with its aspect kept (each side at
/// least a pixel); a smaller one keeps its size; either is centred (Procreate). `None` for a
/// picture without pixels.
#[must_use]
pub fn picture_place(canvas: CanvasSize, size: [u32; 2]) -> Option<([u32; 2], [u32; 2])> {
    if size.contains(&0) {
        return None;
    }
    let limit = [canvas.width(), canvas.height()];
    let fitted = if size[0] <= limit[0] && size[1] <= limit[1] {
        size
    } else {
        let ratio = |axis: usize| f64::from(limit[axis]) / f64::from(size[axis]);
        let scale = ratio(0).min(ratio(1));
        [0, 1].map(|axis| {
            let side = (f64::from(size[axis]) * scale).round() as u32;
            side.clamp(1, limit[axis])
        })
    };
    let origin = [0, 1].map(|axis| (limit[axis] - fitted[axis]) / 2);
    Some((origin, fitted))
}

impl Draft {
    /// The history version the transform began on.
    pub(super) fn version(&self) -> &Artwork {
        self.from.as_ref().unwrap_or(&self.base)
    }

    /// The base with the content placed by `placement`: an inserted picture, and anything while
    /// the hand drags (not `settled`), as the pixels `patch`; anything else settled as a record.
    pub(super) fn placed(
        &self,
        patch: &Patch,
        placement: Placement,
        settled: bool,
    ) -> Result<Artwork, CoreError> {
        let lift = self.lift.clone();
        let pasted = self.from.is_some() && !self.picture;
        if settled && pasted && placement == Placement::IDENTITY {
            // A paste not moved is its layer as pasted: no record.
            return Ok(self.base.clone());
        }
        let applied = if self.picture || !settled {
            self.base.transform_region(lift, patch.clone(), placement)
        } else {
            self.base.place_region(lift, placement)
        };
        applied.map(|applied| applied.artwork)
    }
}

impl Transforms {
    /// The preview as the commit records it: settled equals committed (DECISIONS D57).
    pub(super) fn show_settled(&mut self) {
        let Some(draft) = self.draft.as_mut() else {
            return;
        };
        let Some((patch, _)) = &draft.lifted else {
            return;
        };
        if let Ok(shown) = draft.placed(patch, draft.preview, true) {
            draft.lifted = Some((patch.clone(), shown));
        }
    }

    /// Frames `picture` on a new layer above `artwork`'s active one; a transform in progress is
    /// committed by the caller first.
    pub(crate) fn insert(
        &mut self,
        artwork: &Artwork,
        drawing: bool,
        picture: Patch,
    ) -> Result<(), AppError> {
        if drawing {
            return Err(AppError::Unavailable {
                action: EditorAction::Transform,
                availability: Availability::StrokeInProgress,
            });
        }
        let placed = layered(artwork, NAME).and_then(|base| {
            let lift = None;
            let shown = base
                .transform_region(lift, picture.clone(), Placement::IDENTITY)
                .map_err(|_| TransformRefusal::Failed)?;
            Ok((base, shown.artwork))
        });
        let (base, shown) = match placed {
            Ok(placed) => placed,
            Err(refusal) => {
                self.refused = Some(refusal);
                return Err(AppError::TransformRefused(refusal));
            }
        };
        self.refused = None;
        self.draft = Some(Draft {
            from: Some(artwork.clone()),
            base,
            lift: None,
            lifted: Some((picture, shown)),
            picture: true,
            handed: true,
            steps: vec![Placement::IDENTITY],
            at: 0,
            preview: Placement::IDENTITY,
        });
        Ok(())
    }

    /// Frames `copied` on a new layer above `onto`'s active one (S40, Paste); `from` is the
    /// history version now, which undo at the first step goes back to (`onto` may hold a cut
    /// of it). A transform in progress is committed by the caller first.
    pub(crate) fn paste(
        &mut self,
        from: &Artwork,
        onto: &Artwork,
        copied: &Copied,
    ) -> Result<(), AppError> {
        let pasted = layered(onto, PASTED).and_then(|layered| {
            let applied = layered.paste(copied);
            applied
                .map(|applied| applied.artwork)
                .map_err(|_| TransformRefusal::Failed)
        });
        let base = match pasted {
            Ok(base) => base,
            Err(refusal) => {
                self.refused = Some(refusal);
                return Err(AppError::TransformRefused(refusal));
            }
        };
        self.refused = None;
        self.draft = Some(Draft {
            from: Some(from.clone()),
            base,
            lift: None,
            lifted: None,
            picture: false,
            handed: false,
            steps: vec![Placement::IDENTITY],
            at: 0,
            preview: Placement::IDENTITY,
        });
        Ok(())
    }
}

/// `artwork` with a layer added above its active one, named `name` and active.
fn layered(artwork: &Artwork, name: &str) -> Result<Artwork, TransformRefusal> {
    let refusal = |error: CoreError| match error {
        CoreError::LayerLimit { .. } => TransformRefusal::Full,
        _ => TransformRefusal::Failed,
    };
    let created = artwork
        .apply(ArtworkCommand::Layer(LayerCommand::Create))
        .map_err(refusal)?
        .artwork;
    let layer = created.active();
    let name = LayerName::new(name).map_err(refusal)?;
    let named = created
        .apply(ArtworkCommand::Layer(LayerCommand::Rename { layer, name }))
        .map_err(refusal)?
        .artwork;
    // A layer in a hidden group would hold a picture no one sees.
    if named.shown(layer) {
        Ok(named)
    } else {
        Err(TransformRefusal::Hidden)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(width: u32, height: u32) -> CanvasSize {
        CanvasSize::new(width, height).expect("size")
    }

    #[test]
    fn a_smaller_picture_keeps_its_size_centred() {
        let place = picture_place(canvas(400, 300), [101, 50]);
        assert_eq!(place, Some(([149, 125], [101, 50])));
        assert_eq!(
            picture_place(canvas(400, 300), [400, 300]),
            Some(([0, 0], [400, 300])),
            "the canvas's own size fills it"
        );
    }

    #[test]
    fn a_larger_picture_fits_inside_with_its_aspect_kept() {
        // Wider: the width limits; 4000 x 1000 to 400 x 100, centred top to bottom.
        assert_eq!(
            picture_place(canvas(400, 300), [4000, 1000]),
            Some(([0, 100], [400, 100]))
        );
        // Taller: the height limits; 1500 x 2000 to 225 x 300.
        assert_eq!(
            picture_place(canvas(400, 300), [1500, 2000]),
            Some(([87, 0], [225, 300]))
        );
        // Wider than the canvas only: 500 x 10 to 400 x 8.
        assert_eq!(
            picture_place(canvas(400, 300), [500, 10]),
            Some(([0, 146], [400, 8]))
        );
        // A sliver keeps at least a pixel.
        assert_eq!(
            picture_place(canvas(400, 300), [100_000, 1]),
            Some(([0, 149], [400, 1]))
        );
        assert_eq!(picture_place(canvas(400, 300), [0, 10]), None);
    }
}
