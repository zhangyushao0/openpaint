//! A curved rotation's How (S154f): V大's steps on an example beside its picture: the box round the
//! body in the picture and its axis, the same box at the new angle, each section's square found
//! with its middles on the exact ellipse, the ellipses and the outlines, the visible lines in ink;
//! Compare's red the asked view's seen lines, its hidden ones dashed, nothing ringed.

use openpaint_practice::{
    CELLS, CurvedRotation, CurvedTask, CurvedView, DemoLayout, Drawn, Kind, Marker, On, Point,
    derived, distance,
};

#[test]
fn a_curved_example_lays_its_picture_s_body_another_way_and_its_sections_on_the_construction() {
    let demo = Kind::CurvedRotation(CurvedRotation::ALL[0])
        .how(CELLS)
        .expect("a How");
    assert_eq!(demo.layout, DemoLayout::Pair);
    assert_eq!(demo.steps.len(), 6);
    let picture = demo.shaded.as_ref().expect("the example's picture");
    let shown = picture.downcast::<CurvedView>().expect("a view");
    let example = CurvedTask::generate(CurvedRotation::ALL[0], derived(0xDE30_5EED_0154, 0));
    assert_eq!(shown, &example.shown, "the picture is view A");
    let asked = &example.asked;
    let at = |pen: Marker, on: On| {
        (demo.marks.iter())
            .filter(|m| m.pen == pen && m.on == on && m.to > 0.0)
            .count()
    };
    assert_eq!(
        at(Marker::Pencil, On::Original),
        13,
        "the box round the body and its axis"
    );
    // The box completed, then the ellipses and the outlines.
    let pencil = 9 + asked.sections.len() + asked.answer.sides.len();
    assert_eq!(at(Marker::Pencil, On::Copy), pencil);
    // Each section's middles dotted where its exact ellipse touches its square.
    let dots: Vec<Point> = (demo.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Dot(p) if m.pen == Marker::Construction => Some(p),
            _ => None,
        })
        .collect();
    for section in &asked.sections {
        for touch in section.guide.touches {
            assert!(dots.iter().any(|d| distance(*d, touch) < 1e-7));
        }
    }
    // Compare: the asked view's seen lines red, its hidden ones dashed; nothing ringed.
    let answer = &asked.answer;
    let stretches = answer.curves.iter().flat_map(|c| &c.stretches);
    let red = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Answer)
        .count();
    let seen = stretches.clone().filter(|s| !s.hidden).count();
    assert_eq!(red, answer.sides.len() + seen);
    let hidden = (demo.marks.iter())
        .filter(|m| m.pen == Marker::Hidden)
        .count();
    assert_eq!(
        hidden,
        answer.hidden.len() + stretches.filter(|s| s.hidden).count()
    );
    assert!(hidden > 0);
    assert!(!demo.marks.iter().any(|m| matches!(m.drawn, Drawn::Ring(_))));
    assert!(demo.view.is_some(), "cropped to the two boxes");
}
