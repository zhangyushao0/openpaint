//! Curved structure rotation's exactness (S154f): the scene rules hold seed after seed, every
//! segment, joint and attachment shows in the picture's view, and the answer lies on its
//! construction: each rim on its section's ellipse, each section's middles where V大's lines find
//! them, each cone's point and dome's top where the end face's diagonals cross, each outline
//! touching the ellipses at its ends, all within 1e-7.

use super::answer::{Owner, outlines};
use super::body::Part;
use super::curved::{CurvedRotation, CurvedTask, Handed};
use super::design::{Attached, Attachment, Design, SegmentKind, WIDTHS};
use super::view::CurvedView;
use crate::geometry::ellipse::Ellipse;
use crate::maths::path::{Point, distance};
use crate::maths::random::derived;

/// Tasks of each variant the tests lay.
const EACH: u64 = 16;
/// Within this (frame units) a point lies on its construction.
const EXACT: f64 = 1e-7;

fn tasks() -> impl Iterator<Item = CurvedTask> {
    CurvedRotation::ALL.into_iter().flat_map(|curved| {
        (0..EACH).map(move |k| CurvedTask::generate(curved, derived(0x0154_7E57, k)))
    })
}

/// Whether every corner of `view`'s box lies below its horizon: seen from above.
fn from_above(view: &CurvedView) -> bool {
    view.seen.corners.iter().all(|c| c[1] > view.seen.horizon)
}

#[test]
fn the_scene_rules_hold_seed_after_seed() {
    for task in tasks() {
        let (curved, design) = (task.curved, &task.design);
        let named = format!("{curved:?} {design:?}");
        let [least, most] = curved.segments.map(usize::from);
        assert!((least..=most).contains(&design.segments.len()), "{named}");
        let along = |s: f64| [0.0, 0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 1.0].contains(&s);
        for segment in &design.segments {
            assert!(segment.along.iter().all(|s| along(*s)), "{named}");
            assert!(
                segment.along[1] - segment.along[0] >= 1.0 / 6.0 - 1e-9,
                "{named}"
            );
            let pointed = segment.shape == SegmentKind::Cone;
            let wide = |w: &f64| WIDTHS.contains(w) || (pointed && *w == 0.0);
            assert!(segment.wide.iter().all(wide), "{named}");
        }
        assert!(
            design.segments.iter().any(|s| s.wide.contains(&1.0)),
            "{named}: tight"
        );
        let attached = match design.attachment {
            None => Attached::Nothing,
            Some(Attachment::Spout { .. }) => Attached::Spout,
            Some(Attachment::Handle { .. }) => Attached::Handle,
        };
        assert_eq!(attached, curved.attached, "{named}");
        for view in [&task.shown, &task.asked] {
            assert!(view.seen.keeps(false), "{named}: the box's rules");
            assert!(
                view.sections
                    .iter()
                    .all(|s| s.guide.exact.roundness() >= 0.2),
                "{named}"
            );
            let inside = |v: f64| (0.02..=0.98).contains(&v);
            assert!(
                view.extent.iter().all(|p| inside(p[0]) && inside(p[1])),
                "{named}"
            );
        }
        // Laid another way: standing where the picture lies, or along the box's other side.
        assert_ne!(task.shown.lying.axis, task.asked.lying.axis, "{named}");
        assert!(
            from_above(&task.shown),
            "{named}: the picture seen from above"
        );
        if curved.handed == Handed::Ticked {
            assert!(!task.shown.standing() && task.asked.standing(), "{named}");
            assert!(from_above(&task.asked), "{named}");
        }
        assert_eq!(
            task.angle.is_some(),
            curved.handed == Handed::Bare,
            "{named}"
        );
    }
}

#[test]
fn every_segment_joint_and_attachment_shows_in_the_picture_s_view() {
    for task in tasks() {
        let (shown, design) = (&task.shown.answer, &task.design);
        let named = format!("{:?} {design:?}", task.curved);
        for k in 0..design.segments.len() {
            assert!(
                shown.shown(Owner::Segment(k)) >= 0.05,
                "{named}: segment {k}"
            );
        }
        for (k, section) in task.shown.sections.iter().enumerate() {
            if section.rim.edge {
                assert!(shown.shown(Owner::Section(k)) >= 0.05, "{named}: rim {k}");
            }
        }
        if design.attachment.is_some() {
            assert!(shown.shown(Owner::Attachment) >= 0.2, "{named}");
            assert!(
                task.asked.answer.shown(Owner::Attachment) >= 0.05,
                "{named}"
            );
        }
    }
}

/// How far `p` lies off the line through `a` and `b`.
fn off_line(p: Point, [a, b]: [Point; 2]) -> f64 {
    let long = distance(a, b).max(f64::MIN_POSITIVE);
    ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs() / long
}

/// Whether the line heading `along` touches `ellipse` at its point `p`: square to its normal.
fn touches(ellipse: &Ellipse, p: Point, along: Point) -> f64 {
    let (sin, cos) = ellipse.angle.sin_cos();
    let [dx, dy] = [p[0] - ellipse.centre[0], p[1] - ellipse.centre[1]];
    let (x, y) = (dx * cos + dy * sin, -dx * sin + dy * cos);
    let [a, b] = ellipse.axes;
    let (gx, gy) = (x / (a * a), y / (b * b));
    let normal = [gx * cos - gy * sin, gx * sin + gy * cos];
    let long = normal[0].hypot(normal[1]) * along[0].hypot(along[1]);
    (normal[0] * along[0] + normal[1] * along[1]).abs() / long.max(f64::MIN_POSITIVE)
}

#[test]
fn the_answer_lies_on_its_construction_within_1e_7() {
    let (mut rims, mut outlined, mut worst) = (0, 0, 0.0_f64);
    for task in tasks() {
        let design = &task.design;
        for view in [&task.shown, &task.asked] {
            let named = format!("{:?} {design:?}", task.curved);
            // Each rim on its section's ellipse.
            for curve in &view.answer.curves {
                let Owner::Section(k) = curve.owner else {
                    continue;
                };
                let exact = view.sections[k].guide.exact;
                for p in curve.stretches.iter().flat_map(|s| &s.points) {
                    let off = exact.distance(*p);
                    worst = worst.max(off);
                    assert!(off < EXACT, "{named}: rim {k} off by {off}");
                }
                rims += 1;
            }
            // Each section's square on the diagonals of the box cut across at its place, drawn;
            // where they cross, the axis; then a cone's point or a dome's top on its end face's.
            for (k, section) in view.sections.iter().enumerate() {
                let along = section.rim.along;
                let cut = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]
                    .map(|[x, y]| view.projected(view.lying.local([along, x, y])));
                let diagonals = [[cut[0], cut[2]], [cut[1], cut[3]]];
                let lines = &view.stages[k].lines;
                assert!(diagonals.iter().all(|d| lines.contains(d)), "{named}");
                for corner in section.guide.corners {
                    let off = diagonals
                        .iter()
                        .map(|d| off_line(corner, *d))
                        .fold(1.0, f64::min);
                    assert!(off < EXACT, "{named}: section {k}'s corner off by {off}");
                }
                assert_eq!(view.stages[k].middles, section.guide.touches, "{named}");
            }
            for stage in &view.stages {
                for (found, at) in stage.found.iter().zip(&stage.at) {
                    assert!(distance(*found, view.projected(*at)) < EXACT, "{named}");
                }
            }
            assert!(view.stages.len() >= view.sections.len(), "{named}");
            outlined += touching(view, design, &named);
        }
    }
    println!("{rims} rims, {outlined} outline ends, at most {worst:.1e} off");
    assert!(rims > 0 && outlined > 0);
}

/// That each frustum's outlines end on the ellipses of the sections at its ends, touching them;
/// how many ends.
fn touching(view: &CurvedView, design: &Design, named: &str) -> usize {
    let mut count = 0;
    let segments = view.parts.segments.iter().enumerate();
    let rims: Vec<(f64, f64)> = view
        .sections
        .iter()
        .map(|s| (s.rim.along, s.rim.wide))
        .collect();
    for (k, part) in segments {
        let Part::Frustum(frustum) = part else {
            continue;
        };
        let segment = &design.segments[k];
        for [p0, p1] in outlines(frustum) {
            let [a, b] = [p0, p1].map(|p| view.view.at(p));
            let along = [b[0] - a[0], b[1] - a[1]];
            for (end, p) in [(0, a), (1, b)] {
                let rim = (segment.along[end], segment.wide[end]);
                let Some(j) = rims.iter().position(|r| *r == rim) else {
                    continue;
                };
                let exact = view.sections[j].guide.exact;
                assert!(exact.distance(p) < EXACT, "{named}: an outline's end");
                assert!(
                    touches(&exact, p, along) < 1e-6,
                    "{named}: an outline touches"
                );
                count += 1;
            }
        }
    }
    count
}

#[test]
fn a_view_s_picture_is_its_parts_lit_on_paper() {
    let task = CurvedTask::generate(CurvedRotation::ALL[2], 5);
    let picture = task.shown.picture(200);
    assert_eq!(picture.len(), 200 * 200 * 4);
    let covered = picture.chunks(4).filter(|px| px[3] == 255).count();
    assert!(covered > 2_000, "the body fills its place: {covered}");
    // Where the eye sees a part the picture shows it, the paper elsewhere.
    let middle = task.shown.projected([0.5; 3]);
    assert!(task.shown.surface_at(middle).is_some());
    assert!(task.shown.surface_at([0.01, 0.01]).is_none());
}
