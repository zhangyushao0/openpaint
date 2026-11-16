//! A curved object's sections (S154f, V大's weeks 7 and 8: a section at each joint, placed in the
//! box): the circle at each joint and at each flat end, inscribed in its square across the axis,
//! the square's sides along the box's faces; seen, the square's image and the circle's, exact
//! (`ellipse.rs`). A joint where the diameter steps holds two; a dome's foot on a cylinder as wide
//! is the one joint where the surface runs on smooth, no edge but still a section.
//!
//! V大's construction places each by lines in the box, the biggest first: the box cut across at the
//! joint's place along the axis (its four edges along the axis, at the joint's share of them), the
//! cut's diagonals crossing on the axis, and the section's square inside it, its corners on those
//! diagonals (a half or a third of the cut across, or the cut itself); a cone's point and a dome's
//! top where the diagonals of the box's end face cross.

use crate::geometry::ellipse::{EllipseGuide, square_on};
use crate::kinds::curved_rotations::body::Lying;
use crate::kinds::curved_rotations::design::{Design, Segment, SegmentKind};
use crate::kinds::objects::object_construction::Stage;
use crate::kinds::objects::object_design::Local;
use crate::maths::path::{Point, meet};

/// A section's circle in the body's terms: where it lies along the axis (a share of the length),
/// its diameter (a share of the width), and whether the surface turns there (an edge the eye
/// sees).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rim {
    pub along: f64,
    pub wide: f64,
    pub edge: bool,
}

/// A section seen: its rim and its square with the exact ellipse in it (frame units).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Section {
    pub rim: Rim,
    pub guide: EllipseGuide,
}

/// The corners of a square across the axis, `half` of the width from it either way, in order round.
const ROUND: [[f64; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];

/// The rims of `design`, the biggest first, then from the foot.
pub(crate) fn rims(design: &Design) -> Vec<Rim> {
    let segments = &design.segments;
    let mut rims: Vec<Rim> = Vec::new();
    let mut take = |along: f64, wide: f64, edge: bool| {
        if wide > 0.0 && !rims.iter().any(|r| r.along == along && r.wide == wide) {
            rims.push(Rim { along, wide, edge });
        }
    };
    let [first, last] = [&segments[0], &segments[segments.len() - 1]];
    if first.shape != SegmentKind::Dome {
        take(0.0, first.wide[0], true);
    }
    for pair in segments.windows(2) {
        let (below, above) = (&pair[0], &pair[1]);
        let along = below.along[1];
        let [under, over] = [below.wide[1], above.wide[0]];
        // A dome on an end as wide as it and as straight runs on smooth: no edge there.
        let same = (under - over).abs() <= 1e-9;
        let smooth = same && (smooth_onto(below, above) || smooth_onto(above, below));
        take(along, under, !smooth);
        take(along, over, !smooth);
    }
    if last.shape != SegmentKind::Dome {
        take(1.0, last.wide[1], true);
    }
    rims.sort_by(|a, b| b.wide.total_cmp(&a.wide).then(a.along.total_cmp(&b.along)));
    rims
}

/// Whether `dome` is a dome on the straight `end` (a cylinder): its surface runs on smooth.
fn smooth_onto(dome: &Segment, end: &Segment) -> bool {
    dome.shape == SegmentKind::Dome && end.shape == SegmentKind::Cylinder
}

/// The square `half` of the width either side of the axis at the share `along` of its length, in
/// the box's own coordinates as `lying` lays the body, its corners in order round.
fn square(along: f64, half: f64, lying: Lying) -> [Local; 4] {
    ROUND.map(|[x, y]| lying.local([along, x * half, y * half]))
}

/// `design`'s sections as `lying` lays it, each square's corners seen at `seen`.
pub(crate) fn sections(
    design: &Design,
    lying: Lying,
    seen: impl Fn(Local) -> Point,
) -> Vec<Section> {
    (rims(design).into_iter())
        .map(|rim| Section {
            rim,
            guide: square_on(square(rim.along, rim.wide / 2.0, lying).map(&seen)),
        })
        .collect()
}

/// V大's construction of `sections`, a stage each, then one for a cone's point or a dome's top at
/// either end: the box cut across, its diagonals, the square on them (seen at `seen`); what each
/// finds: where the diagonals cross on the axis, the square's middles.
pub(crate) fn construction(
    design: &Design,
    (lying, sections): (Lying, &[Section]),
    seen: impl Fn(Local) -> Point,
) -> Vec<Stage> {
    // The cut's sides (none at an end: the box's own face) and diagonals, where they cross.
    let cut = |along: f64| -> (Vec<[Point; 2]>, Point, Local) {
        let corners = square(along, 0.5, lying).map(&seen);
        let diagonals = [[corners[0], corners[2]], [corners[1], corners[3]]];
        let inside = along > 0.0 && along < 1.0;
        let mut lines: Vec<[Point; 2]> = (0..4)
            .filter(|_| inside)
            .map(|k| [corners[k], corners[(k + 1) % 4]])
            .collect();
        lines.extend(diagonals);
        let crossing = meet(diagonals[0], diagonals[1]).unwrap_or(corners[0]);
        (lines, crossing, lying.local([along, 0.0, 0.0]))
    };
    let mut stages: Vec<Stage> = (sections.iter())
        .map(|section| {
            let (mut lines, found, at) = cut(section.rim.along);
            if section.rim.wide < 1.0 {
                let corners = section.guide.corners;
                lines.extend((0..4).map(|k| [corners[k], corners[(k + 1) % 4]]));
            }
            Stage {
                lines,
                found: vec![found],
                at: vec![at],
                middles: section.guide.touches.to_vec(),
            }
        })
        .collect();
    let pointed = |segment: &Segment, end: usize| {
        segment.shape == SegmentKind::Dome
            || (segment.shape == SegmentKind::Cone && segment.wide[end] == 0.0)
    };
    let segments = &design.segments;
    let ends = [
        (0.0, pointed(&segments[0], 0)),
        (1.0, pointed(&segments[segments.len() - 1], 1)),
    ];
    for (along, _) in ends.into_iter().filter(|(_, pointed)| *pointed) {
        let (lines, found, at) = cut(along);
        stages.push(Stage {
            lines,
            found: vec![found],
            at: vec![at],
            middles: Vec::new(),
        });
    }
    stages
}
