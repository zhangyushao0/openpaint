//! Value masses (S96): the answer validated on landscapes whose masses are known (`Masses`, its
//! parts' values as its colours keep them): each mass found and its edges within `ACCURACY` of
//! the parts' edges, every island under `ISLAND` merged; and the How's demonstration filling the
//! middle, painting the dark, the light, then compared (S100). A copy's hatching is
//! `value_masses_hatching.rs`.

use openpaint_practice::{
    Demo, Drawn, Family, ISLAND, MASSES_SIDE, Marker, Masses, RIM, Value, ValueMasses,
};

/// How near the answer's mass edges lie to the landscape's known ones, frame units (5 canvas
/// pixels on a 400-pixel frame, the ink pen's width): the algorithm's accuracy, far stricter than
/// the grey pen's width a copy is judged by.
const ACCURACY: f64 = 0.0125;

#[test]
fn the_demonstration_fills_the_middle_then_paints_dark_then_light_and_is_compared_with_its_hatching()
 {
    let demo = Demo::of(Family::ValueMasses).expect("a demonstration");
    let [answer, copy] = demo.values.as_deref().expect("its value masses").clone();
    // The hand's copy strays from the answer, wider in dark (its dark masses run on rightwards).
    let dark = |map: &ValueMasses| map.values.iter().filter(|v| **v == 0).count();
    assert!(dark(&copy) > dark(&answer));
    // S100: Fill frame first, the copy's frame the middle grey (the overall tone), then each step
    // paints its values over it: the dark, then the light too.
    let filled = (demo.marks.iter()).find(|m| m.on == openpaint_practice::On::Copy);
    let filled = filled.map(|m| (&m.drawn, m.from >= demo.steps[0] && m.to <= demo.steps[1]));
    assert!(matches!(
        filled,
        Some((Drawn::Band { colour: Some(grey), .. }, true)) if *grey == Value::Middle.grey()
    ));
    let painted: Vec<[bool; 3]> = (demo.marks.iter())
        .filter_map(|m| match m.drawn {
            Drawn::Values {
                copy: true, shown, ..
            } => Some(shown),
            _ => None,
        })
        .collect();
    assert_eq!(painted, [[true, false, false], [true, false, true]]);
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

/// `ACCURACY` in an answer's cells: the picture's longer side fills its frame (S99).
fn accuracy_cells() -> f64 {
    ACCURACY * f64::from(MASSES_SIDE)
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

/// The cells left out of the check: a detail and everything within twice `reach` of it (a
/// squinting eye may keep it or lose it), and the picture's rim, which is no mass (an edge runs
/// straight across it).
fn left_out(truth: &[(Value, bool)], w: usize, reach: f64) -> Vec<bool> {
    let details: Vec<bool> = truth.iter().map(|(_, d)| *d).collect();
    let h = truth.len() / w;
    let rim = (RIM * f64::from(MASSES_SIDE)).round() as usize;
    let on_rim = |at: usize| {
        let (x, y) = (at % w, at / w);
        x < rim || y < rim || x + rim >= w || y + rim >= h
    };
    (0..truth.len())
        .map(|at| on_rim(at) || near(&details, w, at, 2.0 * reach))
        .collect()
}

#[test]
fn each_known_mass_is_found_and_its_edges_lie_within_five_pixels() {
    let reach = accuracy_cells();
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
        let left_out = left_out(&truth, w, reach);
        let edges = |values: &[Value]| -> Vec<bool> {
            (0..values.len()).map(|at| on_edge(values, w, at)).collect()
        };
        let (known_edges, found_edges) = (edges(&expected), edges(&found));
        let (mut inside, mut wrong, mut stray, mut missed, mut edge_cells) = (0, 0, 0, 0, 0);
        for at in (0..truth.len()).filter(|at| !left_out[*at]) {
            if !near(&known_edges, w, at, reach) {
                inside += 1;
                wrong += usize::from(found[at] != expected[at]);
            }
            if found_edges[at] && !near(&known_edges, w, at, reach) {
                stray += 1;
            }
            if known_edges[at] {
                edge_cells += 1;
                missed += usize::from(!near(&found_edges, w, at, reach));
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
