//! A colour combination's copy compared (S82, S101): read from its frame's pixels on the compare
//! worker against the areas it was made with (a generated one's) or its crop holds, each area's
//! colour and the weak contrasts made strong; a crop no pack holds with areas, its picture laid
//! over it instead.

use std::sync::Arc;

use super::marks::{AreasCompared, AreasRead};
use crate::colour_areas::areas::Areas;
use crate::marks::asked::{Asked, Compare, Pixels, Read, Reading, Reads};
use crate::marks::compared::Comparison;
use crate::marks::drawing::picture_laid;
use crate::maths::colour::Rgb8;
use crate::maths::path::Point;
use crate::records::metric::Metrics;

/// A combination's copy read against its areas, the picture filling `extent` of its frame.
#[derive(Debug, Clone, PartialEq)]
pub struct AreasReading {
    pub areas: Arc<Areas>,
    pub extent: Point,
}

impl Reads for AreasReading {
    fn unread(&self) -> Comparison {
        Comparison::of(AreasCompared {
            colours: self.areas.colours.clone(),
            read: None,
        })
    }

    fn read(&self, size: [u32; 2], pixels: &[u8], paper: Rgb8, _: Option<Pixels<'_>>) -> Read {
        let areas = &self.areas;
        let copy = areas.read(size, pixels, paper);
        let strong = (copy.contrasts.iter())
            .filter(|c| c.made_strong())
            .flat_map(|c| areas.edges_in_frame(c.areas.0, Some(c.areas.1), self.extent))
            .collect();
        let metrics: Metrics = copy.metrics(areas);
        let compared = AreasCompared {
            colours: areas.colours.clone(),
            read: Some(AreasRead { copy, strong }),
        };
        Read {
            comparison: Comparison::of(compared),
            metrics,
            pictures: Vec::new(),
        }
    }
}

/// `asked`'s copy compared, once anything lies in its frame: read against its areas, or with none
/// held, its picture laid over it.
pub(crate) fn compare(asked: &Asked<'_>) -> Option<Compare> {
    if !asked.held() {
        return None;
    }
    let areas = (asked.original.areas()).or_else(|| asked.picture?.areas_in_frame());
    match areas {
        Some((areas, extent)) => Some(Compare::Read(Reading::of(AreasReading {
            areas: Arc::new(areas.clone()),
            extent,
        }))),
        None => picture_laid(asked, Metrics::new()),
    }
}
