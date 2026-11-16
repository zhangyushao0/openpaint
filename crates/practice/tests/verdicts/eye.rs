//! What only the eye judges says `Eye` whatever was drawn: lines, groups, single shapes (the
//! overlay is the feedback), free boxes (no answer), boxes, ellipses, solids, objects and
//! structure rotation (their exact answer laid, the copy never read: S145, D123), divisions, whole
//! copies and long colour copies (no answer), and a painted kind whose areas or masses no pack
//! holds now. Value masses and key shifts read with their answers: `painted.rs`.

use openpaint_practice::{
    Arrangement, Asked, BoxView, Carve, Combination, Compare, Copied, DrawnCopy, Estimate,
    Exercise, Guide, Horizon, ItemRef, Kind, Lean, Level, LineTier, PictureFit, Point, Rgb8,
    Rotation, ScaleTone, SheetPicture, Solid, Tendency, Tier, Trick, Verdict,
};

use crate::{Laid, compare, shown, steady};

/// A pack's picture of `size` with nothing beside it: no outline, masses or areas.
fn bare_picture(size: [u32; 2]) -> SheetPicture {
    SheetPicture {
        item: ItemRef::default(),
        size,
        fit: PictureFit::of(size),
        outline: None,
        credit: None,
        masses: None,
        areas: None,
    }
}

#[test]
fn what_only_the_eye_judges_says_eye_however_it_was_drawn() {
    let picture = bare_picture([400, 300]);
    let mut kinds: Vec<(Kind, Option<&SheetPicture>)> = vec![
        (Kind::Line(LineTier::ALL[1]), None),
        (Kind::Combined(Combination::ALL[2]), None),
        (Kind::FreeBoxes(Horizon::ALL[0]), None),
        (Kind::Study, Some(&picture)),
    ];
    // The drawn kinds whose exact answer Compare lays for the eye (S145).
    kinds.extend(BoxView::ALL.map(|view| (Kind::Box(view), None)));
    kinds.extend(Guide::ALL.map(|guide| (Kind::Ellipse(guide), None)));
    kinds.extend(Solid::ALL.map(|solid| (Kind::Solid(solid), None)));
    kinds.extend(Carve::ALL.map(|carve| (Kind::Object(carve), None)));
    kinds.push((Kind::Rotation(Rotation::ALL[0]), None));
    for tier in [Tier::ALL[0], Tier::ALL[3]] {
        kinds.extend([
            (Kind::Shape(tier), Some(&picture)),
            (Kind::Division(tier), Some(&picture)),
            (Kind::WholeCopy(tier), Some(&picture)),
            // Painted kinds whose picture holds no masses or areas: laid over the copy.
            (Kind::ValueMasses(tier), Some(&picture)),
            (Kind::Crop(tier), Some(&picture)),
        ]);
    }
    let random = &mut openpaint_practice::Random::new(1);
    let tendency = Tendency::chosen(Lean::ALL[0], None, openpaint_practice::AWAY, random);
    kinds.push((Kind::KeyShift(tendency), Some(&picture)));
    // Lines across the frame and a box's corner, as anyone might draw.
    let drawn = steady(
        vec![
            vec![[0.2, 0.3], [0.8, 0.35]],
            vec![[0.5, 0.2], [0.45, 0.8]],
            vec![[0.3, 0.7], [0.6, 0.6], [0.7, 0.4]],
        ],
        9,
    );
    for (kind, picture) in kinds {
        let exercise = Exercise {
            kind,
            seed: 5,
            picture: picture.map(|p| p.item.clone()),
        };
        let laid = Laid {
            lines: drawn.clone(),
            picks: vec![[120, 90, 60]],
            ..Laid::default()
        };
        let compared = compare(&exercise, picture, &laid).map(shown);
        let compared = compared.unwrap_or_else(|| panic!("{kind:?}: compared"));
        assert_eq!(compared.verdict(), Verdict::Eye, "{kind:?}");
        // A drawn copy laid over by its answer: nothing read, nothing kept (codes 47, 48 no more).
        let laid_over = matches!(
            kind,
            Kind::Box(_) | Kind::Ellipse(_) | Kind::Solid(_) | Kind::Object(_) | Kind::Rotation(_)
        );
        assert!(!laid_over || compared.metrics().is_empty(), "{kind:?}");
    }
}

/// A copy that holds something but must never be read.
struct NeverRead;

impl Copied for NeverRead {
    fn holds(&self) -> bool {
        true
    }

    fn picks(&self) -> Vec<Rgb8> {
        panic!("a drawn copy judged by eye is never read")
    }

    fn drawn(&self) -> DrawnCopy {
        panic!("a drawn copy judged by eye is never read")
    }

    fn division(&self) -> Vec<(Level, Vec<Point>)> {
        panic!("a drawn copy judged by eye is never read")
    }
}

/// S145: boxes (drawn through too), ellipses, solids and objects lay their answer without reading
/// a stroke of the copy, at every variant.
#[test]
fn the_drawn_kinds_laid_over_by_their_answer_never_read_the_copy() {
    let mut kinds: Vec<Kind> = BoxView::ALL.map(Kind::Box).to_vec();
    kinds.extend(Guide::ALL.map(Kind::Ellipse));
    kinds.extend(Solid::ALL.map(Kind::Solid));
    kinds.extend(Carve::ALL.map(Kind::Object));
    kinds.extend(Horizon::ALL.map(Kind::FreeBoxes));
    for kind in kinds {
        for seed in 0..5 {
            let exercise = Exercise::generated(kind, seed);
            let original = exercise.original();
            let asked = Asked {
                exercise: &exercise,
                original: &*original,
                picture: None,
                copy: &NeverRead,
                choices: openpaint_practice::Choices::default(),
            };
            let Some(Compare::Checked(shown)) = original.compare(&asked) else {
                panic!("{kind:?}: laid as geometry")
            };
            assert_eq!(shown.verdict(), Verdict::Eye, "{kind:?}");
        }
    }
}

#[test]
fn a_kind_the_app_judges_never_says_eye() {
    // Whatever is drawn, an estimate's, a point in space's, a scale's or a combination's
    // comparison is the app's or incomplete: each reads a direct measure.
    let kinds = [
        Kind::Flat(Estimate::ALL[0]),
        Kind::Flat(Estimate::ALL[2]),
        Kind::Space(Trick::ALL[0]),
        Kind::Scale(ScaleTone::Grey, 5),
        Kind::Areas(Arrangement::ALL[0]),
    ];
    let laid = Laid {
        lines: steady(vec![vec![[0.2, 0.3], [0.8, 0.35]]], 3),
        ..Laid::default()
    };
    for kind in kinds {
        let exercise = Exercise::generated(kind, 2);
        let compared = compare(&exercise, None, &laid).map(shown);
        let verdict = compared.unwrap_or_else(|| panic!("{kind:?}")).verdict();
        assert_ne!(verdict, Verdict::Eye, "{kind:?}");
    }
}
