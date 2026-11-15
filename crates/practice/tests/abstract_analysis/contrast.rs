//! The contrast measure (S112a): synthetic pictures, each clear in the views its contrast is drawn
//! for, along its edge only, and veiled whole in every other.

use openpaint_practice::{ContrastView, Contrasts, Point, Rgb8, VEILED, from_oklch};

const SIZE: [u32; 2] = [300, 200];
/// Clear and veiled, as strengths.
const CLEAR: f64 = 0.9;
const VEIL: f64 = 0.1;
const DARK: Rgb8 = [70; 3];
const LIGHT: Rgb8 = [190; 3];

/// The contrasts of a picture whose pixel at `[u, v]` (shares of its width and height) is
/// `colour`.
fn contrasts(colour: impl Fn(Point) -> Rgb8) -> Contrasts {
    let [width, height] = SIZE.map(f64::from);
    let rgba: Vec<u8> = (0..SIZE[1])
        .flat_map(|y| (0..SIZE[0]).map(move |x| (x, y)))
        .flat_map(|(x, y)| {
            let [r, g, b] = colour([(f64::from(x) + 0.5) / width, (f64::from(y) + 0.5) / height]);
            [r, g, b, 255]
        })
        .collect();
    Contrasts::of(SIZE, &rgba)
}

fn strength(contrasts: &Contrasts, view: ContrastView, p: Point) -> f64 {
    contrasts.at(view, p).expect("a veiled view")
}

/// `views` clear at `p`, every other veiled there.
fn clear_only(contrasts: &Contrasts, views: &[ContrastView], p: Point, what: &str) {
    for view in ContrastView::ALL
        .into_iter()
        .filter(|v| *v != ContrastView::Squint)
    {
        let strength = strength(contrasts, view, p);
        if views.contains(&view) {
            assert!(strength > CLEAR, "{what}: {view:?} at {p:?} is {strength}");
        } else {
            assert!(strength < VEIL, "{what}: {view:?} at {p:?} is {strength}");
        }
    }
}

/// Every veiled view veiled at `p`.
fn veiled(contrasts: &Contrasts, p: Point, what: &str) {
    clear_only(contrasts, &[], p, what);
}

/// `view` veiled all over the picture.
fn veiled_whole(contrasts: &Contrasts, view: ContrastView, what: &str) {
    let cells = contrasts.strength(view).expect("a veiled view");
    let most = cells.iter().copied().fold(0.0, f64::max);
    assert!(most < VEIL, "{what}: {view:?} is {most} somewhere");
}

#[test]
fn a_dark_block_is_clear_in_lightness_and_edges_along_its_edge_only() {
    let inside = |[u, v]: Point| (0.35..0.65).contains(&u) && (0.3..0.7).contains(&v);
    let block = contrasts(|p| if inside(p) { DARK } else { LIGHT });
    let edge = [0.35, 0.5];
    clear_only(
        &block,
        &[ContrastView::Lightness, ContrastView::Edges],
        edge,
        "edge",
    );
    veiled(&block, [0.5, 0.5], "the block's middle");
    veiled(&block, [0.1, 0.1], "the ground");
    for view in [ContrastView::Chroma, ContrastView::Hue] {
        veiled_whole(&block, view, "a grey block");
    }
}

#[test]
fn a_pure_patch_on_a_grey_as_light_is_clear_in_chroma_only() {
    let pure = from_oklch([0.6, 0.15, 30.0]);
    let grey = from_oklch([0.6, 0.0, 0.0]);
    let patch = contrasts(|[u, v]| {
        if (0.35..0.65).contains(&u) && (0.3..0.7).contains(&v) {
            pure
        } else {
            grey
        }
    });
    clear_only(&patch, &[ContrastView::Chroma], [0.35, 0.5], "edge");
    veiled(&patch, [0.5, 0.5], "the patch's middle");
    for view in [
        ContrastView::Lightness,
        ContrastView::Hue,
        ContrastView::Edges,
    ] {
        veiled_whole(&patch, view, "a patch as light as its ground");
    }
}

#[test]
fn a_hue_step_at_one_lightness_and_chroma_is_clear_in_hue_only() {
    let [red, green] = [30.0, 150.0].map(|hue| from_oklch([0.65, 0.08, hue]));
    let step = contrasts(|[u, _]| if u < 0.5 { red } else { green });
    clear_only(&step, &[ContrastView::Hue], [0.5, 0.5], "the step");
    veiled(&step, [0.2, 0.5], "the red");
    veiled(&step, [0.8, 0.5], "the green");
    for view in [
        ContrastView::Lightness,
        ContrastView::Chroma,
        ContrastView::Edges,
    ] {
        veiled_whole(&step, view, "two hues alike but for the hue");
    }
}

#[test]
fn a_hard_edge_is_clear_in_edges_and_a_soft_one_veiled() {
    let hard = contrasts(|[u, _]| if u < 0.5 { DARK } else { LIGHT });
    let soft = contrasts(|[u, _]| {
        let t = ((u - 0.46) / 0.08).clamp(0.0, 1.0);
        let [dark, light] = [DARK[0], LIGHT[0]].map(f64::from);
        [(dark + t * (light - dark)).round() as u8; 3]
    });
    for at in [[0.5, 0.5], [0.49, 0.2]] {
        let views = [ContrastView::Lightness, ContrastView::Edges];
        clear_only(&hard, &views, at, "a hard edge");
        clear_only(&soft, &[ContrastView::Lightness], at, "a soft edge");
    }
    for at in [[0.2, 0.5], [0.8, 0.5]] {
        veiled(&hard, at, "beside a hard edge");
        veiled(&soft, at, "beside a soft edge");
    }
}

#[test]
fn a_flat_picture_is_veiled_whole_and_its_squint_is_its_colour() {
    let colour = from_oklch([0.5, 0.1, 250.0]);
    let flat = contrasts(|_| colour);
    for view in ContrastView::ALL.into_iter().take(4) {
        veiled_whole(&flat, view, "a flat picture");
        let veil = flat.picture(view);
        let alpha = (VEILED * 255.0).round() as u8;
        assert!(veil.chunks(4).all(|pixel| pixel == [255, 255, 255, alpha]));
    }
    let squint = flat.picture(ContrastView::Squint);
    assert!(squint.chunks(4).all(|pixel| {
        let near = |k: usize| pixel[k].abs_diff(colour[k]) <= 1;
        near(0) && near(1) && near(2) && pixel[3] == 255
    }));
    assert_eq!(flat.at(ContrastView::Squint, [0.5, 0.5]), None);
}

#[test]
fn a_strong_contrast_lays_no_veil_on_its_edge() {
    let block = contrasts(|[u, _]| if u < 0.5 { [20; 3] } else { [235; 3] });
    let veil = block.picture(ContrastView::Lightness);
    let [width, height] = block.size.map(|side| side as usize);
    let edge = 4 * ((height / 2) * width + width / 2);
    assert_eq!(veil[edge..edge + 4], [255, 255, 255, 0]);
}
