//! A placement's How (S153): V大's steps on an example beside its plan: the ground's halves and
//! quarters by its diagonals, the footprints' corners landing on the example's exact corners, the
//! heights carried to its exact tops, the edges in ink; Compare's red the example's seen edges, its
//! hidden ones dashed, and nothing ringed (the eye judges).

use openpaint_practice::{CELLS, DemoLayout, Drawn, Kind, Marker, On, Placement, Point, distance};

#[test]
fn a_placement_s_example_finds_its_corners_and_tops_where_its_answer_has_them() {
    let kind = Kind::Placement(Placement::ALL[0]);
    let demo = kind.how(CELLS).expect("a How");
    assert_eq!(demo.layout, DemoLayout::Pair);
    assert_eq!(demo.steps.len(), 5, "grid, corners, heights, ink, Compare");
    assert!(demo.view.is_none(), "both frames whole");
    // The plan printed beside from the start: its ground, footprints and front view.
    let plan = (demo.marks.iter())
        .filter(|m| m.on == On::Original && m.pen == Marker::Printed && m.to == 0.0)
        .count();
    assert!(plan >= 12, "{plan}");
    // Each corner and top found is dotted where it lies.
    let dots: Vec<Point> = (demo.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Dot(p) if m.pen == Marker::Construction => Some(p),
            _ => None,
        })
        .collect();
    let red: Vec<[Point; 2]> = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .filter_map(|m| match &m.drawn {
            Drawn::Line(points) if points.len() == 2 => Some([points[0], points[1]]),
            _ => None,
        })
        .collect();
    assert!(red.len() >= 12, "three boxes' edges seen");
    let ends: Vec<Point> = red.iter().flatten().copied().collect();
    // The example's front corners' tops are ends of its red edges, and dotted.
    let near_dots = dots
        .iter()
        .filter(|d| ends.iter().any(|e| distance(**d, *e) < 1e-9));
    assert!(
        near_dots.count() >= 3,
        "a top or corner dotted on the answer for every box"
    );
    let hidden = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Hidden)
        .count();
    assert!(hidden > 0, "what the boxes hide dashed");
    assert!(
        demo.marks
            .iter()
            .all(|m| !matches!(m.drawn, Drawn::Ring(_)))
    );
}
