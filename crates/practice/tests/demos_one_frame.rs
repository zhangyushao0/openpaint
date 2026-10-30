//! The demonstrations of the kinds built in one frame (S97): a box's and free boxes' Compare the
//! points ringed red and only the edge drawn off run on past its point, nothing blue; an
//! ellipse's rings only on the middles it misses; a value scale of nine numbered in the order of
//! halving and painted a round at a time, its uneven cell outlined with its even one.

use openpaint_practice::{Demo, Drawn, Family, Layout, Marker, On, PAST, Point, distance};

fn demo(family: Family) -> Demo {
    Demo::of(family).unwrap_or_else(|| panic!("{family:?} has no demonstration"))
}

/// How far the line from `from` to `to` passes `p` short of `to`, and how far off it `p` lies.
fn passing([from, to]: [Point; 2], p: Point) -> (f64, f64) {
    let (d, long) = ([to[0] - from[0], to[1] - from[1]], distance(from, to));
    let along = ((p[0] - from[0]) * d[0] + (p[1] - from[1]) * d[1]) / long;
    let off = ((p[0] - from[0]) * d[1] - (p[1] - from[1]) * d[0]).abs() / long;
    (long - along, off)
}

#[test]
fn a_box_s_compare_rings_its_points_and_runs_on_only_the_edge_drawn_off() {
    for family in [Family::Box, Family::FreeBoxes] {
        let demo = demo(family);
        let red_rings: Vec<Point> = (demo.marks.iter())
            .filter_map(|m| match m.drawn {
                Drawn::Ring(p) if m.pen == Marker::Answer => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(red_rings.len(), 2, "{family:?}");
        assert!(
            demo.marks.iter().all(|m| m.pen != Marker::Found),
            "{family:?}"
        );
        let missed: Vec<[Point; 2]> = (demo.marks.iter())
            .filter(|m| m.pen == Marker::Missed)
            .filter_map(|m| match &m.drawn {
                Drawn::Line(points) => Some([points[0], points[points.len() - 1]]),
                _ => None,
            })
            .collect();
        assert_eq!(missed.len(), 1, "{family:?}");
        let (past, off) = (red_rings.into_iter())
            .map(|p| passing(missed[0], p))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .expect("its point");
        assert!((past - PAST).abs() < 1e-9, "{family:?}: {past}");
        assert!(off > 0.01, "{family:?}: misses its point by {off}");
    }
}

#[test]
fn an_ellipses_compare_rings_only_the_middles_it_misses() {
    let demo = demo(Family::Ellipse);
    let rings = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Answer && matches!(m.drawn, Drawn::Ring(_)))
        .count();
    assert!((1..4).contains(&rings), "{rings}");
    assert!(demo.marks.iter().all(|m| m.pen != Marker::Found));
}

/// The How of a scale follows the cell count of the scale in hand (the review of S97): of five,
/// its order 2 1 2 under the inner cells, the 3rd painted first, then the 2nd and 4th, Compare.
#[test]
fn a_scale_of_five_is_numbered_and_painted_as_a_five_cell_row_is() {
    assert_eq!(
        Demo::of_scale(9),
        demo(Family::Scale),
        "nine's is the kind's"
    );
    let scale = Demo::of_scale(5);
    assert_eq!((scale.family, scale.layout), (Family::Scale, Layout::Row));
    let numbered: Vec<(f64, u8)> = (scale.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Numeral(at, n) if m.to == 0.0 => Some((at[0], n)),
            _ => None,
        })
        .collect();
    let mut along = numbered.clone();
    along.sort_by(|a, b| a.0.total_cmp(&b.0));
    let left_to_right: Vec<u8> = along.iter().map(|(_, n)| *n).collect();
    assert_eq!(left_to_right, [2, 1, 2]);
    assert_eq!(
        scale.steps.len(),
        3,
        "the middle, the halves' middles, Compare"
    );
    let fills: Vec<f64> = (scale.marks.iter())
        .filter(|m| m.on == On::Copy && m.pen == Marker::Ink)
        .map(|m| m.from)
        .collect();
    let in_step = |at: usize| {
        let (start, end) = (scale.steps[at], scale.steps[at + 1]);
        fills
            .iter()
            .filter(|from| (start..end).contains(*from))
            .count()
    };
    assert_eq!([in_step(0), in_step(1)], [1, 2]);
    let outlined = (scale.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .count();
    assert_eq!(outlined, 1, "the uneven cell with its even one");
    assert!((4.5..=10.3).contains(&scale.length), "{}", scale.length);
}

#[test]
fn a_scale_of_nine_is_numbered_and_painted_a_round_at_a_time() {
    let scale = demo(Family::Scale);
    assert_eq!(scale.layout, Layout::Row);
    let numbered: Vec<u8> = (scale.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Numeral(_, n) if m.to == 0.0 => Some(n),
            _ => None,
        })
        .collect();
    assert_eq!(numbered, [1, 2, 2, 3, 3, 3, 3]);
    let fills: Vec<f64> = (scale.marks.iter())
        .filter(|m| m.on == On::Copy && m.pen == Marker::Ink)
        .map(|m| m.from)
        .collect();
    assert_eq!(fills.len(), 7, "every cell between the ends");
    let in_step = |at: usize| {
        let (start, end) = (scale.steps[at], scale.steps[at + 1]);
        fills
            .iter()
            .filter(|from| (start..end).contains(*from))
            .count()
    };
    assert_eq!([in_step(0), in_step(1), in_step(2)], [1, 2, 4]);
    let outlined = (scale.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .count();
    assert_eq!(outlined, 1, "the uneven cell with its even one");
}
