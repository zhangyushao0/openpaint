//! An organic arrow's How (S156l): Drawabox's way on an example of the level's own: the path and
//! the arrowhead given, the width marked in pencil along the path, the edges and the folds in
//! ink, then Compare's red the ribbon, what it hides dashed, nothing ringed.

use openpaint_practice::{
    Arrow, ArrowTask, CELLS, DemoLayout, Drawn, Kind, Marker, derived, distance,
};

#[test]
fn an_arrow_example_marks_its_widths_near_wide_far_narrow_and_lays_its_ribbon() {
    for arrow in Arrow::ALL {
        let demo = Kind::Arrow(arrow).how(CELLS).expect("a How");
        assert_eq!(demo.layout, DemoLayout::Single);
        assert_eq!(demo.steps.len(), 4);
        let example = ArrowTask::generate(arrow, derived(0xDE30_5EED_0156, 0));
        let given = |m: &&openpaint_practice::Mark| m.to == 0.0;
        let path = (demo.marks.iter().filter(given))
            .find(|m| m.pen == Marker::PrintedHidden)
            .expect("the path");
        assert_eq!(
            path.drawn,
            Drawn::Line(example.path.clone()),
            "the example's"
        );
        // The widths marked in pencil: each a width across the ribbon, the far ones narrower.
        let widths: Vec<f64> = (demo.marks.iter())
            .filter(|m| m.pen == Marker::Pencil)
            .map(|m| match &m.drawn {
                Drawn::Line(points) => distance(points[0], points[1]),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(widths.len(), 5);
        assert!(widths[4] < widths[0] / 1.5, "{arrow:?}: {widths:?}");
        // Compare: the ribbon's seen stretches red, its hidden ones dashed.
        let answer = &example.answer;
        let stretches = (answer.stretches.iter().flatten()).chain(answer.folds.iter().flatten());
        let hidden = stretches.clone().filter(|s| s.hidden).count();
        let count = |pen: Marker| demo.marks.iter().filter(|m| m.pen == pen).count();
        assert_eq!(count(Marker::Answer), stretches.count() - hidden);
        assert_eq!(count(Marker::Hidden), hidden);
        assert_eq!(
            count(Marker::Ink),
            count(Marker::Answer),
            "{arrow:?}: each seen one inked"
        );
        assert!(!demo.marks.iter().any(|m| matches!(m.drawn, Drawn::Ring(_))));
        assert!(demo.view.is_some(), "framed on the ribbon");
    }
}
