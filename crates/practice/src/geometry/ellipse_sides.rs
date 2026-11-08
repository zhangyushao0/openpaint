//! The sides of an ellipse's frame a drawn curve falls short of or runs past by more than a pen's
//! width (`REACH`), each named by the way it faces, its middle where the exact ellipse touches it
//! (S84, S97, S98): an ellipse's check and what is off on a solid's or an object's curves.

use crate::geometry::ellipse::EllipseGuide;
use crate::geometry::off::PEN_WIDTH;
use crate::maths::path::Point;

/// How far an ellipse may fall short of a side or run past it before it misses it: the Draw pen's
/// width, 5 pixels of a 400-pixel frame.
pub const REACH: f64 = PEN_WIDTH;

/// A side of the guide, named by the way it faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

/// Whether a drawn ellipse falls short of a side or runs past it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    Short,
    Past,
}

/// A side of the guide the drawn ellipse misses: which, which way, and its middle, where the exact
/// one touches it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Missed {
    pub side: Side,
    pub reach: Reach,
    pub middle: Point,
}

/// The sides of `guide` a curve falls short of or runs past by more than `REACH`, in order round:
/// how far it reaches along each side's outward normal is `reach(side, normal)` (none: the curve
/// says nothing of that side, S98's pieces too little round to fit).
pub(crate) fn missed(
    guide: &EllipseGuide,
    reach: impl Fn(usize, Point) -> Option<f64>,
) -> Vec<Missed> {
    let c = guide.corners;
    let middle = [0, 1].map(|k| c.iter().map(|p| p[k]).sum::<f64>() / 4.0);
    (0..4)
        .filter_map(|i| {
            let (a, b) = (c[i], c[(i + 1) % 4]);
            let along = [b[0] - a[0], b[1] - a[1]];
            let long = along[0].hypot(along[1]).max(f64::MIN_POSITIVE);
            let mut normal = [along[1] / long, -along[0] / long];
            let outward = (a[0] + b[0]) / 2.0 - middle[0];
            let outward = [outward, (a[1] + b[1]) / 2.0 - middle[1]];
            if normal[0] * outward[0] + normal[1] * outward[1] < 0.0 {
                normal = [-normal[0], -normal[1]];
            }
            let gap = normal[0] * a[0] + normal[1] * a[1] - reach(i, normal)?;
            let reach = if gap > REACH {
                Reach::Short
            } else if gap < -REACH {
                Reach::Past
            } else {
                return None;
            };
            let side = if normal[0].abs() >= normal[1].abs() {
                if normal[0] > 0.0 {
                    Side::Right
                } else {
                    Side::Left
                }
            } else if normal[1] > 0.0 {
                Side::Bottom
            } else {
                Side::Top
            };
            Some(Missed {
                side,
                reach,
                middle: guide.touches[i],
            })
        })
        .collect()
}
