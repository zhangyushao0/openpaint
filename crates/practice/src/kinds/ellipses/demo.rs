//! An ellipse's demonstration (`demos/demo.rs`, week 2): the diagonals, the four middles through
//! the centre, passes in the air, one stroke, then compared as the exercise compares it (S97):
//! the answer red, the middles it misses ringed.

use crate::demos::demo::{Demo, Drawn, Layout, Marker, On, Script, clearest, line, wobbled};
use crate::demos::demo_space::red_rings;
use crate::geometry::ellipse::{Ellipse, EllipseGuide, Guide};
use crate::kinds::ellipses::ellipse_check::EllipseCheck;
use crate::sessions::plan::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const ELLIPSE_SEED: u64 = 0xDE30_5EED_0009;
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
    // The hand's ellipse a little wide, turned and to the left: its middles at the sides it falls
    // short of or runs past are ringed.
    let exact = guide.exact;
    let drawn = Ellipse {
        centre: [exact.centre[0] - 0.02, exact.centre[1]],
        axes: [exact.axes[0] * 1.04, exact.axes[1] * 0.97],
        angle: exact.angle + 0.05,
    }
    .outline(PIECES);
    script.draw(On::Copy, Marker::Ink, Drawn::Line(drawn.clone()));
    let mut compared = vec![(On::Copy, Marker::Answer, Drawn::Line(exact.outline(PIECES)))];
    let check = EllipseCheck::of(&guide, &[&drawn]);
    compared.extend(red_rings(check.sides.iter().map(|missed| missed.middle)));
    script.compare(Family::Ellipse, Layout::Single, compared)
}
