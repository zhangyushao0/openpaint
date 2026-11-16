//! An ellipse's demonstration (`demos/demo.rs`, week 2): the diagonals, the four middles through
//! the centre, passes in the air, one stroke, then compared as the exercise compares it (S145):
//! the exact ellipse red over the ink, for the eye. A funnel's (S156l): the axis first and each
//! ellipse's long axis square to it, passes in the air round it, each ellipse between its marks
//! touching both sides (the second a little turned off the axis), then the exact ellipses red.

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Script, clearest, line, wobbled};
use crate::geometry::ellipse::{Ellipse, EllipseGuide, Guide};
use crate::kinds::ellipses::funnel::{Funnel, MARK};

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const ELLIPSE_SEED: u64 = 0xDE30_5EED_0009;
const FUNNEL_SEED: u64 = 0xDE30_5EED_0156;
/// How far the hand turns the second ellipse off the funnel's axis (radians).
const TURNED_OFF: f64 = 0.1;
/// An ellipse's outline in pieces.
const PIECES: usize = 96;

/// An ellipse in a square in perspective.
pub(crate) fn ellipse() -> Demo {
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
        Marker::Printed,
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
    // The hand's ellipse a little wide, turned and to the left: the red shows it apart.
    let exact = guide.exact;
    let drawn = Ellipse {
        centre: [exact.centre[0] - 0.02, exact.centre[1]],
        axes: [exact.axes[0] * 1.04, exact.axes[1] * 0.97],
        angle: exact.angle + 0.05,
    }
    .outline(PIECES);
    script.draw(On::Copy, Marker::Ink, Drawn::Line(drawn));
    let compared = vec![(On::Copy, Marker::Answer, Drawn::Line(exact.outline(PIECES)))];
    script.compare(DemoLayout::Single, compared)
}

/// A funnel of three ellipses on one axis.
pub(crate) fn funnel() -> Demo {
    let funnel = Funnel::generate(FUNNEL_SEED);
    let mut script = Script::new();
    for [a, b] in funnel.sides {
        script.given(On::Copy, Marker::Printed, line(a, b));
    }
    script.given(
        On::Copy,
        Marker::PrintedHidden,
        line(funnel.axis[0], funnel.axis[1]),
    );
    let across = funnel.across;
    let by = |p: [f64; 2], s: f64| [p[0] + s * across[0], p[1] + s * across[1]];
    for ring in &funnel.rings {
        script.given(On::Copy, Marker::Printed, line(ring.ends[0], ring.ends[1]));
        for end in ring.ends {
            let mark = line(by(end, -MARK), by(end, MARK));
            script.given(On::Copy, Marker::Printed, mark);
        }
    }
    // The axis first, then each ellipse's long axis square to it.
    script.step();
    script.draw(
        On::Copy,
        Marker::Pencil,
        line(funnel.axis[0], funnel.axis[1]),
    );
    let long = (funnel.rings.iter())
        .map(|ring| {
            let (c, a) = (ring.exact.centre, ring.exact.axes[0]);
            (On::Copy, Marker::Pencil, line(by(c, -a), by(c, a)))
        })
        .collect();
    script.together(long);
    script.step();
    for phase in [0.3, 1.9] {
        let pass = wobbled(&funnel.rings[0].exact.outline(PIECES), 0.008, phase);
        script.draw(On::Copy, Marker::Air, Drawn::Line(pass));
    }
    // Each between its marks, touching both sides; the second turned a little off the axis.
    script.step();
    for (k, ring) in funnel.rings.iter().enumerate() {
        let turned = if k == 1 { TURNED_OFF } else { 0.0 };
        let drawn = Ellipse {
            angle: ring.exact.angle + turned,
            ..ring.exact
        };
        script.draw(On::Copy, Marker::Ink, Drawn::Line(drawn.outline(PIECES)));
    }
    let compared = (funnel.rings.iter())
        .map(|ring| {
            (
                On::Copy,
                Marker::Answer,
                Drawn::Line(ring.exact.outline(PIECES)),
            )
        })
        .collect();
    script.compare(DemoLayout::Single, compared)
}
