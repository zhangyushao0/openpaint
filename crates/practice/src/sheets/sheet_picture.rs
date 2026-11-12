//! A pack's picture on a practice sheet (S80, S99): as its pack holds it (its size, its outline,
//! its value masses, its colour areas and the credit to show under it), filling its frame of its
//! shape. What a kind's Compare and painted picture read of it (S109b).

use std::sync::Arc;

use crate::colour_areas::areas::Areas;
use crate::compositions::value_masses::ValueMasses;
use crate::maths::path::Point;
use crate::packs::pack::{ItemRef, Pack, PackItem};
use crate::sheets::layout::PictureFit;

/// A pack's picture on the sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetPicture {
    pub item: ItemRef,
    /// The picture's size, in its pack's pixels.
    pub size: [u32; 2],
    /// Where it lies in its frame.
    pub fit: PictureFit,
    /// Its outline's closed lines in frame units; none where its pack has none.
    pub outline: Option<Vec<Vec<Point>>>,
    /// The credit that shows small under it: an Attribution licence's.
    pub credit: Option<String>,
    /// Its value masses (a composition's whose masses read, S96); none where its pack has none.
    pub masses: Option<MassesShown>,
    /// Its colour areas (a crop's, S101), in shares of the picture; none for another picture.
    pub areas: Option<Arc<Areas>>,
}

/// A picture's value masses as its frame shows them: the answer, and its mass edges in frame
/// units (where the picture lies in its frame).
#[derive(Debug, Clone, PartialEq)]
pub struct MassesShown {
    pub answer: Arc<ValueMasses>,
    pub edges: Vec<Vec<Point>>,
}

impl SheetPicture {
    /// The picture `item` names, as `packs` hold it, filling its frame of its own shape (S99);
    /// `None` when no pack kept holds it.
    #[must_use]
    pub fn of(item: &ItemRef, packs: &[Pack]) -> Option<Self> {
        let found: &PackItem = packs.iter().find_map(|pack| pack.find(item))?;
        let fit = PictureFit::of(found.size);
        let outline = (found.outline.as_ref()).map(|outline| {
            (outline.lines().into_iter())
                .map(|line| line.iter().map(|p| fit.to_frame(*p)).collect())
                .collect()
        });
        let masses = found.masses.as_ref().map(|answer| {
            let [w, h] = fit.size;
            let at = |[u, v]: Point| [u * w, v * h];
            MassesShown {
                edges: (answer.edges().into_iter())
                    .map(|edge| edge.into_iter().map(at).collect())
                    .collect(),
                answer: Arc::clone(answer),
            }
        });
        Some(Self {
            item: item.clone(),
            size: found.size,
            fit,
            outline,
            credit: found.shown_credit().map(str::to_owned),
            masses,
            areas: found.areas.clone(),
        })
    }

    /// Its colour areas and the share of its frame its picture fills; none for a picture without.
    #[must_use]
    pub fn areas_in_frame(&self) -> Option<(&Areas, Point)> {
        Some((self.areas.as_deref()?, self.fit.size))
    }
}
