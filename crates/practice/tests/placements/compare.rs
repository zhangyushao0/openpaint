//! A placement's givens and Compare (S153): the plan printed in the original's frame, the ground,
//! its horizon and the unit in the copy's; Compare lays the exact answer over the copy, seen edges
//! red and hidden ones dashed, a construction to lay with it, and leaves the result to the eye
//! (D123): the copy's strokes are never read.

use openpaint_practice::{
    Asked, CardItem, Choices, Compare, Copied, DrawnCopy, Exercise, Figure, In, Kind, Learner,
    Level, LinePen, Pictures, Placement, PlacementTask, Point, Rgb8, Seen, SheetMark, Skill,
    SkillLevel, Standing, Verdict, schedule,
};

/// A copy that holds something but must never be read.
struct Unread;

impl Copied for Unread {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        panic!("a placement's copy is never read")
    }

    fn drawn(&self) -> DrawnCopy {
        panic!("a placement's copy is never read")
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        panic!("a placement's copy is never read")
    }
}

/// An empty copy.
struct Empty;

impl Copied for Empty {
    fn holds(&self) -> bool {
        false
    }

    fn picks(&self) -> Vec<Rgb8> {
        Vec::new()
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy::default()
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        Vec::new()
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

/// The lines among `marks` in `pen` in frame `on`: whole and dashed.
fn lines(marks: &[SheetMark], on: In, pen: LinePen) -> [usize; 2] {
    let of = |dashed: bool| {
        (marks.iter())
            .filter(|mark| mark.on == on)
            .filter(|mark| {
                matches!(&mark.drawn, Figure::Line { pen: p, dashed: d, .. } if *p == pen && *d == dashed)
            })
            .count()
    };
    [of(false), of(true)]
}

fn count(marks: &[SheetMark], on: In, kind: impl Fn(&Figure) -> bool) -> usize {
    (marks.iter())
        .filter(|mark| mark.on == on && kind(&mark.drawn))
        .count()
}

#[test]
fn compare_lays_the_answer_for_the_eye_and_never_reads_the_copy() {
    for placement in Placement::ALL {
        let exercise = Exercise::generated(Kind::Placement(placement), 0x5EED_0153);
        let original = exercise.original();
        let task = original.downcast::<PlacementTask>().expect("a placement");
        let asked = |copy: &'static dyn Copied| Asked {
            exercise: &exercise,
            original: &*original,
            picture: None,
            copy,
            choices: Choices::default(),
        };
        assert!(original.compare(&asked(&Empty)).is_none(), "nothing drawn");
        let Some(Compare::Checked(shown)) = original.compare(&asked(&Unread)) else {
            panic!("{placement:?}: checked, a construction to lay");
        };
        assert_eq!(shown.verdict(), Verdict::Eye, "the learner judges");
        assert!(shown.metrics().is_empty(), "nothing kept");
        assert_eq!(shown.card(&seen(false)), [CardItem::Construction]);
        // The answer: the seen edges and outlines red, what the pieces hide dashed red.
        let curves = task.answer.outlines.iter().flatten();
        let seen_curves = curves.clone().filter(|s| !s.hidden).count();
        let hidden_curves = curves.filter(|s| s.hidden).count();
        let marks = shown.marks(&seen(false));
        let [whole, dashed] = lines(&marks, In::Copy, LinePen::Answer);
        assert_eq!(
            whole,
            task.answer.sides.len() + seen_curves,
            "{placement:?}"
        );
        assert_eq!(dashed, task.hidden.len() + hidden_curves, "{placement:?}");
        assert_eq!(count(&marks, In::Copy, |f| matches!(f, Figure::Ring(_))), 0);
        assert_eq!(lines(&marks, In::Copy, LinePen::Pencil), [0, 0]);
        assert!(marks.iter().all(|mark| mark.on == In::Copy));
        // 看画法: the grid and height lines dashed, the upright and footprints in pencil, every
        // corner and top it finds ringed.
        let built = &task.construction;
        let constructed = shown.marks(&seen(true));
        let rounds = (task.plan.pieces.iter())
            .filter(|p| matches!(p.footprint, openpaint_practice::Footprint::Round { .. }))
            .count();
        let carried: usize = built.carried.iter().map(|c| c.lines.len()).sum();
        assert_eq!(
            lines(&constructed, In::Copy, LinePen::Pencil),
            [
                1 + built.footprints.len() + rounds,
                built.grid.len() + carried + built.runs.len()
            ]
        );
        let found = count(&constructed, In::Copy, |f| matches!(f, Figure::Found(_)));
        assert_eq!(found, 5 * task.plan.pieces.len());
    }
}

#[test]
fn the_givens_are_the_plan_and_the_ground_with_its_upright_or_at_the_third_level_its_unit() {
    for placement in Placement::ALL {
        let original = Exercise::generated(Kind::Placement(placement), 0x5EED_0153).original();
        let task = original.downcast::<PlacementTask>().expect("a placement");
        let givens = original.givens();
        let pieces = task.plan.pieces.len();
        // The plan: the ground, each footprint and its front view shaded and outlined, the
        // baseline, the unit bar and its ticks (a turned box's nearest corner's upright too).
        let shaded = count(&givens, In::Original, |f| matches!(f, Figure::Shade(_)));
        assert_eq!(shaded, 2 * pieces);
        let turned = usize::from(placement == Placement::Turned);
        let [printed, _] = lines(&givens, In::Original, LinePen::Printed);
        assert_eq!(printed, 1 + 2 * pieces + 1 + 3 + turned);
        let up = placement.upright_given();
        let dots = count(&givens, In::Original, |f| matches!(f, Figure::Given { .. }));
        assert_eq!(dots, if up { 1 } else { 2 });
        // The copy: the horizon, the ground's edges, its near corner; the upright or the unit.
        let horizon = givens.iter().find_map(|mark| match mark.drawn {
            Figure::Horizon(height) => Some(height),
            _ => None,
        });
        assert_eq!(horizon, Some(task.horizon));
        let [edges, _] = lines(&givens, In::Copy, LinePen::Printed);
        assert_eq!(edges, if up { 5 } else { 4 }, "{placement:?}");
        let dots = count(&givens, In::Copy, |f| matches!(f, Figure::Given { .. }));
        assert_eq!(dots, if up { 1 } else { 2 });
        assert!(original.painted(None).is_none(), "no picture");
        let answer = original.answer();
        let curves = task.answer.outlines.iter().flatten().filter(|s| !s.hidden);
        assert_eq!(answer.len(), task.answer.sides.len() + curves.count());
    }
}

#[test]
fn each_level_lays_one_placement_of_its_variants_in_turn() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let skill = Skill::of_key("placement").expect("the skill");
    assert_eq!(skill.window(), 3);
    let laid = |level| -> Vec<Kind> {
        let planned = schedule::lesson(skill, level, 7, &standing).expect("a lesson");
        planned.exercises.iter().map(|(e, _)| e.kind).collect()
    };
    let one = |k: usize| vec![Kind::Placement(Placement::ALL[k])];
    assert_eq!(laid(SkillLevel::First), one(0));
    assert_eq!(laid(SkillLevel::Second), one(2));
    assert_eq!(laid(SkillLevel::Third), one(3));
    // Unit 7 teaches it, needing no pack.
    let unit = openpaint_practice::Unit::teaching(skill).expect("its unit");
    assert_eq!((unit.number, skill.lacks(&pictures)), (7, None));
}
