//! Colour combinations from real crops (S101): the answer the pack builder finds, validated on S82's
//! generated pictures, whose areas are known (each area found, its colour within ΔE 0.02, the weak
//! pairs found), and the rules a crop is held to.

use openpaint_practice::{
    Areas, Arrangement, CropRefusal, FEWEST_AREAS, GeneratedAreas, LEAST_LONGER, MOST_AREAS,
    Places, Tier, best, candidates, delta_e, refusal, tier_of,
};

/// The generated pictures' side in pixels: about a crop's.
const SIDE: u32 = 400;

/// The found area each of `known`'s areas lies in: the one most of its core's cells fall in.
fn matched(known: &Areas, found: &Areas) -> Vec<usize> {
    let [w, h] = known.size;
    (0..known.count())
        .map(|area| {
            let mut votes = vec![0usize; found.count()];
            for row in 0..h {
                for column in 0..w {
                    let p = [
                        (column as f64 + 0.5) / w as f64,
                        (row as f64 + 0.5) / h as f64,
                    ];
                    if known.area_at(p) == area {
                        votes[found.area_at(p)] += 1;
                    }
                }
            }
            (0..votes.len()).max_by_key(|a| votes[*a]).unwrap_or(0)
        })
        .collect()
}

#[test]
fn the_answer_finds_every_known_area_its_colour_and_its_weak_pairs() {
    let mut cells = (0usize, 0usize);
    for arrangement in Arrangement::ALL {
        for seed in 0..24 {
            let generated = GeneratedAreas::generate(arrangement, seed);
            let known = &generated.areas;
            let found = Areas::found([SIDE; 2], &generated.picture(SIDE));
            let areas = &found.areas;
            let label = format!("{arrangement:?} {seed}");
            assert_eq!(areas.count(), known.count(), "{label}: every area, no more");
            let to = matched(known, areas);
            let mut seen = to.clone();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), known.count(), "{label}: one found area each");
            for (area, at) in to.iter().enumerate() {
                let off = delta_e(areas.colours[*at], known.colours[area]);
                assert!(off <= 0.02, "{label}: area {area} off by {off}");
            }
            for (a, b) in known.weak_pairs() {
                let pair = (to[a].min(to[b]), to[a].max(to[b]));
                assert!(areas.weak_pairs().contains(&pair), "{label}: weak {a}, {b}");
            }
            assert!(
                found.busy < 0.03,
                "{label}: flat areas read flat ({})",
                found.busy
            );
            for p in (0..64).map(|k| {
                [
                    (f64::from(k % 8) + 0.5) / 8.0,
                    (f64::from(k / 8) + 0.5) / 8.0,
                ]
            }) {
                cells.0 += usize::from(to[known.area_at(p)] == areas.area_at(p));
                cells.1 += 1;
            }
        }
    }
    let share = cells.0 as f64 / cells.1 as f64;
    assert!(
        share > 0.95,
        "the areas lie where the picture's do: {share}"
    );
}

/// A picture `size` pixels painted with `paint`, a colour a pixel from its point in shares.
fn picture(size: [u32; 2], mut paint: impl FnMut([f64; 2]) -> [u8; 3]) -> Vec<u8> {
    (0..size[0] * size[1])
        .flat_map(|at| {
            let [x, y] = [at % size[0], at / size[0]];
            let p = [
                (f64::from(x) + 0.5) / f64::from(size[0]),
                (f64::from(y) + 0.5) / f64::from(size[1]),
            ];
            let [r, g, b] = paint(p);
            [r, g, b, 255]
        })
        .collect()
}

#[test]
fn a_crop_keeps_its_rules_or_says_which_it_breaks() {
    // Two flat colours: too few areas.
    let two = picture([300, 300], |[x, _]| {
        if x < 0.5 {
            [90, 70, 60]
        } else {
            [200, 190, 170]
        }
    });
    let found = Areas::found([300, 300], &two);
    assert_eq!(refusal(&found), Some(CropRefusal::FewAreas));
    // Three bands, every pair far apart: no weak contrast.
    let far = picture([300, 300], |[_, y]| match (3.0 * y) as u32 {
        0 => [40, 40, 40],
        1 => [140, 120, 100],
        _ => [235, 230, 220],
    });
    assert_eq!(
        refusal(&Areas::found([300, 300], &far)),
        Some(CropRefusal::NoWeakContrast)
    );
    // Three pure bands, two of them a weak step apart: nothing grey.
    let pure = picture([300, 300], |[_, y]| match (3.0 * y) as u32 {
        0 => [220, 30, 30],
        1 => [230, 45, 35],
        _ => [30, 60, 220],
    });
    assert_eq!(
        refusal(&Areas::found([300, 300], &pure)),
        Some(CropRefusal::NothingGrey)
    );
    // Three greys a weak step apart each: one surface's shades, no colours plainly apart.
    let close = picture([300, 300], |[_, y]| {
        [[120; 3], [128; 3], [136; 3]][(3.0 * y) as usize]
    });
    let found = Areas::found([300, 300], &close);
    assert_eq!(found.areas.count(), 3);
    assert_eq!(refusal(&found), Some(CropRefusal::AllClose));
    // Noise over everything: more texture than areas.
    let mut state = 0x5EED_0101_u64;
    let noise = picture([300, 300], |_| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let v = (state >> 33) as u8;
        [v, v.wrapping_mul(3), v / 2]
    });
    let found = Areas::found([300, 300], &noise);
    assert!(
        refusal(&found).is_some(),
        "{} areas, busy {}",
        found.areas.count(),
        found.busy
    );
}

#[test]
fn a_shading_is_one_area_and_rust_keeps_every_rule() {
    // A grey shading evenly across two thirds and two blues beside it: the shading is one area,
    // however far apart its ends, for no edge parts it.
    let shaded = picture([300, 300], |[x, y]| match (x < 2.0 / 3.0, y < 0.5) {
        (true, _) => [(110.0 + 90.0 * x) as u8; 3],
        (false, true) => [60, 80, 120],
        (false, false) => [70, 90, 132],
    });
    assert_eq!(Areas::found([300, 300], &shaded).areas.count(), 3);
    // Rust: a dull brown, a slightly lighter brown beside it, a grey and a blue grey.
    let rust = picture([300, 300], |[x, y]| match (x < 0.5, y < 0.5) {
        (true, true) => [120, 80, 60],
        (false, true) => [128, 86, 64],
        (true, false) => [150, 150, 145],
        (false, false) => [80, 95, 115],
    });
    let found = Areas::found([300, 300], &rust);
    assert_eq!(refusal(&found), None);
    assert_eq!(found.areas.count(), 4);
    assert_eq!(tier_of(&found.areas), Tier::Simple);
    assert!((FEWEST_AREAS..=MOST_AREAS).contains(&found.areas.count()));
}

#[test]
fn the_builder_keeps_the_best_crops_none_overlapping_and_the_same_every_time() {
    // A wide picture: four patches of rust meeting at a point left of its middle and right of
    // it, as a crop sees them; only its largest size reaches `LEAST_LONGER` (0.55 of 1,400).
    let size = [2800, 1400];
    let rgba = picture(size, |[x, y]| {
        let centre = if x < 0.5 { 0.1375 } else { 0.8625 };
        match (x < centre, y < 0.5) {
            (true, true) => [120, 80, 60],
            (false, true) => [128, 86, 64],
            (true, false) => [150, 150, 145],
            (false, false) => [80, 95, 115],
        }
    });
    let all = candidates(size, &rgba, Places::Anywhere);
    assert_eq!(all.len(), 27, "one size, three shapes, nine places");
    assert!(
        all.iter()
            .all(|crop| crop.size[0].max(crop.size[1]) >= LEAST_LONGER)
    );
    let small = picture([1200, 600], |_| [128; 3]);
    assert!(
        candidates([1200, 600], &small, Places::Anywhere).is_empty(),
        "no size of a small picture reaches it"
    );
    let kept = best(all.clone(), 2);
    assert_eq!(kept.len(), 2);
    let again = best(candidates(size, &rgba, Places::Anywhere), 2);
    assert_eq!(kept, again, "the same crops from the same picture");
    for crop in &kept {
        assert_eq!(refusal(&crop.found), None);
        assert!(crop.at[0] + crop.size[0] <= size[0] && crop.at[1] + crop.size[1] <= size[1]);
    }
    assert!(kept[0].score() >= kept[1].score());
    let corners = candidates(size, &rgba, Places::Corners);
    assert!(corners.iter().all(|crop| {
        let [x, y] = crop.at;
        (x == 0 || x + crop.size[0] == size[0]) && (y == 0 || y + crop.size[1] == size[1])
    }));
}
