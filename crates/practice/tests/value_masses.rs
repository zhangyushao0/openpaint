//! Value masses (S96): the answer validated on landscapes whose masses are known (`Masses`, its
//! parts' values as its colours keep them): each mass found and its edges within a pen's width of
//! the parts' edges, every island under `ISLAND` merged; a copy painted in the three greys read
//! against it, the hatching only where it differs over more than a pen's width; and the How's
//! demonstration painting the dark, the light, the middle, then compared.

use openpaint_practice::{
    Demo, Drawn, Family, ISLAND, MASSES_PEN, MASSES_SIDE, Marker, Masses, Metric, PICTURE_MARGIN,
    PictureFit, Point, RIM, Value, ValueMasses, value,
};

#[test]
fn the_demonstration_paints_dark_then_light_then_middle_and_is_compared_with_its_hatching() {
    let demo = Demo::of(Family::ValueMasses).expect("a demonstration");
    let [answer, copy] = demo.values.as_deref().expect("its value masses").clone();
    // The hand's copy strays from the answer, wider in dark (its dark masses run on rightwards).
    let dark = |map: &ValueMasses| map.values.iter().filter(|v| **v == 0).count();
    assert!(dark(&copy) > dark(&answer));
    // Each step paints its values on the copy: the dark, then the light too, then all.
    let painted: Vec<[bool; 3]> = (demo.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Values {
                copy: true, shown, ..
            } => Some(shown),
            _ => None,
        })
        .collect();
    assert_eq!(
        painted,
        [[true, false, false], [true, false, true], [true; 3]]
    );
    // Compare: the answer's greys faint, its mass edges red, the stray hatched in the grey the
    // answer holds there (the middle of the hills, the light of the sky by the crown's top), never
    // in the dark it was painted.
    let compare = demo.steps[3];
    let shown = (demo.marks.iter()).filter(|m| m.from >= compare);
    assert!(shown.clone().any(|m| m.pen == Marker::Faint));
    assert!(shown.clone().filter(|m| m.pen == Marker::Answer).count() >= 3);
    let hatched: Vec<Marker> = (shown.map(|m| m.pen))
        .filter(|pen| matches!(pen, Marker::Hatch(_)))
        .collect();
    assert!(hatched.contains(&Marker::Hatch(Value::Middle)));
    assert!(!hatched.contains(&Marker::Hatch(Value::Dark)));
}

/// A pen's width in an answer's cells: the picture's longer side fills the frame but its margin.
fn pen_cells() -> f64 {
    MASSES_PEN / (1.0 - 2.0 * PICTURE_MARGIN) * f64::from(MASSES_SIDE)
}

/// The landscapes: seeds and aspects, day and evening among them.
fn landscapes() -> impl Iterator<Item = (u64, Masses)> {
    (0..12_u64).map(|seed| {
        let aspect = [0.75, 1.0, 0.6][seed as usize % 3];
        (seed, Masses::generate(seed, aspect))
    })
}

/// Each cell's known value and whether a detail lies there (a door, a window, the trunk, the roof).
fn known(masses: &Masses, answer: &ValueMasses) -> Vec<(Value, bool)> {
    let [w, h] = answer.size.map(|side| side as usize);
    let [width, height] = masses.size;
    (0..w * h)
        .map(|at| {
            let (x, y) = (at % w, at / w);
            let p = [
                (x as f64 + 0.5) / w as f64 * width,
                (y as f64 + 0.5) / h as f64 * height,
            ];
            masses.known(p).unwrap_or((Value::Middle, true))
        })
        .collect()
}

/// Whether the cell `at` of a `w`-wide map lies on an edge: its right or lower neighbour differs.
fn on_edge(values: &[Value], w: usize, at: usize) -> bool {
    let (x, h) = (at % w, values.len() / w);
    let right = x + 1 < w && values[at + 1] != values[at];
    let below = at / w + 1 < h && values[at + w] != values[at];
    right || below
}

/// Whether some cell `marked` lies within `reach` cells of the cell `at`.
fn near(marked: &[bool], w: usize, at: usize, reach: f64) -> bool {
    let (x, y, h) = (
        (at % w) as isize,
        (at / w) as isize,
        (marked.len() / w) as isize,
    );
    let r = reach.ceil() as isize;
    (-r..=r).any(|dy| {
        (-r..=r).any(|dx| {
            let (nx, ny) = (x + dx, y + dy);
            let inside = (0..w as isize).contains(&nx) && (0..h).contains(&ny);
            inside
                && ((dx * dx + dy * dy) as f64) <= reach * reach
                && marked[(ny * w as isize + nx) as usize]
        })
    })
}

/// The cells left out of the check: a detail and everything within two pens of it (a squinting
/// eye may keep it or lose it), and the picture's rim, which is no mass (an edge runs straight
/// across it).
fn left_out(truth: &[(Value, bool)], w: usize, pen: f64) -> Vec<bool> {
    let details: Vec<bool> = truth.iter().map(|(_, d)| *d).collect();
    let h = truth.len() / w;
    let rim = (RIM * f64::from(MASSES_SIDE)).round() as usize;
    let on_rim = |at: usize| {
        let (x, y) = (at % w, at / w);
        x < rim || y < rim || x + rim >= w || y + rim >= h
    };
    (0..truth.len())
        .map(|at| on_rim(at) || near(&details, w, at, 2.0 * pen))
        .collect()
}

#[test]
fn each_known_mass_is_found_and_its_edges_lie_within_a_pens_width() {
    let pen = pen_cells();
    let mut checked = [0_usize; 2];
    for (seed, masses) in landscapes() {
        let (size, pixels) = masses.picture(640);
        let answer = ValueMasses::of(size, &pixels);
        let [w, _] = answer.size.map(|side| side as usize);
        let truth = known(&masses, &answer);
        let expected: Vec<Value> = truth.iter().map(|(v, _)| *v).collect();
        let found: Vec<Value> = (answer.values.iter())
            .map(|v| Value::of_index(*v).unwrap_or(Value::Middle))
            .collect();
        let left_out = left_out(&truth, w, pen);
        let edges = |values: &[Value]| -> Vec<bool> {
            (0..values.len()).map(|at| on_edge(values, w, at)).collect()
        };
        let (known_edges, found_edges) = (edges(&expected), edges(&found));
        let (mut inside, mut wrong, mut stray, mut missed, mut edge_cells) = (0, 0, 0, 0, 0);
        for at in (0..truth.len()).filter(|at| !left_out[*at]) {
            if !near(&known_edges, w, at, pen) {
                inside += 1;
                wrong += usize::from(found[at] != expected[at]);
            }
            if found_edges[at] && !near(&known_edges, w, at, pen) {
                stray += 1;
            }
            if known_edges[at] {
                edge_cells += 1;
                missed += usize::from(!near(&found_edges, w, at, pen));
            }
        }
        assert!(
            inside > truth.len() / 2,
            "seed {seed}: {inside} cells checked"
        );
        assert_eq!(
            wrong, 0,
            "seed {seed}: {wrong} of {inside} cells inside a mass wrong"
        );
        assert_eq!(
            stray, 0,
            "seed {seed}: {stray} edge cells away from every known edge"
        );
        assert_eq!(
            missed, 0,
            "seed {seed}: {missed} of {edge_cells} known edge cells missed"
        );
        checked = [checked[0] + inside, checked[1] + edge_cells];
    }
    eprintln!(
        "{} cells inside masses, {} known edge cells",
        checked[0], checked[1]
    );
}

#[test]
fn every_mass_left_is_at_least_an_island_large() {
    for (seed, masses) in landscapes() {
        let (size, pixels) = masses.picture(400);
        let answer = ValueMasses::of(size, &pixels);
        let [w, h] = answer.size.map(|side| side as usize);
        let least = (ISLAND * (w * h) as f64).ceil() as usize;
        let mut seen = vec![false; w * h];
        for start in 0..w * h {
            if seen[start] {
                continue;
            }
            let (mut count, mut stack) = (0, vec![start]);
            seen[start] = true;
            while let Some(at) = stack.pop() {
                count += 1;
                let (x, y) = (at % w, at / w);
                let around = [
                    (x > 0).then(|| at - 1),
                    (x + 1 < w).then_some(at + 1),
                    (y > 0).then(|| at - w),
                    (y + 1 < h).then_some(at + w),
                ];
                for next in around.into_iter().flatten() {
                    if !seen[next] && answer.values[next] == answer.values[start] {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            assert!(
                count >= least,
                "seed {seed}: a mass of {count} cells under {least}"
            );
        }
        // Grain and mottle make no island: speckled noise over two halves is two masses.
        let noisy: Vec<u8> = (0..300 * 200_usize)
            .flat_map(|at| {
                let speck = (at.wrapping_mul(2_654_435_761) >> 7) % 9;
                let base = if at % 300 < 150 { 60 } else { 200 };
                let grey = (base + speck * 6) as u8;
                [grey, grey, grey, 255]
            })
            .collect();
        let halves = ValueMasses::of([300, 200], &noisy);
        let lefts = (halves.values.iter().enumerate())
            .filter(|(at, _)| at % halves.size[0] as usize == 10)
            .map(|(_, v)| *v);
        let first = halves.values[10];
        assert!(lefts.into_iter().all(|v| v == first));
    }
}

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

#[test]
fn a_copy_is_hatched_only_where_it_differs_over_more_than_a_pens_width() {
    let masses = Masses::generate(3, 0.75);
    let (size, pixels) = masses.picture(400);
    let answer = ValueMasses::of(size, &pixels);
    let fit = PictureFit::of(size);
    let place = [fit.origin, fit.size];
    let side = 400;
    // The answer itself: nothing hatched, all of it matched.
    let exact = painted(side, &fit, |share| Some(answer.at(share)));
    let check = answer.read(([side, side], &exact), place);
    assert!(check.hatching.is_empty(), "{:?}", check.hatching.first());
    assert!(check.matched > 0.999, "{}", check.matched);
    assert_eq!(
        value(&check.metrics, Metric::MassesMatched),
        Some(check.matched)
    );
    // Shifted by under a pen's width (half of one): still nothing hatched.
    let shift = 0.5 * MASSES_PEN / fit.size[0];
    let near = painted(side, &fit, |[u, v]| Some(answer.at([u - shift, v])));
    assert!(
        answer
            .read(([side, side], &near), place)
            .hatching
            .is_empty()
    );
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
    let off = painted(side, &fit, wrong);
    let check = answer.read(([side, side], &off), place);
    assert!(!check.hatching.is_empty());
    let pen = MASSES_PEN;
    for (value, ends) in &check.hatching {
        for [x, y] in ends {
            let [u, v] = [
                (x - fit.origin[0]) / fit.size[0],
                (y - fit.origin[1]) / fit.size[1],
            ];
            let bare = u < 0.5 + pen / fit.size[0];
            let darkened = v < 0.25 + pen / fit.size[1] && answer.at([u, v]) != Value::Dark;
            assert!(bare || darkened, "{value:?} at {u:.3}, {v:.3}");
        }
        let share = [
            (ends[0][0] - fit.origin[0]) / fit.size[0],
            (ends[0][1] - fit.origin[1]) / fit.size[1],
        ];
        assert_eq!(*value, answer.at(share), "hatched in the answer's value");
    }
    assert!(check.matched < 0.6, "{}", check.matched);
}
