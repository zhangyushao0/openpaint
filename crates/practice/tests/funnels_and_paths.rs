//! Funnels and path pages (S156l), the harder variants at ellipses' and free boxes' 稳: a funnel's
//! three ellipses aligned on its axis, centred between their marks and touching both sides exactly;
//! Compare laying them (or a path page's horizon run on) for the eye without reading the copy;
//! their Hows; their places in the 稳 lessons.

use openpaint_practice::{
    Asked, CELLS, CardItem, Choices, Compare, Copied, DemoLayout, Drawn, DrawnCopy, Exercise,
    Figure, FreeCompared, Funnel, FunnelCompared, Guide, Horizon, In, Kind, Level, LinePen, Marker,
    PathPage, Point, Rgb8, Seen, SheetMark, Verdict, distance,
};

/// Within this a point lies where it should (frame units).
const EXACT: f64 = 1e-9;

/// A copy that holds something but must never be read.
struct Unread;

impl Copied for Unread {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        panic!("a drawn copy judged by eye is never read")
    }

    fn drawn(&self) -> DrawnCopy {
        panic!("a drawn copy judged by eye is never read")
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        panic!("a drawn copy judged by eye is never read")
    }
}

fn seen(construction: bool) -> Seen<'static> {
    Seen {
        construction,
        shown: 1.0,
        chosen: 0,
        outline: &[],
        grey: false,
        view: 0,
        fit: None,
        probe: None,
    }
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

/// That `funnel`'s ellipses lie apart along its axis from its narrow end, neither overlapping its
/// neighbour, each wider than the one before.
fn apart(funnel: &Funnel, seed: u64) {
    let from = funnel.axis[0];
    let at = |k: usize| {
        let centre = funnel.rings[k].exact.centre;
        dot([centre[0] - from[0], centre[1] - from[1]], funnel.along)
    };
    for k in 1..3 {
        let (a, b) = (funnel.rings[k - 1].exact, funnel.rings[k].exact);
        let step = at(k) - at(k - 1);
        assert!(step >= 0.12 - EXACT, "{seed}");
        assert!(step >= a.axes[1] + b.axes[1] + 0.05 - EXACT, "{seed}");
        assert!(b.axes[0] > a.axes[0], "{seed}: wider towards its wide end");
    }
}

#[test]
fn a_funnel_s_ellipses_lie_on_its_axis_between_their_marks_and_touch_both_sides_exactly() {
    let mut degrees: Vec<f64> = Vec::new();
    for seed in 0..300 {
        let funnel = Funnel::generate(seed);
        assert_eq!(funnel, Funnel::generate(seed), "{seed}: per seed");
        let (along, across) = (funnel.along, funnel.across);
        assert!(dot(along, across).abs() < EXACT && (dot(along, along) - 1.0).abs() < EXACT);
        let [from, to] = funnel.axis;
        for (k, ring) in funnel.rings.iter().enumerate() {
            let e = ring.exact;
            // Its short axis along the funnel's, its centre on it between its marks.
            let major = [e.angle.cos(), e.angle.sin()];
            assert!(dot(major, along).abs() < EXACT, "{seed} {k}: aligned");
            let off = (e.centre[0] - from[0]) * across[0] + (e.centre[1] - from[1]) * across[1];
            assert!(off.abs() < EXACT, "{seed} {k}: on the axis");
            let middle = [0, 1].map(|i| (ring.ends[0][i] + ring.ends[1][i]) / 2.0);
            assert!(distance(middle, e.centre) < EXACT, "{seed} {k}");
            assert!((distance(ring.ends[0], ring.ends[1]) - 2.0 * e.axes[1]).abs() < EXACT);
            // Touching each side exactly: its reach out along the side's normal is the side's.
            for (side, touch) in funnel.sides.iter().zip(ring.touches) {
                let way = [side[1][0] - side[0][0], side[1][1] - side[0][1]];
                let long = way[0].hypot(way[1]);
                let mut normal = [way[1] / long, -way[0] / long];
                if dot(normal, [side[0][0] - from[0], side[0][1] - from[1]]) < 0.0 {
                    normal = [-normal[0], -normal[1]];
                }
                let line = dot(normal, side[0]);
                assert!(
                    (e.support(normal) - line).abs() < EXACT,
                    "{seed} {k}: touches"
                );
                assert!(
                    (dot(normal, touch) - line).abs() < EXACT,
                    "{seed} {k}: on the side"
                );
                assert!(e.distance(touch) < 1e-9, "{seed} {k}: on the ellipse");
            }
            degrees.push(e.degree());
        }
        apart(&funnel, seed);
        let inside = |p: &Point| p.iter().all(|v| (0.06..=0.94).contains(v));
        assert!(
            funnel.sides.iter().flatten().chain(&[from, to]).all(inside),
            "{seed}"
        );
    }
    // Its degree changes along it, narrow to round.
    let low = degrees.iter().copied().fold(90.0, f64::min);
    let high = degrees.iter().copied().fold(0.0, f64::max);
    assert!(low < 18.0 && high > 40.0, "{low} {high}");
}

#[test]
fn compare_lays_a_funnel_s_ellipses_or_a_path_page_s_horizon_for_the_eye_alone() {
    for kind in [Kind::Ellipse(Guide::Funnel), Kind::FreeBoxes(Horizon::Path)] {
        for seed in 0..6 {
            let exercise = Exercise::generated(kind, seed);
            let original = exercise.original();
            let asked = Asked {
                exercise: &exercise,
                original: &*original,
                picture: None,
                copy: &Unread,
                choices: Choices::default(),
            };
            let Some(Compare::Checked(shown)) = original.compare(&asked) else {
                panic!("{kind:?}: laid as geometry")
            };
            assert_eq!(shown.verdict(), Verdict::Eye, "{kind:?}");
            assert!(shown.metrics().is_empty(), "{kind:?}");
            let marks = shown.marks(&seen(false));
            if let Some(funnel) = original.downcast::<Funnel>() {
                let laid = shown.downcast::<FunnelCompared>().expect("a funnel's");
                assert_eq!(&laid.funnel, funnel);
                assert_eq!(shown.card(&seen(false)), [CardItem::Construction]);
                let red = marks.iter().filter(|m| answer_line(m)).count();
                assert_eq!(red, 3, "three ellipses");
                let built = shown.marks(&seen(true));
                let dots = built
                    .iter()
                    .filter(|m| matches!(m.drawn, Figure::Dot { .. }));
                assert_eq!(dots.count(), 6, "where each touches the sides");
                // Its sides and marks printed, its axis dashed.
                let givens = original.givens();
                assert_eq!(givens.len(), 2 + 1 + 3 * 3);
                assert!(givens.iter().all(|m| m.on == In::Copy));
            } else {
                let page = original.downcast::<PathPage>().expect("a path page");
                let horizon = shown
                    .downcast::<FreeCompared>()
                    .expect("a free page's")
                    .horizon;
                assert!((horizon - page.horizon).abs() < 1e-12 && (horizon - 0.4).abs() < 1e-12);
                assert!(shown.card(&seen(false)).is_empty());
                let givens = original.givens();
                assert!(matches!(givens[0].drawn, Figure::Horizon(h) if h == page.horizon));
                assert_eq!(givens.len(), 3, "the horizon and the path's two edges");
            }
        }
    }
}

/// Whether `mark` is a line in the answer's red.
fn answer_line(mark: &SheetMark) -> bool {
    matches!(
        mark.drawn,
        Figure::Line {
            pen: LinePen::Answer,
            ..
        }
    )
}

#[test]
fn a_funnel_s_and_a_path_page_s_hows_play_on_their_own_examples() {
    let funnel = Kind::Ellipse(Guide::Funnel);
    let demo = funnel.how(CELLS).expect("a How");
    assert_eq!((demo.layout, demo.steps.len()), (DemoLayout::Single, 4));
    let example = Funnel::generate(0xDE30_5EED_0156);
    let red: Vec<&Drawn> = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .map(|m| &m.drawn)
        .collect();
    let exact: Vec<Drawn> = (example.rings.iter())
        .map(|ring| Drawn::Line(ring.exact.outline(96)))
        .collect();
    assert_eq!(red, exact.iter().collect::<Vec<_>>());
    assert_ne!(funnel.facts().key, Kind::Ellipse(Guide::Plane).facts().key);
    let path = Kind::FreeBoxes(Horizon::Path);
    let demo = path.how(CELLS).expect("a How");
    assert_eq!(demo.steps.len(), 4, "a box a step, then Compare");
    let inked = demo.marks.iter().filter(|m| m.pen == Marker::Ink).count();
    assert_eq!(inked, 27, "three boxes' nine seen edges");
    assert_eq!(path.facts().key, "free-boxes-path");
}
