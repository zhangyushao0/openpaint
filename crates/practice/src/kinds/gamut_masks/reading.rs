//! A gamut mask's copy compared (S153): read from its frame's pixels on the compare worker as a
//! combination's areas are (the key shift's reading as it stands) and judged by its three rules
//! (`MaskCheck`): the outlines of the areas off and the merged pairs' edges come with the reading,
//! and both frames in grey, the pictures it brings, laid on asking (the Grey view). A crop no pack
//! holds with areas: its picture laid over it, the eye's.

use std::sync::Arc;

use super::check::MaskCheck;
use super::marks::{MaskCompared, MaskRead};
use super::mask::Mask;
use crate::colour_areas::areas::Areas;
use crate::kinds::Kind;
use crate::marks::asked::{Asked, Compare, Pixels, Read, Reading, Reads};
use crate::marks::compared::Comparison;
use crate::marks::drawing::picture_laid;
use crate::maths::colour::{Rgb8, in_grey};
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// A gamut mask's copy read: its crop's areas, the size its picture fills in the frame, its mask
/// and its picture's corners (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct MaskReading {
    pub areas: Arc<Areas>,
    pub extent: Point,
    pub mask: Mask,
    pub corners: [Point; 4],
}

impl Reads for MaskReading {
    fn unread(&self) -> Comparison {
        Comparison::of(MaskCompared {
            colours: self.areas.colours.clone(),
            corners: self.corners,
            mask: self.mask,
            read: None,
        })
    }

    /// What is off and the marks' lines, with the number a first Compare keeps, and both frames in
    /// grey for the Grey view: the copy's (`GREY_COPY`) and the crop's where it was read
    /// (`GREY_ORIGINAL`).
    fn read(
        &self,
        size: [u32; 2],
        pixels: &[u8],
        paper: Rgb8,
        original: Option<Pixels<'_>>,
    ) -> Read {
        let areas = &self.areas;
        let colours = areas.read(size, pixels, paper).colours;
        let check = MaskCheck::of(areas, &colours, self.mask);
        let outlines = (0..areas.count())
            .filter(|area| check.is_outside(*area) || check.off_lightness.contains(area))
            .map(|area| (area, areas.edges_in_frame(area, None, self.extent)))
            .collect();
        let edges = (check.merged.iter())
            .flat_map(|(a, b)| areas.edges_in_frame(*a, Some(*b), self.extent))
            .collect();
        let metrics = check.metrics();
        let compared = MaskCompared {
            colours: areas.colours.clone(),
            corners: self.corners,
            mask: self.mask,
            read: Some(MaskRead {
                colours,
                check,
                outlines,
                edges,
            }),
        };
        let grey = (size, in_grey(pixels));
        let pictures = std::iter::once(grey)
            .chain(original.map(|(size, rgba)| (size, in_grey(rgba))))
            .collect();
        Read {
            comparison: Comparison::of(compared),
            metrics,
            pictures,
        }
    }

    fn greys(&self) -> bool {
        true
    }
}

/// A gamut mask's copy compared, once anything lies in its frame: read against its crop's areas,
/// or with none held, its picture laid over it.
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    if !asked.held() {
        return None;
    }
    let Kind::GamutMask(mask) = asked.exercise.kind else {
        return None;
    };
    let picture = asked.picture?;
    let Some((areas, extent)) = picture.areas_in_frame() else {
        return picture_laid(asked, Metrics::new());
    };
    Some(Compare::Read(Reading::of(MaskReading {
        areas: Arc::new(areas.clone()),
        extent,
        mask,
        corners: picture.fit.corners(),
    })))
}
