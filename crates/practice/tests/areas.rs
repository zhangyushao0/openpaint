//! Colour combinations: the same picture per seed, every picture keeping its rules, its copy read
//! from pixels as the eye reads it, and the weak-contrast check on known cases.

use openpaint_practice::{
    Areas, Arrangement, Contrast, GeneratedAreas, Metric, Rgb8, STRONG, WEAK, delta_e, from_hsv,
    hsv, value,
};

/// The sheet's paper.
const PAPER: Rgb8 = [255, 255, 255];
/// The copy's side in pixels, as the frame is read on the sheet.
const SIDE: u32 = 400;

#[test]
fn the_same_seed_gives_the_same_picture_and_every_picture_keeps_its_rules() {
    for arrangement in Arrangement::ALL {
        let mut softs = 0;
        for seed in 0..60 {
            let generated = GeneratedAreas::generate(arrangement, seed);
            let areas = &generated.areas;
            assert_eq!(
                *areas,
                GeneratedAreas::generate(arrangement, seed).areas,
                "{arrangement:?} {seed}"
            );
            let count = areas.count();
            assert!(
                (4..=6).contains(&count),
                "{arrangement:?} {seed}: {count} areas"
            );
            let contrasts: Vec<f64> = (areas.neighbours.iter())
                .map(|(a, b)| delta_e(areas.colours[*a], areas.colours[*b]))
                .collect();
            assert!(
                contrasts.len() >= count - 1,
                "{arrangement:?} {seed}: areas apart"
            );
            assert!(
                contrasts.iter().all(|c| *c >= 0.025),
                "{arrangement:?} {seed}"
            );
            assert!(
                contrasts.iter().any(|c| *c <= WEAK),
                "{arrangement:?} {seed}: a weak one"
            );
            assert!(
                contrasts.iter().any(|c| *c >= STRONG),
                "{arrangement:?} {seed}: a strong one"
            );
            let low = (areas.colours.iter()).filter(|c| hsv(**c)[1] < 0.3).count();
            assert!(
                2 * low >= count,
                "{arrangement:?} {seed}: low in saturation"
            );
            softs += areas.soft.iter().filter(|w| **w > 0.0).count();
        }
        assert!(
            softs > 20,
            "{arrangement:?}: soft edges among the hard ({softs})"
        );
        assert_ne!(
            GeneratedAreas::generate(arrangement, 11).areas.colours,
            GeneratedAreas::generate(arrangement, 12).areas.colours
        );
    }
}

/// A copy painted flat: each pixel the colour `colours` gives its area.
fn painted(areas: &Areas, colours: &[Rgb8], shift: [f64; 2]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let p = [x, y].map(|v| (f64::from(v) + 0.5) / f64::from(SIDE));
            let moved = [p[0] - shift[0], p[1] - shift[1]].map(|v| v.clamp(0.0, 0.999));
            let [r, g, b] = colours[areas.area_at(moved)];
            pixels.extend_from_slice(&[r, g, b, 255]);
        }
    }
    pixels
}

#[test]
fn the_picture_itself_reads_as_its_colours_whatever_its_soft_edges() {
    for (arrangement, seed) in [
        (Arrangement::Strata, 3),
        (Arrangement::Patches, 5),
        (Arrangement::Blocks, 8),
    ] {
        let generated = GeneratedAreas::generate(arrangement, seed);
        let areas = &generated.areas;
        let picture = generated.picture(SIDE);
        let copy = areas.read([SIDE, SIDE], &picture, PAPER);
        let expected: Vec<Option<Rgb8>> = areas.colours.iter().copied().map(Some).collect();
        assert_eq!(copy.colours, expected, "{arrangement:?} {seed}");
        assert!(copy.contrasts.iter().all(|c| !c.made_strong()));
        let metrics = copy.metrics(areas);
        assert_eq!(value(&metrics, Metric::AreasDelta), Some(0.0));
        assert_eq!(value(&metrics, Metric::AreasMadeStrong), Some(0.0));
        let weak = copy.contrasts.iter().filter(|c| c.weak()).count();
        assert!(weak >= 1);
        assert_eq!(value(&metrics, Metric::AreasWeak), Some(weak as f64));
    }
}

#[test]
fn an_area_reads_as_the_eye_gives_it_edges_off_strays_over_it_or_left_bare() {
    let generated = GeneratedAreas::generate(Arrangement::Patches, 21);
    let areas = &generated.areas;
    let colours = areas.colours.clone();
    // Edges three hundredths of the frame off (12 pixels): the same colours.
    let moved = painted(areas, &colours, [0.03, -0.02]);
    let read = areas.read([SIDE, SIDE], &moved, PAPER);
    assert_eq!(
        read.colours,
        colours.iter().copied().map(Some).collect::<Vec<_>>()
    );
    // A third of the pixels another colour (a texture, strokes of the next area across it):
    // the median keeps the area's.
    let mut strays = painted(areas, &colours, [0.0, 0.0]);
    for (index, pixel) in strays.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        if index % 3 == 0 {
            pixel[..3].copy_from_slice(&[20, 200, 40]);
        }
    }
    let read = areas.read([SIDE, SIDE], &strays, PAPER);
    assert_eq!(
        read.colours,
        colours.iter().copied().map(Some).collect::<Vec<_>>()
    );
    // An area left as the paper is not painted, and its contrasts are not judged.
    let mut bare = colours.clone();
    bare[0] = PAPER;
    let read = areas.read([SIDE, SIDE], &painted(areas, &bare, [0.0, 0.0]), PAPER);
    assert_eq!(read.colours[0], None);
    assert!(read.colours[1..].iter().all(Option::is_some));
    let with_first = read.contrasts.iter().filter(|c| c.areas.0 == 0);
    assert!(with_first.clone().count() >= 1);
    assert!(
        with_first
            .into_iter()
            .all(|c| c.copy.is_none() && !c.made_strong())
    );
}

#[test]
fn weak_and_strong_contrasts_match_known_greys() {
    // Greys a few levels apart are barely told apart; about a Munsell value step apart, plainly.
    let grey = |v: u8| [v; 3];
    assert!(
        delta_e(grey(128), grey(133)) < 0.02,
        "under a just-noticeable step"
    );
    assert!(delta_e(grey(128), grey(140)) <= WEAK, "a weak contrast");
    assert!(delta_e(grey(128), grey(150)) > WEAK, "no longer weak");
    assert!(delta_e(grey(128), grey(180)) >= STRONG, "a strong contrast");
    // A dull brown beside a slightly lighter one: weak; beside a blue grey of its value: not.
    let brown = from_hsv([28.0, 0.35, 0.45]);
    assert!(delta_e(brown, from_hsv([32.0, 0.33, 0.48])) <= WEAK);
    assert!(delta_e(brown, from_hsv([215.0, 0.3, 0.45])) > WEAK);
}

#[test]
fn a_weak_contrast_is_made_strong_only_when_the_copy_plainly_widens_it() {
    let contrast = |original: f64, copy: Option<f64>| Contrast {
        areas: (0, 1),
        original,
        copy,
    };
    assert!(contrast(0.04, Some(0.09)).made_strong(), "0.04 to 0.09");
    assert!(!contrast(0.04, Some(0.085)).made_strong(), "not 0.05 more");
    assert!(contrast(0.03, Some(0.08)).made_strong());
    assert!(
        !contrast(0.03, Some(0.05)).made_strong(),
        "a little stronger, still weak"
    );
    assert!(
        contrast(0.01, Some(0.07)).made_strong(),
        "from nothing to a step"
    );
    assert!(
        !contrast(0.07, Some(0.3)).made_strong(),
        "strong before: not a weak one"
    );
    assert!(!contrast(0.04, None).made_strong(), "not painted");
    assert!(!contrast(0.05, Some(0.01)).made_strong(), "made weaker");
}

/// `colour` lighter or darker by `by` of the wheel's value.
fn lighter(colour: Rgb8, by: f64) -> Rgb8 {
    let [h, s, v] = hsv(colour);
    from_hsv([h, s, (v + by).clamp(0.0, 1.0)])
}

#[test]
fn a_copy_that_widens_a_weak_contrast_is_caught_there_and_one_lighter_all_over_is_not() {
    for (arrangement, seed) in [(Arrangement::Strata, 31), (Arrangement::Blocks, 32)] {
        let generated = GeneratedAreas::generate(arrangement, seed);
        let areas = &generated.areas;
        let itself = areas.read([SIDE, SIDE], &generated.picture(SIDE), PAPER);
        let weak = *itself
            .contrasts
            .iter()
            .find(|c| c.weak())
            .expect("a weak contrast");
        let (a, b) = weak.areas;
        // All over a tenth lighter: every relation kept, nothing made strong.
        let light: Vec<Rgb8> = areas.colours.iter().map(|c| lighter(*c, 0.1)).collect();
        let read = areas.read([SIDE, SIDE], &painted(areas, &light, [0.0, 0.0]), PAPER);
        assert!(
            read.contrasts.iter().all(|c| !c.made_strong()),
            "{arrangement:?}"
        );
        // One of the weak pair pushed away from the other in value, where there is room: caught
        // there.
        let [va, vb] = [a, b].map(|area| hsv(areas.colours[area])[2]);
        let (moved, by) = match (vb >= va, vb + 0.22 <= 1.0, vb - 0.22 >= 0.0) {
            (true, true, _) => (b, 0.22),
            (true, false, _) => (a, -0.22),
            (false, _, true) => (b, -0.22),
            (false, _, false) => (a, 0.22),
        };
        let mut wide = areas.colours.clone();
        wide[moved] = lighter(wide[moved], by);
        let read = areas.read([SIDE, SIDE], &painted(areas, &wide, [0.0, 0.0]), PAPER);
        let pair = read
            .contrasts
            .iter()
            .find(|c| c.areas == (a, b))
            .expect("the pair");
        assert!(pair.made_strong(), "{arrangement:?}: {pair:?}");
        let elsewhere =
            (read.contrasts.iter()).filter(|c| c.areas.0 != moved && c.areas.1 != moved);
        assert!(
            elsewhere.into_iter().all(|c| !c.made_strong()),
            "{arrangement:?}"
        );
        let metrics = read.metrics(areas);
        assert!(value(&metrics, Metric::AreasMadeStrong) >= Some(1.0));
        assert!(value(&metrics, Metric::AreasDelta).is_some_and(|d| d > 0.0));
    }
}

#[test]
fn an_area_s_edges_run_as_smooth_lines_along_its_neighbours() {
    let generated = GeneratedAreas::generate(Arrangement::Strata, 4);
    let areas = &generated.areas;
    let (a, b) = areas.neighbours[0];
    let shared = areas.edges(a, Some(b));
    assert_eq!(shared.len(), 1, "two bands meet along one line");
    let line = &shared[0];
    assert!(line.len() > 100, "chained across the frame");
    let on_border = |p: &[f64; 2]| p.iter().any(|v| *v < 0.01 || *v > 0.99);
    let ends = [line[0], line[line.len() - 1]];
    assert!(ends.iter().all(on_border), "from edge to edge: {ends:?}");
    // Smoothed: no step turns a right angle.
    for w in line.windows(3) {
        let (u, v) = (
            [w[1][0] - w[0][0], w[1][1] - w[0][1]],
            [w[2][0] - w[1][0], w[2][1] - w[1][1]],
        );
        let dot = u[0] * v[0] + u[1] * v[1];
        assert!(dot > 0.0, "{w:?}");
    }
    // A patch inside the frame closes round itself; the frame's edge is never an edge.
    let patches = GeneratedAreas::generate(Arrangement::Patches, 9).areas;
    let inside = (0..patches.count()).find(|area| {
        let lines = patches.edges(*area, None);
        lines.len() == 1 && lines[0].first() == lines[0].last()
    });
    assert!(
        inside.is_some() || patches.count() <= 4,
        "a patch closed round"
    );
    for line in areas.edges(a, None) {
        assert!(
            line.iter()
                .all(|p| (0.001..0.999).contains(&p[1]) || (0.001..0.999).contains(&p[0]))
        );
    }
}
