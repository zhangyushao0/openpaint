//! Value masses' hatching (S96): a copy painted in the three greys read against its answer is
//! hatched in the answer's grey only where it differs over more than the grey pen's width
//! (`VALUE_PEN`, the pens the app paints value masses with): never at the seams strokes leave
//! between them or along the picture's edges, nor where a copy is shifted by under a pen; where a
//! half is left bare or a corner painted dark, there and in the answer's values.

use openpaint_practice::{
    Masses, MassesCheck, Metric, PictureFit, Point, VALUE_PEN, Value, ValueMasses, value,
};

/// The frame's side the copies are painted on, pixels: the sheet's.
const SIDE: u32 = 400;

/// The frame's pixels (`side` square, white paper) of a copy painted in greys: inside the
/// picture's place in the frame, each pixel the grey `paint` gives its picture share.
fn painted(side: u32, fit: &PictureFit, paint: impl Fn(Point) -> Option<Value>) -> Vec<u8> {
    let unit = f64::from(side);
    (0..side * side)
        .flat_map(|at| {
            let (x, y) = (f64::from(at % side), f64::from(at / side));
            let p = [(x + 0.5) / unit, (y + 0.5) / unit];
            let share = [
                (p[0] - fit.origin[0]) / fit.size[0],
                (p[1] - fit.origin[1]) / fit.size[1],
            ];
            let inside = share.iter().all(|s| (0.0..1.0).contains(s));
            let [r, g, b] = inside
                .then(|| paint(share))
                .flatten()
                .map_or([255; 3], Value::grey);
            [r, g, b, 255]
        })
        .collect()
}

/// The landscape the hatching is tried on: its answer and where its picture lies in the frame.
fn hatching_case() -> (ValueMasses, PictureFit) {
    let masses = Masses::generate(3, 0.75);
    let (size, pixels) = masses.picture(400);
    (ValueMasses::of(size, &pixels), PictureFit::of(size))
}

/// A copy painted as `paint` says, read against `answer`.
fn read(
    answer: &ValueMasses,
    fit: &PictureFit,
    paint: impl Fn(Point) -> Option<Value>,
) -> MassesCheck {
    let copy = painted(SIDE, fit, paint);
    answer.read(([SIDE, SIDE], &copy), [fit.origin, fit.size])
}

/// A hatching stretch's end as a share of the picture.
fn share_of(fit: &PictureFit, [x, y]: Point) -> Point {
    [
        (x - fit.origin[0]) / fit.size[0],
        (y - fit.origin[1]) / fit.size[1],
    ]
}

/// The hatching of `answer`'s copy shifted sideways by `pens` of the grey pen's width, and the
/// shift as a share of the picture.
fn shifted(answer: &ValueMasses, fit: &PictureFit, pens: f64) -> (MassesCheck, f64) {
    let shift = pens * VALUE_PEN / fit.size[0];
    let check = read(answer, fit, |[u, v]| Some(answer.at([u - shift, v])));
    (check, shift)
}

#[test]
fn a_copy_is_hatched_only_where_it_differs_over_more_than_the_grey_pens_width() {
    // Broad masses: a light, a middle and a dark band side by side, the middle one cut by a dark
    // block, each several pens wide.
    let [w, h] = [200, 150];
    let values = (0..w * h)
        .map(|at| {
            let (x, y) = (at % w, at / w);
            match x * 3 / w {
                0 => 2,
                1 if (50..100).contains(&y) => 0,
                1 => 1,
                _ => 0,
            }
        })
        .collect();
    let answer = ValueMasses::checked([w as u32, h as u32], values).expect("a map");
    let fit = PictureFit::of([400, 300]);
    // The copy shifted by just under the pen's width: the strips along its edges are narrower
    // than the pen, nothing hatched.
    let (under, _) = shifted(&answer, &fit, 0.9);
    assert!(under.hatching.is_empty(), "{:?}", under.hatching.first());
    assert!(under.matched < 0.95, "{}", under.matched);
    // By well over it: hatched, only where the copy's value differs from the answer's, in the
    // answer's value.
    let (over, shift) = shifted(&answer, &fit, 2.5);
    assert!(!over.hatching.is_empty());
    for (value, ends) in &over.hatching {
        for end in ends {
            let [u, v] = share_of(&fit, *end);
            assert_ne!(
                answer.at([u - shift, v]),
                answer.at([u, v]),
                "{u:.3}, {v:.3}"
            );
        }
        assert_eq!(*value, answer.at(share_of(&fit, ends[0])));
    }
}

#[test]
fn a_landscape_copied_exactly_or_half_a_pen_off_is_not_hatched() {
    let (answer, fit) = hatching_case();
    // The answer itself: nothing hatched, all of it matched, the share kept.
    let check = read(&answer, &fit, |share| Some(answer.at(share)));
    assert!(check.hatching.is_empty(), "{:?}", check.hatching.first());
    assert!(check.matched > 0.999, "{}", check.matched);
    assert_eq!(
        value(&check.metrics, Metric::MassesMatched),
        Some(check.matched)
    );
    // Half a pen off: nothing. (Nearer a whole pen, the thin middle rim the blur leaves between a
    // light and a dark mass, a cell or two wide, moves wholly off its place and the strip of
    // three values is wider than the pen: hatched there, as the rule says.)
    let (half, _) = shifted(&answer, &fit, 0.5);
    assert!(half.hatching.is_empty(), "{:?}", half.hatching.first());
}

#[test]
fn seams_narrower_than_the_grey_pen_between_strokes_are_not_hatched() {
    let (answer, fit) = hatching_case();
    // The pen's width as a share of the picture's width and height.
    let pen = [VALUE_PEN / fit.size[0], VALUE_PEN / fit.size[1]];
    // Middle filled first, then Light in strokes a pen and a half wide, seams of 0.6 of a pen
    // left between them, and the strokes stopped half a pen short of the picture's left and
    // right edges.
    let seamed = |[u, v]: Point| {
        let value = answer.at([u, v]);
        let seam = (v / pen[1]).rem_euclid(2.1) < 0.6;
        let short = u < 0.5 * pen[0] || u > 1.0 - 0.5 * pen[0];
        let middle = value == Value::Light && (seam || short);
        Some(if middle { Value::Middle } else { value })
    };
    let check = read(&answer, &fit, seamed);
    assert!(check.hatching.is_empty(), "{:?}", check.hatching.first());
    assert!(
        check.matched < 0.98,
        "the seams are off the answer: {}",
        check.matched
    );
}

#[test]
fn a_bare_half_and_a_darkened_corner_are_hatched_in_the_answers_values() {
    let (answer, fit) = hatching_case();
    // The right half's top quarter painted dark, the left half left bare: hatched there in the
    // answer's values, nowhere else.
    let wrong = |[u, v]: Point| {
        if u < 0.5 {
            None
        } else if v < 0.25 {
            Some(Value::Dark)
        } else {
            Some(answer.at([u, v]))
        }
    };
    let check = read(&answer, &fit, wrong);
    assert!(!check.hatching.is_empty());
    for (value, ends) in &check.hatching {
        for end in ends {
            let [u, v] = share_of(&fit, *end);
            let bare = u < 0.5 + VALUE_PEN / fit.size[0];
            let darkened = v < 0.25 + VALUE_PEN / fit.size[1] && answer.at([u, v]) != Value::Dark;
            assert!(bare || darkened, "{value:?} at {u:.3}, {v:.3}");
        }
        assert_eq!(
            *value,
            answer.at(share_of(&fit, ends[0])),
            "in the answer's value"
        );
    }
    // Both regions are wider than the pen: the bare half and the dark corner each hatched.
    let hatched_at = |inside: &dyn Fn(Point) -> bool| {
        (check.hatching.iter()).any(|(_, ends)| inside(share_of(&fit, ends[0])))
    };
    assert!(hatched_at(&|[u, _]| u < 0.5));
    assert!(hatched_at(&|[u, v]| u >= 0.5 && v < 0.25));
    assert!(check.matched < 0.6, "{}", check.matched);
}
