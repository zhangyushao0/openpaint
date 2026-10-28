//! Week 2's perspective demonstrations (`demo.rs`): a box to complete (the given edges run on to
//! their vanishing points on the horizon, lines back from the points to the far corners, the nine
//! edges inked), free boxes (two points on the horizon, a front upright, its edges towards the
//! points, the rest) and an ellipse (the diagonals, the four middles through the centre, passes in
//! the air, one stroke), each then compared as the exercise compares it.

use super::{Demo, Drawn, Layout, Marker, On, Script, WIDE_REACH, clearest, line, wobbled};
use crate::ellipse::{Ellipse, EllipseGuide, Guide};
use crate::path::{Point, distance, meet};
use crate::perspective::{BoxView, Direction, EDGES, Horizon, PerspectiveBox};
use crate::plan::Family;

const BOX_SEED: u64 = 0xDE30_5EED_0007;
const FREE_SEED: u64 = 0xDE30_5EED_0008;
const ELLIPSE_SEED: u64 = 0xDE30_5EED_0009;
/// Seeds of the sequence tried for a box whose vanishing points the card has room for.
const TRIES: u64 = 96;
/// An ellipse's outline in pieces.
const PIECES: usize = 96;

/// Of the boxes `make` lays from `seed`'s sequence whose two vanishing points lie within
/// `WIDE_REACH` of the frame's sides, the one widest beside the distance between its points, so
/// it reads largest on the card.
fn within_reach(seed: u64, make: impl Fn(u64) -> PerspectiveBox) -> PerspectiveBox {
    clearest((seed, TRIES), make, |exact| {
        let [Some(left), Some(right)] = [exact.vanishing[0], exact.vanishing[1]] else {
            return -1.0;
        };
        let reach = -WIDE_REACH..=1.0 + WIDE_REACH;
        if !(reach.contains(&left[0]) && reach.contains(&right[0])) {
            return -1.0;
        }
        let xs = exact.corners.map(|c| c[0]);
        let wide = xs.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b))
            - xs.iter().fold(f64::INFINITY, |a, b| a.min(*b));
        wide / distance(left, right).max(f64::MIN_POSITIVE)
    })
}

/// The horizon across a wide example.
fn horizon(height: f64) -> Drawn {
    line([-WIDE_REACH - 0.2, height], [1.2 + WIDE_REACH, height])
}

/// `exact`'s vanishing point for `direction`, if its edges meet.
fn point(exact: &PerspectiveBox, direction: Direction) -> Option<Point> {
    exact.vanishing[direction.index()]
}

/// An edge from `a` to `b` as a hand rules it: its ends a hair off.
fn ruled(a: Point, b: Point, turn: usize) -> Drawn {
    let off = if turn.is_multiple_of(2) {
        0.004
    } else {
        -0.004
    };
    line([a[0] + off, a[1] - off], [b[0] - off, b[1] + off])
}

/// Where the line through `a` and `b` meets the horizon at `height`, from the end nearer it.
fn run_on(a: Point, b: Point, height: f64) -> Option<Drawn> {
    let at = meet([a, b], [[0.0, height], [1.0, height]])?;
    let from = if distance(a, at) < distance(b, at) {
        a
    } else {
        b
    };
    Some(line(from, at))
}

/// Lines in pencil back from `exact`'s vanishing point of `direction` to the far end of each of
/// its `missing` edges running that way.
fn back_lines(
    exact: &PerspectiveBox,
    missing: &[(usize, usize, Direction)],
    direction: Direction,
) -> Vec<(On, Marker, Drawn)> {
    let Some(p) = point(exact, direction) else {
        return Vec::new();
    };
    (missing.iter())
        .filter(|(_, _, d)| *d == direction)
        .map(|(a, b, _)| {
            let [a, b] = [exact.corners[*a], exact.corners[*b]];
            let far = if distance(a, p) > distance(b, p) {
                a
            } else {
                b
            };
            (On::Copy, Marker::Pencil, line(p, far))
        })
        .collect()
}

/// `edges` of `exact` ruled with Draw one after another (the `first`'s hair off one way), and each
/// as Compare draws it on to the horizon in blue.
fn rule_edges(
    script: &mut Script,
    exact: &PerspectiveBox,
    edges: &[(usize, usize, Direction)],
    first: usize,
) -> Vec<(On, Marker, Drawn)> {
    let mut found = Vec::new();
    for (turn, (a, b, direction)) in edges.iter().enumerate() {
        let drawn = ruled(exact.corners[*a], exact.corners[*b], turn + first);
        if let (Drawn::Line(ends), Some(_)) = (&drawn, point(exact, *direction))
            && let Some(on) = run_on(ends[0], ends[1], exact.horizon)
        {
            found.push((On::Copy, Marker::Found, on));
        }
        script.draw(On::Copy, Marker::Ink, drawn);
    }
    found
}

/// A box to complete.
pub(super) fn perspective_box() -> Demo {
    let exact = within_reach(BOX_SEED, |s| PerspectiveBox::generate(BoxView::TwoPoint, s));
    let mut script = Script::new();
    script.given(On::Copy, Marker::Guide, horizon(exact.horizon));
    for (a, b, _) in exact.given() {
        script.given(On::Copy, Marker::Given, line(a, b));
    }
    script.step();
    let run: Vec<_> = (exact.given().into_iter())
        .filter_map(|(_, far, direction)| point(&exact, direction).map(|p| (far, p)))
        .collect();
    script.together(
        run.iter()
            .map(|(far, p)| (On::Copy, Marker::Pencil, line(*far, *p)))
            .collect(),
    );
    script.together(
        run.iter()
            .map(|(_, p)| (On::Copy, Marker::Pencil, Drawn::Ring(*p)))
            .collect(),
    );
    let front = exact.front;
    let missing: Vec<(usize, usize, Direction)> = EDGES
        .into_iter()
        .filter(|(a, b, _)| *a != front && *b != front)
        .collect();
    script.step();
    let back = |direction| back_lines(&exact, &missing, direction);
    let (lefts, rights) = (back(Direction::Left), back(Direction::Right));
    for (left, right) in lefts.into_iter().zip(rights) {
        script.together(vec![left, right]);
    }
    script.step();
    let found = rule_edges(&mut script, &exact, &missing, 0);
    let mut compared: Vec<(On, Marker, Drawn)> = (exact.edges().into_iter())
        .map(|(a, b, _)| (On::Copy, Marker::Answer, line(a, b)))
        .collect();
    compared.extend(
        exact
            .vanishing
            .iter()
            .flatten()
            .map(|p| (On::Copy, Marker::Answer, Drawn::Ring(*p))),
    );
    compared.extend(found);
    script.compare(Family::Box, Layout::Wide, compared)
}

/// Free boxes on a horizon: one drawn as the method draws it.
pub(super) fn free_boxes() -> Demo {
    let example = within_reach(FREE_SEED, |s| {
        PerspectiveBox::on_horizon(Horizon::Middle, s)
    });
    let c = example.corners;
    let [left, right] = [Direction::Left, Direction::Right].map(|d| point(&example, d));
    let mut script = Script::new();
    script.given(On::Copy, Marker::Guide, horizon(example.horizon));
    script.step();
    let rings = [left, right].into_iter().flatten();
    script.together(
        rings
            .map(|p| (On::Copy, Marker::Pencil, Drawn::Ring(p)))
            .collect(),
    );
    script.step();
    script.draw(On::Copy, Marker::Ink, ruled(c[0], c[2], 0));
    script.step();
    for corner in [0, 2] {
        let guides = [left, right].into_iter().flatten();
        script.together(
            guides
                .map(|p| (On::Copy, Marker::Pencil, line(c[corner], p)))
                .collect(),
        );
    }
    let edges = |pairs: &[(usize, usize)]| -> Vec<(usize, usize, Direction)> {
        (EDGES.into_iter())
            .filter(|(a, b, _)| pairs.contains(&(*a, *b)))
            .collect()
    };
    let near = edges(&[(0, 1), (2, 3), (0, 4), (2, 6)]);
    let far = edges(&[(1, 3), (4, 6), (5, 7), (1, 5), (3, 7), (4, 5), (6, 7)]);
    let mut found = rule_edges(&mut script, &example, &near, 0);
    script.step();
    found.extend(rule_edges(&mut script, &example, &far, 1));
    found.extend(
        [left, right]
            .into_iter()
            .flatten()
            .map(|p| (On::Copy, Marker::Found, Drawn::Ring(p))),
    );
    script.compare(Family::FreeBoxes, Layout::Wide, found)
}

/// An ellipse in a square in perspective.
pub(super) fn ellipse() -> Demo {
    // A square lying as a cylinder's top does: its ellipse about half as deep as wide, lying
    // nearly level, large in its frame.
    let guide = clearest(
        (ELLIPSE_SEED, 24),
        |seed| EllipseGuide::generate(Guide::Plane, seed),
        |guide| {
            let exact = guide.exact;
            exact.axes[0].min(0.35) - (exact.roundness() - 0.5).abs()
                + 0.2 * exact.angle.cos().abs()
        },
    );
    let [c0, c1, c2, c3] = guide.corners;
    let t = guide.touches;
    let mut script = Script::new();
    script.given(
        On::Copy,
        Marker::Given,
        Drawn::Line(vec![c0, c1, c2, c3, c0]),
    );
    script.step();
    script.draw(On::Copy, Marker::Pencil, line(c0, c2));
    script.draw(On::Copy, Marker::Pencil, line(c1, c3));
    script.step();
    script.draw(On::Copy, Marker::Pencil, line(t[0], t[2]));
    script.draw(On::Copy, Marker::Pencil, line(t[1], t[3]));
    script.together(
        t.iter()
            .map(|p| (On::Copy, Marker::Pencil, Drawn::Dot(*p)))
            .collect(),
    );
    script.step();
    for phase in [0.3, 1.9] {
        let pass = wobbled(&guide.exact.outline(PIECES), 0.01, phase);
        script.draw(On::Copy, Marker::Air, Drawn::Line(pass));
    }
    script.step();
    let exact = guide.exact;
    let drawn = Ellipse {
        axes: [exact.axes[0] * 1.04, exact.axes[1] * 0.97],
        angle: exact.angle + 0.05,
        ..exact
    }
    .outline(PIECES);
    script.draw(On::Copy, Marker::Ink, Drawn::Line(drawn.clone()));
    let mut compared = vec![(On::Copy, Marker::Answer, Drawn::Line(exact.outline(PIECES)))];
    compared.extend(
        t.iter()
            .map(|p| (On::Copy, Marker::Answer, Drawn::Ring(*p))),
    );
    compared.push((On::Copy, Marker::Found, Drawn::Line(drawn)));
    script.compare(Family::Ellipse, Layout::Single, compared)
}
