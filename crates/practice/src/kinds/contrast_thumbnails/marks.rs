//! A contrast thumbnail's print and Compare (S154). Printed: the composition's mass edges faint
//! in the one frame and the ring or rings dashed in the strong and second pens' colours. Compared:
//! the thumbnail read from its own pixels on the compare worker (`Steps`), its veil laid over it
//! (clear where it is strongest), the rings over the veil, and the rules read directly from the
//! steps, each a red dot's line on the card and, where it is a place, ringed there: the strongest
//! step outside the rings (`Outside`), a place outside within `WITHIN` of the ring's (`Rival`); with
//! two rings, the second not clearly weaker than the first (`SecondStrong`) or not clearly stronger
//! than the rest (`SecondWeak`). The rest lies past each ring's own band (`SPILL`). They are help:
//! the eye judges (D123), no number is kept.

use super::rings::Ring;
use super::steps::Steps;
use crate::compositions::levels::Level;
use crate::compositions::value_masses::BLUR;
use crate::kinds::abstract_analysis::contrast::{CONTRAST_REACH, LIGHTNESS_CONTRAST};
use crate::kinds::value_masses::value_masses_read::BARE_LEFT;
use crate::marks::asked::{Pixels, Read, Reads};
use crate::marks::card::{CardItem, Say};
use crate::marks::compared::{Compared, Comparison, Seen, Verdict};
use crate::marks::mark::{Clip, Figure, In, LinePen, Place, SheetMark, Source};
use crate::maths::colour::Rgb8;
use crate::maths::path::{Point, distance};
use crate::records::metric::Metrics;
use crate::sheets::sheet_picture::MassesShown;

/// Half a value step of the three greys (Oklab lightness): a place this near another's step reads
/// as strong as it, and a step past another by less than `TIE` is no stronger.
pub const WITHIN: f64 = 0.125;
const TIE: f64 = 0.02;
/// How far past a ring its own contrast reads (frame units): a step made at its edge shows as a
/// band `CONTRAST_REACH` wide either side of it, squinted a little wider. The rest begins beyond.
const SPILL: f64 = CONTRAST_REACH + 2.0 * BLUR;

/// What the rules find, each alone a line on the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Finding {
    /// The strongest step lies outside the rings.
    Outside,
    /// A place outside the rings is about as strong as the (first) ring.
    Rival,
    /// The second ring is about as strong as the first, or stronger.
    SecondStrong,
    /// A place outside the rings is about as strong as the second ring, or stronger.
    SecondWeak,
}

impl Finding {
    /// Its words' key on the card.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Outside => "focus-outside",
            Self::Rival => "focus-rival",
            Self::SecondStrong => "second-strong",
            Self::SecondWeak => "second-weak",
        }
    }
}

/// The print in the one frame: the composition's mass edges faint, then the rings.
pub(crate) fn printed(masses: &MassesShown, rings: &[Ring]) -> Vec<SheetMark> {
    let edges = (masses.edges.iter()).map(|edge| {
        SheetMark::free_line(In::Copy, edge.clone(), LinePen::Faint).clipped(Clip::Inside)
    });
    edges.chain(ringed(rings)).collect()
}

/// The rings, dashed in the strong pen's red, the second in its green.
fn ringed(rings: &[Ring]) -> Vec<SheetMark> {
    (rings.iter())
        .map(|ring| {
            let level = if ring.second {
                Level::Second
            } else {
                Level::Main
            };
            SheetMark::dashed(In::Copy, ring.line(), LinePen::Level(level))
        })
        .collect()
}

/// A thumbnail to be read: its rings and the frame its composition fills (frame units).
#[derive(Debug, Clone, PartialEq)]
pub struct ThumbnailReading {
    pub rings: Vec<Ring>,
    pub extent: Point,
}

impl Reads for ThumbnailReading {
    fn unread(&self) -> Comparison {
        Comparison::of(ThumbnailCompared {
            rings: self.rings.clone(),
            read: None,
        })
    }

    /// Its steps from the frame's pixels, the rules over them and its veil, the one picture it
    /// brings (`Source::Read(0)`); no number kept.
    fn read(&self, size: [u32; 2], pixels: &[u8], _: Rgb8, _: Option<Pixels<'_>>) -> Read {
        let steps = Steps::of(size, pixels);
        let read = ThumbnailRead::of(&steps, &self.rings, self.extent);
        Read {
            comparison: Comparison::of(ThumbnailCompared {
                rings: self.rings.clone(),
                read: Some(read),
            }),
            metrics: Metrics::new(),
            pictures: vec![(steps.size, steps.veil())],
        }
    }
}

/// A thumbnail as read: what the rules found, the places they ring (frame units) and the share
/// left bare over more than the pen's width.
#[derive(Debug, Clone, PartialEq)]
pub struct ThumbnailRead {
    pub findings: Vec<Finding>,
    pub places: Vec<Point>,
    pub bare: f64,
}

impl ThumbnailRead {
    /// The rules on `steps` for `rings` in a frame of `extent`.
    #[must_use]
    pub fn of(steps: &Steps, rings: &[Ring], extent: Point) -> Self {
        let at = |[u, v]: Point| [u * extent[0], v * extent[1]];
        let inside = |ring: Option<&Ring>| {
            ring.and_then(|ring| steps.strongest(|p| ring.holds(at(p))))
                .map(|(step, p)| (step, at(p)))
        };
        let first = inside(rings.iter().find(|ring| !ring.second));
        let second = inside(rings.iter().find(|ring| ring.second));
        let spilt = |p: Point| {
            (rings.iter()).any(|ring| distance(ring.centre, at(p)) <= ring.radius + SPILL)
        };
        // Only a contrast at all is a rival: a flat stretch is as weak as a flat ring, not as strong.
        let rest = (steps.strongest(|p| !spilt(p)))
            .filter(|(step, _)| *step >= LIGHTNESS_CONTRAST.weak)
            .map(|(step, p)| (step, at(p)));
        let (mut findings, mut places) = (Vec::new(), Vec::new());
        let step = |found: Option<(f64, Point)>| found.map_or(0.0, |(step, _)| step);
        if let Some((outside, place)) = rest {
            let ring = step(first);
            if outside > ring + TIE {
                findings.push(Finding::Outside);
                places.push(place);
            } else if outside >= ring - WITHIN {
                findings.push(Finding::Rival);
                places.push(place);
            }
            if second.is_some() && outside >= step(second) - WITHIN {
                findings.push(Finding::SecondWeak);
                if !places.contains(&place) {
                    places.push(place);
                }
            }
        }
        let made = step(second) >= LIGHTNESS_CONTRAST.weak;
        if made && step(second) >= step(first) - WITHIN {
            findings.push(Finding::SecondStrong);
        }
        Self {
            findings,
            places,
            bare: steps.bare_share(),
        }
    }
}

/// A thumbnail compared: its rings and, once read, what was found.
#[derive(Debug, Clone, PartialEq)]
pub struct ThumbnailCompared {
    pub rings: Vec<Ring>,
    pub read: Option<ThumbnailRead>,
}

impl Compared for ThumbnailCompared {
    /// Once read, its veil over the frame, the rings over it, the places the rules ring.
    fn marks(&self, _seen: &Seen<'_>) -> Vec<SheetMark> {
        let Some(read) = &self.read else {
            return Vec::new();
        };
        let veil = Figure::Picture {
            source: Source::Read(0),
            place: Place::Frame,
            shown: 1.0,
        };
        let mut marks = vec![SheetMark::free(In::Copy, veil).clipped(Clip::Laid)];
        marks.extend(ringed(&self.rings));
        marks.extend((read.places.iter()).map(|p| SheetMark::cut(In::Copy, Figure::Ring(*p))));
        marks
    }

    /// A line after a red dot for each rule that found something; that part of the frame is still
    /// bare while it is.
    fn card(&self, _seen: &Seen<'_>) -> Vec<CardItem> {
        let Some(read) = &self.read else {
            return vec![CardItem::Reading];
        };
        if read.bare > BARE_LEFT {
            return vec![CardItem::Says(Say::plain("thumbnail-bare"))];
        }
        (read.findings.iter())
            .map(|finding| CardItem::Failed(Say::plain(finding.key())))
            .collect()
    }

    /// Once read and covered: the eye's, the rules its help (D123); before, none is honest.
    fn verdict(&self) -> Verdict {
        match &self.read {
            Some(read) if read.bare <= BARE_LEFT => Verdict::Eye,
            _ => Verdict::Incomplete,
        }
    }
}
