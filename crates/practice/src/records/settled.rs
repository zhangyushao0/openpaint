//! The outcome a first Compare's numbers settle (S120c), for the records written before S119,
//! which kept numbers and no result: the one-time conversion of the session records
//! (practice-store `convert.rs`) asks it of each compared exercise and goes with it (S121).
//!
//! It is the kind's own verdict (`Compared::verdict`) read from the numbers kept, where they hold
//! all it reads: a colour's last difference, a combination's mean difference and the weak
//! contrasts it made strong, a value scale's uneven and reversed cells, a point's or a
//! proportion's distance from the answer of its original made again. Any other kind, or numbers
//! missing, settle none.

use crate::geometry::off::PEN_WIDTH;
use crate::kinds::Original;
use crate::kinds::colours::judgement::{DELTA_SPAN, NOTICEABLE, PART_SPAN};
use crate::kinds::combinations::marks::{AREAS_CLEAN, NEAR_STRONG};
use crate::kinds::estimates::flat::{Flat, NEAR_OFF};
use crate::kinds::scales::scale::{FIRST_CELLS, NEAR_UNEVEN, NEAR_UNEVEN_LONG};
use crate::marks::compared::Outcome;
use crate::maths::path::distance;
use crate::records::metric::{Metric, Metrics, value};
use crate::sessions::plan::{Exercise, Family};

/// The outcome `exercise`'s first Compare kept as `metrics` settles; none where they do not.
#[must_use]
pub fn settled(exercise: &Exercise, metrics: &Metrics) -> Option<Outcome> {
    let of = |metric| value(metrics, metric);
    match exercise.kind.family() {
        Family::Colour => Some(Outcome::within(
            of(Metric::FinalDelta)?,
            [NOTICEABLE, PART_SPAN],
        )),
        Family::Areas => {
            let strong = of(Metric::AreasMadeStrong)? as usize;
            let off = Outcome::within(of(Metric::AreasDelta)?, [AREAS_CLEAN, DELTA_SPAN]);
            Some(match (off, strong) {
                (Outcome::Clean, 0) => Outcome::Clean,
                (Outcome::Clean | Outcome::Near, ..=NEAR_STRONG) => Outcome::Near,
                _ => Outcome::Redo,
            })
        }
        Family::Scale => {
            let cells = exercise.original().cells()?;
            let near = if cells <= FIRST_CELLS {
                NEAR_UNEVEN
            } else {
                NEAR_UNEVEN_LONG
            };
            let (uneven, reversed) = (of(Metric::ScaleUneven)?, of(Metric::ScaleReversed)?);
            let (uneven, reversed) = (uneven as usize, reversed as usize);
            Some(match (uneven, reversed) {
                (0, 0) => Outcome::Clean,
                (uneven, 0) if uneven <= near => Outcome::Near,
                _ => Outcome::Redo,
            })
        }
        Family::Point | Family::Proportion => estimated(exercise, metrics),
        _ => None,
    }
}

/// A point's or a proportion's outcome: clean within a pen's width of the answer, near within
/// `NEAR_OFF`, else redo (`FlatCheck::verdict`).
fn estimated(exercise: &Exercise, metrics: &Metrics) -> Option<Outcome> {
    let Original::Flat(flat) = exercise.original() else {
        return None;
    };
    let off = match flat {
        Flat::Point { .. } => {
            let across = value(metrics, Metric::PointAcross)?;
            let down = value(metrics, Metric::PointDown)?;
            distance([across, down], [0.0, 0.0])
        }
        Flat::Proportion { copy, .. } => {
            value(metrics, Metric::ProportionOff)?.abs() * distance(copy[0], copy[1])
        }
        Flat::Angle { .. } => return None,
    };
    Some(if off <= PEN_WIDTH {
        Outcome::Clean
    } else if off <= NEAR_OFF {
        Outcome::Near
    } else {
        Outcome::Redo
    })
}
