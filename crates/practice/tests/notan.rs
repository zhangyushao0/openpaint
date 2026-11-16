//! Notan (S156): value masses' two-value variant. Its lessons: the first level as before S156, the
//! second and the third in two values every second lesson, on the same skill and its pictures; a
//! copy painted with Dark and Light read against the answer's middle merged into the dark and into
//! the light, compared with the merge it matches better (its picture in two greys brought for the
//! slider, its edges red, the share kept), the eye judging; never compared before it is read.

use std::sync::Arc;

use openpaint_practice::{
    CardItem, Compared, Figure, Fill, Kind, Learner, LinePen, Masses, Massing, Metric,
    NotanCompared, NotanReading, Pack, PackItem, Pen, PictureFit, PictureKind, Pictures, Point,
    Purpose, Reads, Seen, Sheet, SheetMark, Skill, SkillLevel, Source, Standing, Tier, Value,
    ValueMasses, Verdict, schedule, value,
};

/// The frame's longer side the copies are painted on, pixels: the sheet's.
const SIDE: u32 = 400;
/// 2026-10-05 12:00 UTC.
const NOON: u64 = 1_791_201_600;

/// The frame's pixels (the picture's shape, its longer side `SIDE`; white paper) of a copy painted
/// in greys, each pixel the grey `paint` gives its picture share, and its size.
fn painted(fit: &PictureFit, paint: impl Fn(Point) -> Option<Value>) -> ([u32; 2], Vec<u8>) {
    let unit = f64::from(SIDE);
    let size = fit.size.map(|side| (side * unit).round() as u32);
    let pixels = (0..size[0] * size[1])
        .flat_map(|at| {
            let (x, y) = (f64::from(at % size[0]), f64::from(at / size[0]));
            let share = [
                (x + 0.5) / unit / fit.size[0],
                (y + 0.5) / unit / fit.size[1],
            ];
            let [r, g, b] = paint(share).map_or([255; 3], Value::grey);
            [r, g, b, 255]
        })
        .collect();
    (size, pixels)
}

/// `answer` with its middle masses merged into `into`.
fn merged(answer: &ValueMasses, into: Value) -> ValueMasses {
    let values = (answer.values.iter())
        .map(|v| if *v == 1 { into.index() } else { *v })
        .collect();
    ValueMasses::checked(answer.size, values).expect("a map")
}

/// What Compare shows: everything at once, the answer half strong.
fn seen() -> Seen<'static> {
    Seen {
        construction: false,
        shown: 0.5,
        chosen: 0,
        outline: &[],
        grey: false,
        view: 0,
        fit: None,
        probe: None,
    }
}

#[test]
fn a_copy_is_compared_with_the_merge_it_matches_better_and_judged_by_eye() {
    let masses = Masses::generate(3, 0.75);
    let (size, pixels) = masses.picture(SIDE);
    let answer = ValueMasses::of(size, &pixels);
    assert!(answer.shares()[1] > 0.05, "the landscape has middle masses");
    let fit = PictureFit::of(size);
    let reading = NotanReading {
        answer: Arc::new(answer.clone()),
        place: [[0.0, 0.0], fit.size],
        corners: fit.corners(),
    };
    // Unread: nothing laid, no verdict yet.
    let unread = reading.unread();
    assert!(unread.marks(&seen()).is_empty());
    assert_eq!(unread.card(&seen()), [CardItem::Slider, CardItem::Reading]);
    assert_eq!(unread.verdict(), Verdict::Incomplete);
    for into in [Value::Dark, Value::Light] {
        matched_whole(&reading, &fit, into);
    }
    // A three-value copy matches neither merge whole: hatched where its middle lies wider than the
    // pen, the eye still judging.
    let (size, copy) = painted(&fit, |share| Some(answer.at(share)));
    let read = reading.read(size, &copy, [255; 3], None);
    let compared = (read.comparison.downcast::<NotanCompared>()).expect("Notan's");
    let check = compared.check.as_ref().expect("read");
    assert!(!check.hatching.is_empty() && check.matched < 0.999);
    assert_eq!(compared.verdict(), Verdict::Eye);
}

/// A copy painted as the answer's middle merged into `into`: compared with that merge, matched
/// whole and kept, its picture brought in two greys and laid under the slider, its mass edges red,
/// the eye judging.
fn matched_whole(reading: &NotanReading, fit: &PictureFit, into: Value) {
    let two = merged(&reading.answer, into);
    let (size, copy) = painted(fit, |share| Some(two.at(share)));
    let read = reading.read(size, &copy, [255; 3], None);
    let compared = (read.comparison.downcast::<NotanCompared>()).expect("Notan's");
    assert_eq!(compared.merged, Some(into), "the merge it matches");
    let check = compared.check.as_ref().expect("read");
    assert!(
        check.hatching.is_empty() && check.matched > 0.999,
        "{into:?}"
    );
    let kept = value(&read.metrics, Metric::MassesMatched);
    assert_eq!(kept, Some(check.matched));
    assert_eq!(compared.verdict(), Verdict::Eye);
    let [(picture_size, picture)] = read.pictures.as_slice() else {
        panic!("one picture");
    };
    assert_eq!(*picture_size, two.size);
    let greys = [Value::Dark.grey(), Value::Light.grey()];
    let in_two = |p: &[u8]| greys.contains(&[p[0], p[1], p[2]]);
    assert!(picture.chunks(4).all(in_two));
    let marks = compared.marks(&seen());
    let laid = |mark: &&SheetMark| {
        matches!(
            mark.drawn,
            Figure::Picture {
                source: Source::Read(0),
                ..
            }
        )
    };
    assert_eq!(marks.iter().filter(laid).count(), 1);
    let edged = |mark: &&SheetMark| {
        matches!(
            mark.drawn,
            Figure::Line {
                pen: LinePen::Laid,
                ..
            }
        )
    };
    assert!(marks.iter().any(|mark| edged(&mark)), "its mass edges");
    assert_eq!(compared.card(&seen()), [CardItem::Slider]);
}

/// Compositions of the three tiers they come in, four each, whose masses read.
fn compositions() -> Pictures {
    let masses = ValueMasses::checked([2, 2], vec![0, 1, 2, 1]).map(Arc::new);
    let item = |n: usize, tier: Tier| PackItem {
        id: format!("{}-{n}", tier.name()),
        kind: PictureKind::Composition,
        tier,
        size: [800, 600],
        title: "test".to_owned(),
        credit: None,
        outline: None,
        masses: masses.clone(),
        areas: None,
    };
    let tiers = [Tier::Simple, Tier::Medium, Tier::Hard];
    Pictures::of(&[Pack {
        id: "test-compositions".to_owned(),
        title: "Test".to_owned(),
        items: (tiers.iter())
            .flat_map(|tier| (0..4).map(move |n| item(n, *tier)))
            .collect(),
    }])
}

#[test]
fn notan_is_laid_at_the_upper_levels_every_second_lesson_on_value_masses_pictures() {
    let pictures = compositions();
    let skill = Skill::of_key("value-masses").expect("value masses");
    let laid = |learner: &Learner, level| -> Vec<Kind> {
        let standing = Standing {
            pictures: &pictures,
            learner,
        };
        let planned = schedule::lesson(skill, level, 7, &standing).expect("a lesson");
        for (exercise, _) in &planned.exercises {
            assert_eq!(Skill::of(exercise.kind), Some(skill), "the same skill");
            assert!(
                exercise.picture.is_some(),
                "a composition whose masses read"
            );
        }
        planned.exercises.iter().map(|(e, _)| e.kind).collect()
    };
    let mut learner = Learner::default();
    let mut kinds = Vec::new();
    for _ in 0..4 {
        kinds.push(SkillLevel::ALL.map(|level| laid(&learner, level)));
        let standing = Standing {
            pictures: &pictures,
            learner: &learner,
        };
        let planned = schedule::lesson(skill, SkillLevel::First, 7, &standing).expect("a lesson");
        let sheet = Sheet::begun(1, (Purpose::Day, NOON), planned);
        learner = learner.with(&sheet.lessons[0]);
    }
    let two = |kinds: &[Kind]| match kinds {
        [Kind::ValueMasses(massing)] => massing.two(),
        other => panic!("{other:?}"),
    };
    for (turn, [first, second, third]) in kinds.iter().enumerate() {
        assert!(!two(first), "{turn}: 入门 as before");
        let even = turn % 2 == 0;
        assert_eq!((two(second), two(third)), (even, even), "{turn}");
    }
    assert_eq!(kinds[0][1], [Kind::ValueMasses(Massing::Two(Tier::Medium))]);
    assert_eq!(kinds[0][2], [Kind::ValueMasses(Massing::Two(Tier::Hard))]);
    assert_eq!(
        kinds[1][1],
        [Kind::ValueMasses(Massing::Three(Tier::Medium))]
    );
}

#[test]
fn notan_is_painted_with_dark_and_light_and_fills_its_frame() {
    let notan = Kind::ValueMasses(Massing::Two(Tier::Medium)).facts();
    assert_eq!(notan.key, "notan");
    assert_eq!(notan.tools.pens, &Pen::TWO_TONES[..]);
    assert_eq!(notan.tools.fill, Some(Fill::Frame));
    assert_eq!(
        notan.tools.lightest(),
        Pen::Light,
        "Fill frame lays the light first"
    );
    let three = Kind::ValueMasses(Massing::Three(Tier::Medium)).facts();
    assert_eq!(
        (three.key, three.tools.pens),
        ("value-masses", &Pen::VALUES[..])
    );
    for massing in Massing::ALL {
        let tier = massing.tier();
        let code = if massing.two() {
            4 + tier.code()
        } else {
            tier.code()
        };
        assert_eq!(Kind::ValueMasses(massing).code(), (17, code), "{massing:?}");
    }
}
