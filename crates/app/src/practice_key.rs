//! A key shift on a practice sheet (S107, V大's 变调; `practice_sheet.rs`): painted as a colour
//! combination is (its crop's areas, the painting pens, Fill frame, the eyedropper on its own
//! paint only: its original is never picked, `practice_pick.rs`), compared with no answer: its
//! copy is read on the compare worker (`ReadJob`) as a combination's areas are and judged by the
//! three checks (`openpaint_practice::KeyCheck`); the marked areas' outlines, the merged pairs'
//! edges and both frames in grey come with the reading (`KeyCompared`), the Grey view showing only
//! while compared.

use std::sync::Arc;

use openpaint_practice::{Comparison, Family, KeyCheck, KeyCompared, KeyRead, in_grey};

use super::read::{ReadAs, in_frame};
use super::{PracticeSheet, unavailable};
use crate::error::AppError;
use crate::practice_copy::PAPER;
use crate::practice_pictures::PictureImage;
use crate::practice_shown::{CopyRead, Greys};
use crate::projection::Availability;

/// A key shift's copy, its frame's `pixels`, read as `asked` (its crop's areas filling its extent
/// of the frame, shifted toward its tendency): what is off, the marks' lines, both frames in grey
/// (the crop's picture, where read); with the numbers a first Compare keeps.
pub(super) fn read_key(asked: &ReadAs, pixels: &PictureImage) -> CopyRead {
    let ReadAs::KeyShift {
        areas,
        extent,
        tendency,
        corners,
        original,
    } = asked
    else {
        return unread(asked);
    };
    let colours = areas.read(pixels.size, &pixels.pixels, PAPER).colours;
    let check = KeyCheck::of(areas, &colours, *tendency);
    let outlines = (0..areas.count())
        .filter(|area| check.off_lightness.contains(area) || check.unmoved.contains(area))
        .map(|area| (area, in_frame(areas.edges(area, None), *extent)))
        .collect();
    let edges = (check.merged.iter())
        .flat_map(|(a, b)| in_frame(areas.edges(*a, Some(*b)), *extent))
        .collect();
    let grey = |image: &PictureImage| {
        Arc::new(PictureImage {
            size: image.size,
            pixels: in_grey(&image.pixels),
        })
    };
    let metrics = check.metrics();
    let compared = KeyCompared {
        colours: areas.colours.clone(),
        corners: *corners,
        tendency: *tendency,
        read: Some(KeyRead {
            colours,
            check,
            outlines,
            edges,
        }),
    };
    CopyRead {
        comparison: Comparison::of(compared),
        metrics,
        greys: Some(Arc::new(Greys {
            copy: grey(pixels),
            original: original.as_deref().map(grey),
        })),
    }
}

/// What a key shift shows until its copy is read: nothing but its card's saying so.
fn unread(asked: &ReadAs) -> CopyRead {
    CopyRead {
        comparison: unread_key(asked),
        metrics: openpaint_practice::Metrics::new(),
        greys: None,
    }
}

/// A key shift asked as `asked`, not read yet.
pub(super) fn unread_key(asked: &ReadAs) -> Comparison {
    let ReadAs::KeyShift {
        areas,
        tendency,
        corners,
        ..
    } = asked
    else {
        return Comparison::of(openpaint_practice::PictureCompared {
            corners: [[0.0; 2]; 4],
            metrics: openpaint_practice::Metrics::new(),
        });
    };
    Comparison::of(KeyCompared {
        colours: areas.colours.clone(),
        corners: *corners,
        tendency: *tendency,
        read: None,
    })
}

impl PracticeSheet {
    /// Whether exercise `index` is a key shift.
    pub(super) fn key_shift(&self, index: usize) -> bool {
        self.family(index) == Some(Family::KeyShift)
    }

    /// What exercise `index`'s copy is read as, if it is a key shift whose crop a pack holds: its
    /// areas, its tendency, its picture's corners and its picture as read for the screen.
    pub(super) fn key_asked(&self, index: usize) -> Option<ReadAs> {
        let tendency = (*self.sheet.tendencies.get(index)?)?;
        let (areas, extent) = self.sheet.areas(index)?;
        let picture = self.sheet.pictures.get(index)?.as_ref()?;
        let original = (self.images.iter())
            .find(|(item, _)| *item == picture.item)
            .and_then(|(_, image)| image.clone());
        Some(ReadAs::KeyShift {
            areas: Arc::new(areas.clone()),
            extent,
            tendency,
            corners: picture.fit.corners(),
            original,
        })
    }

    /// The compared key shift's frames in grey (`on`) or in colour; refused on anything else.
    pub(super) fn grey(&mut self, on: bool) -> Result<(), AppError> {
        let key = self.current().is_some_and(|index| self.key_shift(index));
        if !key || self.compared.is_none() {
            return Err(unavailable(Availability::NothingLeft));
        }
        self.painting_state.grey = on;
        Ok(())
    }
}
