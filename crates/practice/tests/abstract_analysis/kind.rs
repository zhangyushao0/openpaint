//! The kind: its facts, its lesson, its card, its choices and its Compare.

use openpaint_practice::{
    AnalysisCompared, AnalysisReading, Arg, Asked, CONTRASTS, CardItem, Choices, Compare,
    ContrastView, Contrasts, Copied, DISTRIBUTIONS, DrawnCopy, Exercise, Figure, ItemRef, Kind,
    Learner, Level, Metric, Pack, PackItem, PictureFit, PictureKind, Pictures, Point, Rgb8, Seen,
    SheetPicture, Skill, SkillLevel, Source, Standing, Tier, VIEWS, Verdict, schedule,
};

/// A copy holding these marks, by their pens' levels.
struct Marked(Vec<(Level, Vec<Point>)>);

impl Copied for Marked {
    fn holds(&self) -> bool {
        !self.0.is_empty()
    }

    fn picks(&self) -> Vec<Rgb8> {
        Vec::new()
    }

    fn drawn(&self) -> DrawnCopy {
        DrawnCopy::default()
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        self.0.clone()
    }
}

fn ring(level: Level) -> (Level, Vec<Point>) {
    (
        level,
        vec![[0.4, 0.4], [0.6, 0.4], [0.6, 0.6], [0.4, 0.6], [0.4, 0.4]],
    )
}

fn exercise() -> Exercise {
    Exercise {
        picture: Some(ItemRef {
            pack: "test-compositions".to_owned(),
            item: "harbour".to_owned(),
        }),
        ..Exercise::generated(Kind::Analysis(Tier::Simple), 0x5EED_0112)
    }
}

/// `exercise`'s marks `copy` asked to be compared with `choices`, its picture 400 by 300.
fn compared(copy: &Marked, choices: Choices) -> Option<Compare> {
    let exercise = exercise();
    let original = exercise.original();
    let picture = SheetPicture {
        item: exercise.picture.clone().expect("its picture"),
        size: [400, 300],
        fit: PictureFit::of([400, 300]),
        outline: None,
        credit: None,
        masses: None,
        areas: None,
    };
    original.compare(&Asked {
        exercise: &exercise,
        original: &*original,
        picture: Some(&picture),
        copy,
        choices,
    })
}

fn seen(view: usize) -> Seen<'static> {
    Seen {
        construction: false,
        shown: 1.0,
        chosen: 0,
        outline: &[],
        grey: false,
        view,
        fit: None,
        probe: None,
    }
}

/// A picture of a dark block on a light ground, `size`.
fn block(size: [u32; 2]) -> Vec<u8> {
    (0..size[0] * size[1])
        .flat_map(|at| {
            let (x, y) = (at % size[0], at / size[0]);
            let inside = (size[0] / 3..2 * size[0] / 3).contains(&x) && y > size[1] / 3;
            let grey = if inside { 50 } else { 200 };
            [grey, grey, grey, 255]
        })
        .collect()
}

#[test]
fn its_picture_lies_under_marks_drawn_with_the_levels_pens_named_as_marks() {
    let facts = Kind::Analysis(Tier::Simple).facts();
    assert_eq!(facts.key, "abstract-analysis");
    assert!(facts.under && facts.constructed && facts.compared);
    assert!(facts.tools.marks && facts.tools.levels() && facts.tools.history);
    assert!(!facts.tools.eyedropper && !facts.picks.original);
    let original = exercise().original();
    assert_eq!(original.givens()[0].drawn, Figure::Pictured, "its credit");
    let task = original.task();
    assert!(task.contains(&CardItem::Toggles(&CONTRASTS)));
    assert!(task.contains(&CardItem::Shapes(&DISTRIBUTIONS)));
    assert_eq!(VIEWS.len(), ContrastView::ALL.len());
}

#[test]
fn a_lesson_is_one_composition_of_the_level_s_tier() {
    let item = |id: &str, tier: Tier| PackItem {
        id: id.to_owned(),
        kind: PictureKind::Composition,
        tier,
        size: [800, 600],
        title: "Test".to_owned(),
        credit: None,
        outline: None,
        masses: None,
        areas: None,
    };
    let tiers = [Tier::Simple, Tier::Medium, Tier::Hard];
    let pack = Pack {
        id: "test-compositions".to_owned(),
        title: "Test".to_owned(),
        items: tiers.iter().map(|tier| item(tier.name(), *tier)).collect(),
    };
    let (pictures, learner) = (Pictures::of(&[pack]), Learner::default());
    let standing = Standing {
        pictures: &pictures,
        learner: &learner,
    };
    let skill = Skill::of(Kind::Analysis(Tier::Simple)).expect("a skill");
    assert_eq!(skill.window(), 3);
    for (level, tier) in SkillLevel::ALL.into_iter().zip(tiers) {
        let lesson = schedule::lesson(skill, level, 7, &standing).expect("a lesson");
        let kinds: Vec<Kind> = lesson.exercises.iter().map(|(e, _)| e.kind).collect();
        assert_eq!(kinds, [Kind::Analysis(tier)], "{level:?}");
    }
}

#[test]
fn choices_keep_their_chips_and_shape_as_two_metrics() {
    let choices = Choices::default().toggled(2).toggled(3).toggled(5);
    let choices = Choices {
        shape: Some(3),
        ..choices
    };
    assert!(choices.on(2) && choices.on(5) && !choices.on(0));
    let metrics = choices.metrics();
    assert_eq!(
        (Metric::ChosenChips.code(), Metric::ChosenShape.code()),
        (59, 60)
    );
    assert_eq!(Choices::kept(&metrics), Some(choices));
    assert_eq!(Choices::kept(&[]), None);
    assert!(!choices.toggled(2).on(2), "a second tap turns it off");
}

#[test]
fn compare_reads_the_picture_into_five_views_the_marks_over_them() {
    assert!(compared(&Marked(Vec::new()), Choices::default()).is_none());
    let choices = Choices {
        chips: 0b101,
        shape: Some(3),
    };
    let second = Marked(vec![ring(Level::Second)]);
    let Some(Compare::Read(reading)) = compared(&second, choices) else {
        panic!("read on the worker")
    };
    assert_eq!(reading.unread().verdict(), Verdict::Incomplete, "no 强 yet");
    let marks = Marked(vec![ring(Level::Main), ring(Level::Third)]);
    let Some(Compare::Read(reading)) = compared(&marks, choices) else {
        panic!("read on the worker")
    };
    let unread = reading.unread();
    assert_eq!(unread.verdict(), Verdict::Eye);
    assert_eq!(unread.card(&seen(0))[0], CardItem::Reading);
    assert_eq!(unread.metrics(), choices.metrics());
    let as_read = reading.downcast::<AnalysisReading>().expect("its reading");
    assert_eq!(as_read.lines.len(), 2);
    let size = [240, 180];
    let picture = block(size);
    let read = reading.read([10, 10], &[255; 400], [255; 3], Some((size, &picture)));
    let contrasts = Contrasts::of(size, &picture);
    let expected: Vec<Vec<u8>> = (ContrastView::ALL.iter())
        .map(|view| contrasts.picture(*view))
        .collect();
    let pictures: Vec<Vec<u8>> = read.pictures.iter().map(|(_, rgba)| rgba.clone()).collect();
    assert_eq!(pictures, expected);
    assert!(read.pictures.iter().all(|(at, _)| *at == contrasts.size));
    assert_eq!(read.metrics, choices.metrics());
    let shown = read.comparison;
    let marks = shown.marks(&seen(3));
    assert!(matches!(
        marks[0].drawn,
        Figure::Picture {
            source: Source::Read(3),
            ..
        }
    ));
    assert_eq!(marks.len(), 3, "the view, then the two marks");
    let card = shown.card(&seen(0));
    assert!(card.contains(&CardItem::Views(&VIEWS)));
    let chosen = card.iter().find_map(|item| match item {
        CardItem::Says(say) if say.key == "analysis-chosen" => Some(say.args.clone()),
        _ => None,
    });
    let words = [CONTRASTS[0], CONTRASTS[2]].map(Arg::Word).to_vec();
    assert_eq!(chosen, Some(words));
}

#[test]
fn without_its_picture_read_compare_lays_no_view() {
    let marks = Marked(vec![ring(Level::Main)]);
    let Some(Compare::Read(reading)) = compared(&marks, Choices::default()) else {
        panic!("read on the worker")
    };
    let read = reading.read([10, 10], &[255; 400], [255; 3], None);
    assert!(read.pictures.is_empty());
    let compared = read
        .comparison
        .downcast::<AnalysisCompared>()
        .expect("its own");
    assert_eq!(compared.read, Some(false));
    assert!(
        !read
            .comparison
            .card(&seen(0))
            .contains(&CardItem::Views(&VIEWS))
    );
    assert_eq!(read.comparison.marks(&seen(0)).len(), 1, "the mark alone");
}
