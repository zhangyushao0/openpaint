//! Form intersections (S156k): both forms printed in the one frame as the level gives them;
//! Compare laying the exact line where they meet for the eye without reading the copy, the way to
//! find it to lay with it; its lessons; its How. The line's exactness is the folder's own tests'.

use openpaint_practice::{
    Asked, CELLS, CardItem, Choices, Compare, Copied, DemoLayout, DrawnCopy, Exercise, Figure, In,
    IntersectionTask, Kind, Learner, Level, LinePen, Marker, Meeting, Pictures, Point, Rgb8, Seen,
    SheetMark, Skill, SkillLevel, Standing, UNITS, Verdict, schedule,
};

/// A copy that holds something but must never be read.
struct Unread;

impl Copied for Unread {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        panic!("a form intersection's copy is never read")
    }

    fn drawn(&self) -> DrawnCopy {
        panic!("a form intersection's copy is never read")
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        panic!("a form intersection's copy is never read")
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

/// The lines among `marks` in `pen`: whole and dashed.
fn lines(marks: &[SheetMark], pen: LinePen) -> [usize; 2] {
    let of = |dashed: bool| {
        (marks.iter())
            .filter(|mark| {
                matches!(&mark.drawn, Figure::Line { pen: p, dashed: d, .. } if *p == pen && *d == dashed)
            })
            .count()
    };
    [of(false), of(true)]
}

#[test]
fn compare_lays_the_meeting_line_for_the_eye_and_never_reads_the_copy() {
    for meeting in Meeting::ALL {
        let exercise = Exercise::generated(Kind::Intersection(meeting), 0x5EED_0156);
        let original = exercise.original();
        let task = original
            .downcast::<IntersectionTask>()
            .expect("an intersection");
        let asked = Asked {
            exercise: &exercise,
            original: &*original,
            picture: None,
            copy: &Unread,
            choices: Choices::default(),
        };
        let Some(Compare::Checked(shown)) = original.compare(&asked) else {
            panic!("{meeting:?}: checked, a construction to lay");
        };
        assert_eq!(shown.verdict(), Verdict::Eye, "the learner judges");
        assert!(shown.metrics().is_empty(), "nothing kept");
        assert_eq!(shown.card(&seen(false)), [CardItem::Construction]);
        let marks = shown.marks(&seen(false));
        let count = |hidden: bool| task.answer.iter().filter(|s| s.hidden == hidden).count();
        assert_eq!(lines(&marks, LinePen::Answer), [count(false), count(true)]);
        assert!(marks.iter().all(|mark| mark.on == In::Copy));
        assert!(!marks.iter().any(|m| matches!(m.drawn, Figure::Ring(_))));
        assert_eq!(lines(&marks, LinePen::Pencil), [0, 0]);
        // 看画法: the lines carried along faces and sides, the sections, the points found.
        let constructed = shown.marks(&seen(true));
        let way = &task.found;
        let [whole, dashed] = lines(&constructed, LinePen::Pencil);
        assert_eq!(whole, way.rings.len(), "{meeting:?}");
        assert_eq!(
            dashed,
            way.lines.len() + 3 * way.squares.len(),
            "{meeting:?}"
        );
        let ringed = (constructed.iter()).filter(|m| matches!(m.drawn, Figure::Found(_)));
        assert_eq!(ringed.count(), task.pierced.len());
        assert!(dashed + whole > 0, "{meeting:?}: a way to find it");
    }
}

#[test]
fn the_givens_are_the_horizon_and_both_forms_drawn_through_or_as_seen_by_level() {
    for meeting in Meeting::ALL {
        let original = Exercise::generated(Kind::Intersection(meeting), 0x5EED_0156).original();
        let task = original
            .downcast::<IntersectionTask>()
            .expect("an intersection");
        let givens = original.givens();
        assert!(givens.iter().all(|mark| mark.on == In::Copy), "one frame");
        let horizon = givens.iter().find_map(|mark| match mark.drawn {
            Figure::Horizon(height) => Some(height),
            _ => None,
        });
        assert_eq!(horizon, Some(task.seen.horizon));
        let count = |hidden: bool| task.printed.iter().filter(|s| s.hidden == hidden).count();
        assert_eq!(lines(&givens, LinePen::Printed), [count(false), 0]);
        assert_eq!(lines(&givens, LinePen::PrintedHalf), [0, count(true)]);
        assert_eq!(count(true) > 0, meeting.through(), "{meeting:?}");
        assert!(original.painted(None).is_none(), "no picture");
        let facts = Kind::Intersection(meeting).facts();
        assert!(facts.compared && facts.constructed && !facts.aligned);
    }
}

#[test]
fn each_level_lays_two_pairs_of_its_forms_in_unit_eight_before_contrast_thumbnails() {
    let (pictures, learner) = (Pictures::default(), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let skill = Skill::of_key("intersections").expect("the skill");
    let contours = Skill::of_key("contours").expect("contours");
    let thumbnails = Skill::of_key("contrast-thumbnail").expect("thumbnails");
    let at = |skill: Skill| UNITS[7].skills.iter().position(|held| *held == skill);
    assert!(at(contours) < at(skill) && at(skill) < at(thumbnails));
    let pairs = [
        [Meeting::Edge, Meeting::Corner],
        [Meeting::CylinderIntoBox, Meeting::BoxIntoCylinder],
        [Meeting::Cylinders, Meeting::Sphere],
    ];
    for (level, pair) in SkillLevel::ALL.into_iter().zip(pairs) {
        let planned = schedule::lesson(skill, level, 7, &standing).expect("a lesson");
        let mut kinds: Vec<Kind> = planned.exercises.iter().map(|(e, _)| e.kind).collect();
        kinds.sort_by_key(|kind| kind.code());
        assert_eq!(kinds, pair.map(Kind::Intersection), "{level:?}");
    }
}

#[test]
fn an_intersection_is_stored_by_its_variant() {
    for (place, meeting) in Meeting::ALL.into_iter().enumerate() {
        let kind = Kind::Intersection(meeting);
        assert_eq!(kind.code(), (35, place as u8));
        assert_eq!(Kind::of_code(35, place as u8), Some(kind));
        let task = IntersectionTask::generate(meeting, 9);
        let exercise = Exercise::generated(kind, 9);
        assert!(exercise.original().downcast::<IntersectionTask>() == Some(&task));
    }
}

#[test]
fn the_how_carries_the_edges_into_the_faces_finds_the_pierce_points_and_joins_them() {
    let demo = Kind::Intersection(Meeting::Edge).how(CELLS).expect("a How");
    assert_eq!(demo.layout, DemoLayout::Single);
    assert_eq!(demo.steps.len(), 4);
    let drawn = |pen: Marker| {
        demo.marks
            .iter()
            .filter(move |m| m.pen == pen && m.to > 0.0)
    };
    assert!(
        drawn(Marker::Construction).count() > 4,
        "edges carried, points found"
    );
    assert!(drawn(Marker::Ink).count() > 0, "the line joined in ink");
    assert!(drawn(Marker::Answer).count() > 0, "Compare's red");
    assert!(demo.view.is_some(), "framed on the forms");
}
