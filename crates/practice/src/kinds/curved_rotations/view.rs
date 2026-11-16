//! One view of a curved object (S154f): its box tight round the body seen two-point by an eye
//! looking level, as unit 4's objects' boxes are seen (turned 25 to 65 degrees, below or above the
//! eye's level, far enough that a standing body's sections are no slivers), the horizon near the
//! frame's edge and the box as large as the room beyond allows; the body lying in it as the view
//! lays it (`Lying`); its sections (each joint's and end's circle in its square, exact), placed in
//! the box by V大's lines (`sections.rs`), what stands on the side boxed, and the exact answer
//! (`answer.rs`).
//!
//! A view keeps its rules or is laid again: its box's (`PerspectiveBox::keeps`), every section no
//! sliver, every line inside the frame; the picture's view (`Rules::Pictured`) shows each
//! segment's outlines, each section the surface turns at and what stands on the side; the asked
//! one each segment (by its outlines or the rims at its ends) and what stands on the side.

use crate::geometry::ellipse::EllipseGuide;
use crate::geometry::perspective::PerspectiveBox;
use crate::geometry::perspective_eye::{box_corners, directions, fitted, framing, projected};
use crate::kinds::curved_rotations::answer::{
    CurvedAnswer, Line, Owner, dome_outline, outlines, seen,
};
use crate::kinds::curved_rotations::attached::{boxed, handle_lines};
use crate::kinds::curved_rotations::body::{Lying, Part, Parts};
use crate::kinds::curved_rotations::design::{Attachment, Design};
use crate::kinds::curved_rotations::sections::{Section, construction, rims, sections};
use crate::kinds::objects::object_body::Placed;
use crate::kinds::objects::object_construction::Stage;
use crate::kinds::solids::solid_answer::View;
use crate::maths::path::Point;
use crate::maths::random::Random;
use crate::maths::vector::{Vector, across, add, times, unit};

/// The box's longer side in the frame, smaller where something stands on the side (its room);
/// the body's width in the world.
const SIDE: [f64; 2] = [0.56, 0.66];
const ATTACHED_SIDE: [f64; 2] = [0.42, 0.5];
const WIDTH: [f64; 2] = [0.7, 1.0];
const DISTANCE: [f64; 2] = [2.4, 4.0];
/// How far the eye looks down on the box's top or up at its bottom (degrees, from its middle's
/// distance): a standing body's sections no slivers, a lying one's top or bottom face seen.
const STANDING_TILT: [f64; 2] = [12.0, 24.0];
const LYING_TILT: [f64; 2] = [8.0, 20.0];
/// The horizon's place from the frame's edge nearer it, and the room the box keeps from the other.
const HORIZON: [f64; 2] = [0.08, 0.16];
const ROOM: f64 = 0.1;
/// Every section at least this round (minor over major); every line this far inside the frame
/// (frame units); each segment's lines, each section's rim the surface turns at, what stands on
/// the side, showing this long at least (20 pixels of a 400-pixel frame).
const ROUND: f64 = 0.2;
const MARGIN: f64 = 0.02;
const SHOWN: f64 = 0.05;
/// What stands on the side showing this long at least in the picture's view: read, not glimpsed.
const ATTACHED_SHOWN: f64 = 0.2;

/// Which rules a view keeps: the picture's every one, the asked view's those its answer needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rules {
    Pictured,
    Drawn,
}

/// One view of a curved object.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvedView {
    /// The box tight round the body: its front corner, three edges and horizon.
    pub seen: PerspectiveBox,
    /// The axis from the centre of the box's face at the foot to the one at the top.
    pub axis: [Point; 2],
    /// Where each joint and end lies on the axis, from the foot.
    pub joints: Vec<Point>,
    /// Each section, the biggest first.
    pub sections: Vec<Section>,
    /// V大's construction of the sections, a stage each, then the points of a cone or a dome.
    pub stages: Vec<Stage>,
    /// What stands on the side boxed: its box's edges and a spout's sections.
    pub attached: (Vec<[Point; 2]>, Vec<EllipseGuide>),
    pub answer: CurvedAnswer,
    pub(crate) lying: Lying,
    pub(crate) parts: Parts,
    pub(crate) placed: Placed,
    pub(crate) view: View,
    /// Where the light comes from (one long, towards it).
    pub(crate) light: Vector,
    /// The least and greatest frame point anything of it reaches.
    pub(crate) extent: [Point; 2],
}

impl CurvedView {
    /// Where the box's own point `p` lies in the frame: the object in space projected.
    #[must_use]
    pub fn projected(&self, p: [f64; 3]) -> Point {
        self.view.at(self.placed.world(p))
    }

    /// Whether the body stands in its box (its axis up).
    #[must_use]
    pub const fn standing(&self) -> bool {
        self.lying.standing()
    }

    /// The view of `design` lying as `lying` lays it, seen from above (`above`) or below, the rest
    /// of the eye laid from `random`; none when it breaks a rule of `rules`.
    pub(crate) fn seen_by(
        design: &Design,
        laid: (Lying, bool),
        random: &mut Random,
        rules: Rules,
    ) -> Option<Self> {
        Self::laid(design, laid, random, Some(rules))
    }

    /// The view of `design` lying as `lying` lays it, seen from above (`above`) or below, the rest
    /// of the eye laid from `random`, whatever rule it breaks.
    pub(crate) fn built(design: &Design, laid: (Lying, bool), random: &mut Random) -> Self {
        let view = Self::laid(design, laid, random, None);
        view.unwrap_or_else(|| unreachable!("a view without rules is always laid"))
    }

    /// The view of `design` laid so, whatever rule it breaks without `rules`; else none when it
    /// breaks one (its box's and its sections' looked at before its answer is found).
    fn laid(
        design: &Design,
        (lying, above): (Lying, bool),
        random: &mut Random,
        rules: Option<Rules>,
    ) -> Option<Self> {
        let width = random.range(WIDTH[0], WIDTH[1]);
        let size = lying.size(width, width * design.proportion);
        let side = if design.attachment.is_some() {
            ATTACHED_SIDE
        } else {
            SIDE
        };
        let (seen, placed, view, lights) = looked(size, (lying, above), (side, random));
        let sections = sections(design, lying, |p| view.at(placed.world(p)));
        let round = (sections.iter()).all(|s| s.guide.exact.roundness() >= ROUND);
        if rules.is_some() && !(seen.keeps(false) && round) {
            return None;
        }
        let parts = Parts::of(design, lying, placed);
        let stages = construction(design, (lying, &sections), |p| view.at(placed.world(p)));
        let answer = seen_answer(design, &parts, view);
        let at = |p: [f64; 3]| view.at(parts.frame.at(p));
        let joints = std::iter::once(0.0)
            .chain(design.segments.iter().map(|s| s.along[1]))
            .map(|s| at([s, 0.0, 0.0]))
            .collect();
        let attached = boxed(design, at);
        let points: Vec<Point> = (seen.corners.iter().copied())
            .chain(answer.points())
            .collect();
        let low = [0, 1].map(|k| points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
        let high = [0, 1].map(|k| {
            points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let from_left = random.unit() < 0.5;
        let view = Self {
            seen,
            axis: [at([0.0; 3]), at([1.0, 0.0, 0.0])],
            joints,
            sections,
            stages,
            attached,
            answer,
            lying,
            parts,
            placed,
            view,
            light: lights[usize::from(from_left)],
            extent: [low, high],
        };
        (rules.is_none_or(|rules| view.keeps(design, rules))).then_some(view)
    }

    /// Whether it keeps every rule of `rules` but its box's and its sections'.
    fn keeps(&self, design: &Design, rules: Rules) -> bool {
        let inside = |v: f64| (MARGIN..=1.0 - MARGIN).contains(&v);
        let framed = self.extent.iter().all(|p| inside(p[0]) && inside(p[1]));
        let answer = &self.answer;
        let attached =
            |least: f64| design.attachment.is_none() || answer.shown(Owner::Attachment) >= least;
        let rim = |k: usize| answer.shown(Owner::Section(k));
        let shown = match rules {
            // Each segment's outlines, each rim the surface turns at, what stands on the side well.
            Rules::Pictured => {
                let segments =
                    (0..design.segments.len()).all(|k| answer.shown(Owner::Segment(k)) >= SHOWN);
                let rims = (self.sections.iter().enumerate())
                    .filter(|(_, s)| s.rim.edge)
                    .all(|(k, _)| rim(k) >= SHOWN);
                segments && rims && attached(ATTACHED_SHOWN)
            }
            // Each segment by its outlines or the rims at its ends.
            Rules::Drawn => {
                (design.segments.iter().enumerate()).all(|(k, segment)| {
                    let at_end = |s: &Section| {
                        segment.along.contains(&s.rim.along) && segment.wide.contains(&s.rim.wide)
                    };
                    let ends = (self.sections.iter().enumerate())
                        .filter(|(_, s)| at_end(s))
                        .map(|(j, _)| rim(j))
                        .sum::<f64>();
                    answer.shown(Owner::Segment(k)) + ends >= SHOWN
                }) && attached(SHOWN)
            }
        };
        framed && shown
    }
}

/// The box `size` seen by the eye looking level from above (`above`) or below, tilted as `lying`
/// asks, the rest laid from `random`: the box fitted in the frame with its longer side in `side`,
/// where it lies in the world, the view, the lights it may be lit by (from the left or the right,
/// above).
fn looked(
    size: [f64; 3],
    (lying, below): (Lying, bool),
    (side, random): ([f64; 2], &mut Random),
) -> (PerspectiveBox, Placed, View, [Vector; 2]) {
    let yaw = random.range(25.0, 65.0).to_radians();
    let depth = random.range(DISTANCE[0], DISTANCE[1]);
    let tilt = if lying.standing() {
        STANDING_TILT
    } else {
        LYING_TILT
    };
    let over = size[1] / 2.0 + depth * random.range(tilt[0], tilt[1]).to_radians().tan();
    let middle = [
        random.range(-0.2, 0.2) * depth,
        if below { -over } else { over },
        depth,
    ];
    let world = box_corners(middle, yaw, size);
    let image = world.map(projected);
    let [left, right, up] = directions(yaw);
    // The horizon near the frame's edge on its side, the box as large as the room beyond allows.
    let low = [0, 1].map(|k| image.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let high = [0, 1].map(|k| image.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max));
    let extent = (high[0] - low[0]).max(high[1] - low[1]);
    let horizon = random.range(HORIZON[0], HORIZON[1]);
    let reach = if below { high[1] } else { -low[1] };
    let room = (1.0 - horizon - ROOM) * extent / reach.max(f64::MIN_POSITIVE);
    let long = random.range(side[0], side[1]).min(room);
    let scale = long / extent;
    let horizon = if below { horizon } else { 1.0 - horizon };
    let shift = [
        random.range(-0.05, 0.05),
        horizon - 0.5 + (low[1] + high[1]) / 2.0 * scale,
    ];
    let (scale, offset) = framing(&image, 0.0, (long, shift), None);
    let vanishing = [Some(projected(left)), Some(projected(right)), None];
    let seen = fitted(image, (0.0, vanishing), (long, shift), None);
    let placed = Placed {
        origin: world[0],
        axes: [
            times(right, size[0]),
            times(up, size[1]),
            times(left, size[2]),
        ],
    };
    let light = |from_left: bool| {
        let [lit, dim] = if from_left { [0.9, 0.35] } else { [0.35, 0.9] };
        unit(add(
            times(up, 1.6),
            add(times(right, -lit), times(left, -dim)),
        ))
    };
    (
        seen,
        placed,
        View { scale, offset },
        [light(false), light(true)],
    )
}

/// The answer of `design` built as `parts`, through `view`: each segment's outlines, each section
/// the surface turns at, what stands on the side.
fn seen_answer(design: &Design, parts: &Parts, view: View) -> CurvedAnswer {
    let mut lines: Vec<(Line, Owner)> = Vec::new();
    for (k, part) in parts.segments.iter().enumerate() {
        match part {
            Part::Frustum(frustum) => {
                let sides = outlines(frustum).into_iter().map(Line::Straight);
                lines.extend(sides.map(|line| (line, Owner::Segment(k))));
            }
            Part::Dome(dome) => lines.push((dome_outline(dome), Owner::Segment(k))),
        }
    }
    let frame = parts.frame;
    for (k, rim) in rims(design).iter().enumerate().filter(|(_, rim)| rim.edge) {
        let radius = rim.wide / 2.0 * frame.width;
        let circle = Line::Circle {
            centre: frame.at([rim.along, 0.0, 0.0]),
            u: times(frame.side, radius),
            v: times(frame.across, radius),
            dome: None,
        };
        lines.push((circle, Owner::Section(k)));
    }
    let attached = attached_lines(design, parts).into_iter();
    lines.extend(attached.map(|line| (line, Owner::Attachment)));
    seen(parts, view, &lines)
}

/// The lines of what stands on the side: a spout's outlines, its tip and where it meets the body;
/// a handle's edges and where its bars meet the body.
fn attached_lines(design: &Design, parts: &Parts) -> Vec<Line> {
    match (design.attachment, parts.spout) {
        (Some(Attachment::Spout { .. }), Some(spout)) => {
            let mut lines: Vec<Line> = outlines(&spout).into_iter().map(Line::Straight).collect();
            let [u, v] = across(spout.axis);
            lines.push(Line::Circle {
                centre: add(spout.centre, times(spout.axis, spout.length)),
                u: times(u, spout.radii[1]),
                v: times(v, spout.radii[1]),
                dome: None,
            });
            lines.push(Line::Spout(spout));
            lines
        }
        (Some(Attachment::Handle { along, reach }), _) => handle_lines(&parts.frame, along, reach),
        _ => Vec::new(),
    }
}
