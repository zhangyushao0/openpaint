//! What only the eye judges says `Eye` whatever was drawn: lines, groups, single shapes (the
//! overlay is the feedback), free boxes (their rules, no answer), divisions, whole copies and long
//! colour copies (no answer), and a painted kind whose areas or masses no pack holds now.

use openpaint_practice::{
    Arrangement, BoxView, Combination, Exercise, Horizon, ItemRef, Kind, Lean, LineTier,
    PictureFit, ScaleTone, SheetPicture, Tendency, Tier, Verdict,
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
    }
}

#[test]
fn a_kind_the_app_judges_never_says_eye() {
    // Whatever is drawn, a box, an ellipse, an estimate's, a scale's or a combination's comparison
    // is the app's or incomplete.
    let kinds = [
        Kind::Box(BoxView::ALL[0]),
        Kind::Box(BoxView::ALL[1]),
        Kind::Ellipse(openpaint_practice::Guide::ALL[0]),
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
