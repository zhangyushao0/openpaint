//! Cold-warm copies (S154): a project whose pair of tendencies turns from copy to copy, a warm and
//! a cool about opposite, kept in its variant; its picture laid in its frame and its two swatches
//! beside the copy's, room left for them on its sheet; compared as a long copy with a Grey view and
//! a disc, no result and no number.

use openpaint_practice::{
    Asked, BESIDE_NAMES, CardItem, Choices, Compare, Compared, Copied, DrawnCopy, Figure, In,
    ItemRef, Kind, Layout, Leans, Level, PictureFit, Place, Point, Purpose, Reading, Rgb8,
    STUDY_SIDE, Seen, Sheet, SheetPicture, ShiftCompared, Source, Verdict, number, oklab, oklch,
};

fn picture() -> ItemRef {
    ItemRef {
        pack: "test-shapes".to_owned(),
        item: "vase".to_owned(),
    }
}

/// How far apart two hues lie round the circle, degrees.
fn apart(a: f64, b: f64) -> f64 {
    let turn = (a - b).rem_euclid(360.0);
    turn.min(360.0 - turn)
}

#[test]
fn the_pairs_turn_from_copy_to_copy_a_warm_and_a_cool_about_opposite_kept_in_the_variant() {
    let warms: Vec<u16> = (0..13).map(|n| Leans::nth(n).warm).collect();
    let mut seen = warms.clone();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 13, "every warm hue before any again: {warms:?}");
    assert_ne!(Leans::nth(0), Leans::nth(1));
    for n in 0..40 {
        let leans = Leans::nth(n);
        let opposite = apart(f64::from(leans.warm), f64::from(leans.cool));
        assert!(opposite >= 130.0, "{leans:?}: {opposite}");
        let [lights, shadows] = [leans.lights(), leans.shadows()].map(oklch);
        assert!(lights[0] > shadows[0] + 0.2, "the lights' lighter");
        assert!(apart(lights[2], f64::from(leans.warm)) < 15.0, "{leans:?}");
        assert!(apart(shadows[2], f64::from(leans.cool)) < 15.0, "{leans:?}");
        let kind = Kind::ColdWarm(leans);
        let (code, variant) = kind.code();
        assert_eq!((code, Kind::of_code(code, variant)), (32, Some(kind)));
    }
    let pairs = (0..=u8::MAX).filter(|code| Kind::of_code(32, *code).is_some());
    assert_eq!(pairs.count(), 13 * 15);
}

#[test]
fn a_cold_warm_copy_is_a_project_numbered_among_its_own_with_room_beside_its_frame() {
    let first = Sheet::cold_warm(1, (5, 100), picture(), &[]);
    let copy = Sheet::copy(2, (6, 200), picture());
    let second = Sheet::cold_warm(3, (7, 300), picture(), &[first.clone(), copy.clone()]);
    for (sheet, n) in [(&first, 0), (&second, 1)] {
        assert_eq!(sheet.purpose, Purpose::Copy);
        let kind = sheet.laid(0).map(|laid| laid.exercise.kind);
        assert_eq!(
            kind,
            Some(Kind::ColdWarm(Leans::nth(n))),
            "the pair after the last"
        );
        assert!(
            sheet.lesson().and_then(|lesson| lesson.skill()).is_none(),
            "no lesson"
        );
    }
    let all = [first.clone(), copy.clone(), second.clone()];
    let numbers = [&first, &copy, &second].map(|sheet| number(&all, sheet, 0));
    assert_eq!(numbers, [1, 1, 2], "numbered among their own sort");
    let framed = |sheet: &Sheet| {
        let mut sheet = sheet.clone();
        sheet.lessons[0].exercises[0].frame = [800, 600];
        Layout::of_sheet(&sheet)
    };
    let (shifted, plain) = (framed(&first), framed(&copy));
    assert_eq!(
        shifted.pairs, plain.pairs,
        "the same frames as a colour copy's"
    );
    let room = f64::from(shifted.size[0] - plain.size[0]);
    assert!((room - 0.12 * STUDY_SIDE).abs() < 1.0, "{room}");
}

/// A copy with paint in its frame.
struct Held;

impl Copied for Held {
    fn holds(&self) -> bool {
        true
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

fn seen(grey: bool, probe: Option<(Rgb8, Rgb8)>) -> Seen<'static> {
    Seen {
        construction: false,
        shown: 0.4,
        chosen: 0,
        outline: &[],
        grey,
        view: 0,
        fit: None,
        probe,
    }
}

#[test]
fn its_picture_is_laid_in_its_frame_and_its_two_swatches_beside_the_copy_s() {
    let sheet = Sheet::cold_warm(1, (5, 100), picture(), &[]);
    let leans = Leans::nth(0);
    let original = sheet.laid(0).expect("its exercise").exercise.original();
    let givens = original.givens();
    assert_eq!(
        givens[0].drawn,
        Figure::Pictured,
        "the picture in its frame"
    );
    let swatches: Vec<(Rgb8, Option<&str>)> = (givens.iter())
        .filter_map(|mark| match mark.drawn {
            Figure::Beside { colour, name, .. } if mark.on == In::Copy => Some((colour, name)),
            _ => None,
        })
        .collect();
    let named = [
        (leans.lights(), Some("lights")),
        (leans.shadows(), Some("shadows")),
    ];
    assert_eq!(swatches, named);
    assert!(BESIDE_NAMES.contains(&"lights") && BESIDE_NAMES.contains(&"shadows"));
}

/// The first cold-warm copy compared, its picture 800 by 600 held: how it is read, and the
/// picture's place.
fn compared() -> (Reading, [Point; 4]) {
    let sheet = Sheet::cold_warm(1, (5, 100), picture(), &[]);
    let exercise = sheet.laid(0).expect("its exercise").exercise.clone();
    let original = exercise.original();
    let fit = PictureFit::of([800, 600]);
    let laid = SheetPicture {
        item: picture(),
        size: [800, 600],
        fit,
        outline: None,
        credit: None,
        masses: None,
        areas: None,
    };
    let asked = Asked {
        exercise: &exercise,
        original: &*original,
        picture: Some(&laid),
        copy: &Held,
        choices: Choices::default(),
    };
    let Some(Compare::Read(reading)) = original.compare(&asked) else {
        panic!("read for its Grey view");
    };
    (reading, fit.corners())
}

#[test]
fn it_is_compared_with_its_picture_laid_a_grey_view_and_a_disc_and_no_result() {
    let (reading, corners) = compared();
    assert!(reading.greys());
    let laid = reading.unread().marks(&seen(false, None));
    let picture_laid = Figure::Picture {
        source: Source::Pack,
        place: Place::Corners(corners),
        shown: 0.4,
    };
    assert_eq!(laid.len(), 1);
    assert_eq!(laid[0].drawn, picture_laid, "laid at the slider's strength");
    let pixels = [200_u8, 120, 60, 255].repeat(16);
    let read = reading.read([4, 4], &pixels, [255; 3], Some(([4, 4], &pixels)));
    assert!(read.metrics.is_empty(), "no number");
    assert_eq!(read.pictures.len(), 2, "both frames in grey");
    let [r, g, b, _] = read.pictures[0].1[..4] else {
        panic!("a pixel");
    };
    assert!(r == g && g == b, "grey");
    let kept = (oklab([r, g, b])[0] - oklab([200, 120, 60])[0]).abs();
    assert!(kept < 0.01, "its lightness kept");
    let compared = read
        .comparison
        .downcast::<ShiftCompared>()
        .expect("compared");
    assert_eq!(compared.verdict(), Verdict::Nobody, "no result");
    let sources: Vec<(In, Source)> = (compared.marks(&seen(true, None)).iter())
        .filter_map(|mark| match mark.drawn {
            Figure::Picture { source, .. } => Some((mark.on, source)),
            _ => None,
        })
        .collect();
    let greys = [
        (In::Original, Source::Read(1)),
        (In::Copy, Source::Read(0)),
        (In::Copy, Source::Read(1)),
    ];
    assert_eq!(sources, greys);
    let (original, copy) = ([180, 150, 120], [200, 140, 90]);
    let card = compared.card(&seen(false, Some((original, copy))));
    assert_eq!(card[..2], [CardItem::Grey, CardItem::Slider]);
    let tendencies = Leans::nth(0).on_disc();
    let leaning = CardItem::Leaning {
        original,
        copy,
        tendencies,
    };
    assert_eq!(card[2..], [leaning, CardItem::Bar { original, copy }]);
}
