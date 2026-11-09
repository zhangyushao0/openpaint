//! An ellipse compared, and Compare's words for it (S84): the ellipse fitted to the hand's lines
//! (`geometry/ellipse_fit.rs`) against the exact one in what the eye sees: rounder or flatter (its
//! degree, `DEGREE`), tilted which way (its major axis, `TILT`, only where both are clearly not
//! round), and which side of the guide it falls short of or runs past by more than a pen's width
//! (`geometry/ellipse_sides.rs`), the side's middle where the exact one touches it ringed (S97: the
//! fitted one is not drawn; the copy's own line beside the red answer shows its shape); and its
//! verdict once the lines go round far enough to judge (S119).

use crate::geometry::ellipse::{Ellipse, EllipseGuide, upright};
use crate::geometry::ellipse_fit::fit_ellipse;
use crate::geometry::ellipse_sides::{Missed, missed};
use crate::kinds::solids::solid_check::covered;
use crate::marks::compared::{Outcome, Verdict};
use crate::maths::path::Point;

/// Degrees of difference in degree (Drawabox's: 90 a circle) before one is rounder or flatter.
pub const DEGREE: f64 = 4.0;
/// Degrees the major axes may differ before one is tilted; only while both are flatter than
/// `ROUND` (a near circle's axis says nothing).
pub const TILT: f64 = 4.0;
const ROUND: f64 = 0.9;
/// A near ellipse has at most this many findings (rounder or flatter, tilted, a side missed).
pub const NEAR_FINDINGS: usize = 1;

/// How a drawn ellipse's roundness compares with the exact one's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Roundness {
    Rounder,
    Flatter,
}

/// Which way a drawn ellipse's major axis is turned from the exact one's, as the screen shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tilt {
    Clockwise,
    Anticlockwise,
}

/// An ellipse compared: the one fitted to the copy's lines (none found: nothing more), how it
/// differs from the exact one, and whether the lines go round it far enough to judge
/// (`COVERED`, as a solid's ends).
#[derive(Debug, Clone, PartialEq)]
pub struct EllipseCheck {
    pub fitted: Option<Ellipse>,
    pub roundness: Option<Roundness>,
    pub tilt: Option<Tilt>,
    pub sides: Vec<Missed>,
    pub round: bool,
}

impl EllipseCheck {
    /// The ellipse in `lines` (frame units) against `guide`'s exact one.
    #[must_use]
    pub fn of(guide: &EllipseGuide, lines: &[&[Point]]) -> Self {
        let pieces: Vec<Vec<Point>> = lines.iter().map(|line| line.to_vec()).collect();
        let round = covered(&guide.exact, &pieces);
        let Some(fitted) = fit_ellipse(lines) else {
            return Self {
                fitted: None,
                roundness: None,
                tilt: None,
                sides: Vec::new(),
                round,
            };
        };
        let exact = guide.exact;
        let degrees = fitted.degree() - exact.degree();
        let roundness = (degrees.abs() >= DEGREE).then_some(if degrees > 0.0 {
            Roundness::Rounder
        } else {
            Roundness::Flatter
        });
        let flat = fitted.roundness() < ROUND && exact.roundness() < ROUND;
        let turned = upright(fitted.angle - exact.angle).to_degrees();
        let tilt = (flat && turned.abs() >= TILT).then_some(if turned > 0.0 {
            Tilt::Clockwise
        } else {
            Tilt::Anticlockwise
        });
        Self {
            fitted: Some(fitted),
            roundness,
            tilt,
            sides: missed(guide, |_, normal| Some(fitted.support(normal))),
            round,
        }
    }

    /// Incomplete while no ellipse can be fitted or the lines go less than `COVERED` round it (an
    /// arc fits an ellipse it barely shows); clean with no finding (roundness, tilt, a side short
    /// or past), near with `NEAR_FINDINGS`, else redo.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        if self.fitted.is_none() || !self.round {
            return Verdict::Incomplete;
        }
        let findings = usize::from(self.roundness.is_some())
            + usize::from(self.tilt.is_some())
            + self.sides.len();
        Verdict::App(Outcome::counting(findings, NEAR_FINDINGS))
    }
}
