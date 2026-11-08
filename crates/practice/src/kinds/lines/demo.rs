//! A single line's demonstration (`demos/demo.rs`): its ends and where it bulges most ticked, two
//! passes in the air, one quick stroke, then compared.

use crate::demos::demo::{
    Demo, Drawn, Layout, Marker, On, Script, TICK, clearest, off_middle, tick, wobbled,
};
use crate::kinds::lines::line::{LineExercise, LineTier};
use crate::maths::path::{Point, distance};
use crate::sessions::plan::Family;

/// The example's seed: never an exercise's but by a chance of one in 2^64.
const LINE_SEED: u64 = 0xDE30_5EED_0001;

/// A single line.
pub(crate) fn line() -> Demo {
    // A long arc near the frame's middle, plainly bent.
    let original = clearest(
        (LINE_SEED, 32),
        |seed| LineExercise::generate(LineTier::Arc, seed).points,
        |line| {
            let chord = line
                .first()
                .zip(line.last())
                .map_or(0.0, |(a, b)| distance(*a, *b));
            let bent = depth(line).min(0.12);
            chord.min(0.7) + 2.0 * bent - off_middle(line)
        },
    );
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, Drawn::Line(original.clone()));
    script.step();
    let n = original.len();
    let bulge = bulges(&original).into_iter().map(|i| (i, i + 1));
    for (at, towards) in [(0, 1), (n - 1, n - 2)].into_iter().chain(bulge) {
        let mark = tick(original[at], original[towards.min(n - 1)], TICK);
        script.draw(On::Copy, Marker::Pencil, Drawn::Line(mark));
    }
    script.step();
    for phase in [0.4, 2.1] {
        let pass = wobbled(&original, 0.012, phase);
        script.draw(On::Copy, Marker::Air, Drawn::Line(pass));
    }
    script.step();
    let copy = wobbled(&original, 0.03, 0.5);
    script.draw(On::Copy, Marker::Ink, Drawn::Line(copy));
    let red = vec![(On::Copy, Marker::Answer, Drawn::Line(original))];
    script.compare(Family::Line, Layout::Pair, red)
}

/// How far `line` lies from the chord between its ends at most.
fn depth(line: &[Point]) -> f64 {
    let (Some(&a), Some(&b)) = (line.first(), line.last()) else {
        return 0.0;
    };
    let chord = [b[0] - a[0], b[1] - a[1]];
    let long = chord[0].hypot(chord[1]).max(f64::MIN_POSITIVE);
    (line.iter())
        .map(|p| (((p[0] - a[0]) * chord[1] - (p[1] - a[1]) * chord[0]) / long).abs())
        .fold(0.0, f64::max)
}

/// Where `line` lies farthest from the chord between its ends, on either side, when it does so
/// plainly (its most bulging points, V大's 最鼓处).
fn bulges(line: &[Point]) -> Vec<usize> {
    let (Some(&a), Some(&b)) = (line.first(), line.last()) else {
        return Vec::new();
    };
    let chord = [b[0] - a[0], b[1] - a[1]];
    let long = chord[0].hypot(chord[1]).max(f64::MIN_POSITIVE);
    let side = |p: Point| ((p[0] - a[0]) * chord[1] - (p[1] - a[1]) * chord[0]) / long;
    let mut found = Vec::new();
    for sign in [1.0, -1.0] {
        let far =
            (0..line.len())
                .map(|i| (i, sign * side(line[i])))
                .fold(
                    (0, 0.0),
                    |best, here| if here.1 > best.1 { here } else { best },
                );
        if far.1 > 0.04 && far.0 + 1 < line.len() {
            found.push(far.0);
        }
    }
    found
}
