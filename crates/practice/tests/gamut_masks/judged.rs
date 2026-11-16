//! A gamut mask's copy judged (S153): a synthetic copy painted inside by rule clean whatever the
//! mask; copies made wrong on purpose each marked by its rule, with its reason first.

use openpaint_practice::{
    AWAY, Areas, CardItem, Compared, LIGHTNESS_OFF, Mask, MaskCheck, MaskCompared, MaskRead,
    MaskShape, Metric, Outcome, Random, Rgb8, Say, Seen, Verdict, from_oklab, gamut_distance,
    gamut_nearest, in_gamut, main_hue, oklab, way,
};

use super::ab;

/// A crop of four areas in quadrants, each a quarter, all four meeting: a rust, a cool grey, a
/// blue and a green, of middle lightnesses.
pub(crate) fn quadrants() -> Areas {
    let map: Vec<u8> = (0..64)
        .map(|cell| {
            let (row, column) = (cell / 8, cell % 8);
            u8::from(column >= 4) + 2 * u8::from(row >= 4)
        })
        .collect();
    let colours = [
        [0.55, 0.1, 40.0],
        [0.68, 0.015, 240.0],
        [0.48, 0.1, 255.0],
        [0.62, 0.1, 145.0],
    ];
    Areas::of_map([8, 8], map, colours.map(in_gamut).to_vec()).expect("a crop")
}

/// `colour` painted inside `mask` by rule, its lightness kept: where it lies outside, a third of
/// the way from its nearest point toward the mask's middle.
fn inside(mask: Mask, colour: Rgb8) -> Rgb8 {
    let [l, a, b] = oklab(colour);
    let corners = mask.corners();
    if gamut_distance(&corners, [a, b]) == 0.0 {
        return colour;
    }
    let q = gamut_nearest(&corners, [a, b]);
    let m = mask.middle();
    let [x, y] = [q[0] + (m[0] - q[0]) / 3.0, q[1] + (m[1] - q[1]) / 3.0];
    from_oklab([l, x, y])
}

/// `areas`' copy painted inside `mask` by rule.
fn by_rule(areas: &Areas, mask: Mask) -> Vec<Option<Rgb8>> {
    (areas.colours.iter())
        .map(|c| Some(inside(mask, *c)))
        .collect()
}

/// A compared copy as read.
fn compared(areas: &Areas, copy: Vec<Option<Rgb8>>, mask: Mask) -> MaskCompared {
    let check = MaskCheck::of(areas, &copy, mask);
    MaskCompared {
        colours: areas.colours.clone(),
        corners: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        mask,
        read: Some(MaskRead {
            colours: copy,
            check,
            outlines: Vec::new(),
            edges: Vec::new(),
        }),
    }
}

fn seen() -> Seen<'static> {
    Seen {
        construction: false,
        shown: 1.0,
        chosen: 0,
        outline: &[],
        grey: false,
        view: 0,
        fit: None,
        probe: None,
    }
}

#[test]
fn a_copy_painted_inside_by_rule_is_clean_whatever_the_mask() {
    let areas = quadrants();
    assert_eq!(areas.neighbours.len(), 4, "the quadrants meet side by side");
    let mut random = Random::new(11);
    for shape in MaskShape::ALL {
        for _ in 0..12 {
            let mask = Mask::chosen(shape, main_hue(&areas), AWAY, &mut random);
            let copy = by_rule(&areas, mask);
            let check = MaskCheck::of(&areas, &copy, mask);
            assert!(check.outside.is_empty(), "{mask:?}: {check:?}");
            assert!(check.off_lightness.is_empty(), "{mask:?}");
            assert_eq!(check.metrics(), [(Metric::MaskOutside, 0.0)]);
            let compared = compared(&areas, copy, mask);
            if check.merged.is_empty() {
                assert_eq!(compared.verdict(), Verdict::App(Outcome::Clean), "{mask:?}");
                assert!(compared.reason().is_none());
                let card = compared.card(&seen());
                assert!(matches!(card[0], CardItem::Grey), "no reason first");
            }
        }
    }
}

#[test]
fn an_area_outside_is_marked_with_which_way_it_left() {
    let areas = quadrants();
    // A triangle toward orange: the blue across from it must be greyed.
    let orange = Mask {
        shape: MaskShape::Triangle,
        hue: 60,
    };
    let blue = 2;
    let mut copy = by_rule(&areas, orange);
    copy[blue] = Some(areas.colours[blue]);
    let compared = compared(&areas, copy.clone(), orange);
    let check = &compared.read.as_ref().expect("read").check;
    assert_eq!(check.outside.len(), 1);
    assert!(check.is_outside(blue) && check.marked(blue));
    assert_eq!(check.metrics(), [(Metric::MaskOutside, 1.0)]);
    assert_eq!(compared.verdict(), Verdict::App(Outcome::Near));
    let (area, say) = compared.reason().expect("a reason");
    assert_eq!((area, say.key), (blue, "outside-mask"));
    assert_eq!(way(&orange.corners(), ab(areas.colours[blue])), "pure");
    assert_eq!(compared.named_area(), Some(blue));
    let card = compared.card(&seen());
    assert_eq!(card[0], CardItem::Says(say), "the reason first");
    let marked = card.iter().find_map(|item| match item {
        CardItem::Gamut { marked, .. } => Some(marked.clone()),
        _ => None,
    });
    assert_eq!(marked, Some(vec![blue]), "ringed on the disc");
    // Two areas outside: again.
    copy[3] = Some(areas.colours[3]);
    assert_eq!(
        self::compared(&areas, copy, orange).verdict(),
        Verdict::App(Outcome::Redo)
    );
}

#[test]
fn off_a_small_mask_a_grey_is_too_grey_and_a_colour_beside_it_leans_to_a_hue() {
    let small = Mask {
        shape: MaskShape::Small,
        hue: 60,
    };
    let at = |chroma: f64, hue: f64| {
        let (sin, cos) = hue.to_radians().sin_cos();
        [chroma * cos, chroma * sin]
    };
    assert_eq!(way(&small.corners(), [0.0, 0.0]), "grey");
    assert_eq!(way(&small.corners(), [0.01, 0.005]), "grey");
    assert!(!small.holds(at(0.1, 120.0)));
    assert_eq!(way(&small.corners(), at(0.1, 120.0)), "green");
    assert_eq!(way(&small.corners(), at(0.1, 340.0)), "purple");
    assert_eq!(
        way(&small.corners(), at(0.3, 90.0)),
        "pure",
        "far out along its hue"
    );
}

#[test]
fn a_lightness_left_or_neighbours_merged_are_each_marked() {
    let areas = quadrants();
    let mask = Mask {
        shape: MaskShape::Diamond,
        hue: 60,
    };
    let rule = by_rule(&areas, mask);
    let rust = 0;
    for (off, marked) in [(LIGHTNESS_OFF + 0.03, true), (LIGHTNESS_OFF - 0.03, false)] {
        let mut copy = rule.clone();
        let [l, a, b] = oklab(rule[rust].expect("painted"));
        copy[rust] = Some(from_oklab([l + off, a, b]));
        let compared = compared(&areas, copy, mask);
        let check = &compared.read.as_ref().expect("read").check;
        assert_eq!(check.off_lightness.contains(&rust), marked, "{off}");
        assert!(check.outside.is_empty());
        if marked {
            let (area, say) = compared.reason().expect("a reason");
            assert_eq!(area, rust);
            assert_eq!(say, Say::way("colour", "light").numbered(1));
        }
    }
    // Two neighbours of near lightness painted one colour (the rust's over the blue's): merged,
    // its reason the blocks.
    let mut copy = rule.clone();
    copy[2] = copy[0];
    let compared = compared(&areas, copy, mask);
    let check = &compared.read.as_ref().expect("read").check;
    assert_eq!(check.merged, [(0, 2)]);
    assert_eq!(check.findings(), 1);
    let (_, say) = compared.reason().expect("a reason");
    assert_eq!(say, Say::counting("merged", &[2, 1]));
    // An area not painted: incomplete, judged by nothing.
    let mut copy = rule;
    copy[rust] = None;
    let compared = self::compared(&areas, copy, mask);
    assert_eq!(compared.verdict(), Verdict::Incomplete);
    assert!(!compared.read.as_ref().expect("read").check.marked(rust));
}
