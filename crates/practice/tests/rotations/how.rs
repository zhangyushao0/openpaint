//! A rotation's How (S113): V大's steps on an example beside its picture: the box round the object
//! in the picture, the same box at the new angle, the change's points by halves and thirds landing
//! on the turned object's corners, the visible edges in ink; Compare's red the asked view's seen
//! edges, its hidden ones dashed, and nothing ringed (the eye judges).

use openpaint_practice::{
    CELLS, DemoLayout, Drawn, Kind, Marker, ObjectTask, On, Point, Rotation, RotationTask, derived,
    distance,
};

#[test]
fn a_rotation_s_example_turns_its_picture_s_object_and_its_corners_land_on_the_construction() {
    let demo = Kind::Rotation(Rotation::ALL[1]).how(CELLS).expect("a How");
    assert_eq!(demo.layout, DemoLayout::Pair);
    let picture = demo.shaded.as_ref().expect("the example's picture");
    let shown = picture.downcast::<ObjectTask>().expect("an object");
    let example = RotationTask::generate(Rotation::ALL[1], derived(0xDE30_5EED_0113, 0));
    assert_eq!(shown, &example.shown, "the picture is view A");
    let asked = &example.asked;
    assert_eq!(
        asked.stages.len(),
        1,
        "one change: one step of construction"
    );
    let at = |pen: Marker, on: On| {
        (demo.marks.iter())
            .filter(|m| m.pen == pen && m.on == on && m.to > 0.0)
            .count()
    };
    assert_eq!(
        at(Marker::Pencil, On::Original),
        12,
        "the box round the object, through"
    );
    assert_eq!(
        at(Marker::Pencil, On::Copy),
        9,
        "the same box at the new angle"
    );
    // Each corner found is dotted where the turned object's corner projects.
    let dots: Vec<Point> = (demo.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Dot(p) if m.pen == Marker::Construction => Some(p),
            _ => None,
        })
        .collect();
    for (found, corner) in asked.stages[0].found.iter().zip(&asked.stages[0].at) {
        assert!(distance(*found, asked.projected(*corner)) < 1e-9);
        assert!(dots.iter().any(|d| distance(*d, *found) < 1e-12));
    }
    // Compare: the asked view's seen edges red, its hidden ones dashed; nothing ringed.
    let red = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .count();
    let seen = (asked.answer.outlines.iter().flatten())
        .filter(|s| !s.hidden)
        .count();
    assert_eq!(red, asked.answer.sides.len() + seen);
    let hidden = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Hidden)
        .count();
    assert!(hidden >= asked.hidden.len() && !asked.hidden.is_empty());
    assert!(
        demo.marks
            .iter()
            .all(|m| !matches!(m.drawn, Drawn::Ring(_)))
    );
    assert!(demo.view.is_some(), "cropped to the two boxes");
}
