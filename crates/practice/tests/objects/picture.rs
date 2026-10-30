//! An object's picture (S94) against its answer. The picture's own edges, where the surface its
//! depth test shows changes between neighbouring pixels (a step of depth or of face), lie within a
//! pixel of the visible edges and outlines and nowhere else; on a flat face its shade never steps;
//! and every visible edge shows as a step of shade along its length.

use openpaint_practice::{ObjectTask, Point, distance, length, nearest, resample};

use crate::exact::objects;

/// The picture's side, as the app draws it; a step of shade between neighbours (composited on the
/// paper): half the least step across an edge, which antialiasing splits over two pairs.
const SIDE: usize = 400;
const STEP: f64 = 0.045;
/// How far an edge of the picture may lie off the answer (pixels), how near an edge a step of
/// shade is looked for, and how far from an edge's ends.
const NEAR: f64 = 1.0;
const SHOWN: f64 = 2.0;
const CLEAR: f64 = 6.0;

/// The pixel point `[x, y]` in frame units.
fn frame(p: Point) -> Point {
    [p[0] / SIDE as f64, p[1] / SIDE as f64]
}

/// The answer's visible lines, in pixels.
fn visible(task: &ObjectTask) -> Vec<Vec<Point>> {
    let seen = (task.answer.outlines.iter().flatten()).filter(|stretch| !stretch.hidden);
    (task.answer.sides.iter().map(|side| side.to_vec()))
        .chain(seen.map(|stretch| stretch.points.clone()))
        .map(|line| {
            line.iter()
                .map(|p| [p[0] * SIDE as f64, p[1] * SIDE as f64])
                .collect()
        })
        .collect()
}

/// How far the pixel point `p` lies from `lines`, in pixels.
fn off(lines: &[Vec<Point>], p: Point) -> f64 {
    let all: Vec<&[Point]> = lines.iter().map(Vec::as_slice).collect();
    nearest(&all, p).map_or(f64::INFINITY, |(_, d)| d)
}

/// The picture's shade per pixel, on the paper.
fn shades(task: &ObjectTask) -> Vec<f64> {
    (task.picture(SIDE as u32).chunks(4))
        .map(|p| {
            let luma = 0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]);
            let cover = f64::from(p[3]) / 255.0;
            luma / 255.0 * cover + (1.0 - cover)
        })
        .collect()
}

/// Each pair of neighbouring pixels, right and down: its two indices and its middle in pixels.
fn pairs() -> impl Iterator<Item = (usize, usize, Point)> {
    (0..SIDE).flat_map(|y| {
        (0..SIDE).flat_map(move |x| {
            let at = y * SIDE + x;
            let right = (x + 1 < SIDE).then(|| (at, at + 1, [x as f64 + 1.0, y as f64 + 0.5]));
            let down = (y + 1 < SIDE).then(|| (at, at + SIDE, [x as f64 + 0.5, y as f64 + 1.0]));
            right.into_iter().chain(down)
        })
    })
}

/// Along `lines`, every 4 pixels (`CLEAR` from their ends): how many places were looked at and
/// how many have a step of shade (`stepped`) within `SHOWN`.
fn shown_along(lines: &[Vec<Point>], stepped: &[bool]) -> (usize, usize) {
    let near_step = |p: Point| {
        let reach = SHOWN.ceil() as isize;
        (-reach..=reach).any(|dy| {
            (-reach..=reach).any(|dx| {
                let (x, y) = (p[0] as isize + dx, p[1] as isize + dy);
                (0..SIDE as isize).contains(&x)
                    && (0..SIDE as isize).contains(&y)
                    && stepped[y as usize * SIDE + x as usize]
                    && distance([x as f64 + 0.5, y as f64 + 0.5], p) <= SHOWN
            })
        })
    };
    let (mut samples, mut shown) = (0, 0);
    for line in lines {
        let long = length(line);
        let even = resample(line, 4.0);
        let mut walked = 0.0;
        for (k, p) in even.iter().enumerate() {
            walked += if k > 0 {
                distance(even[k - 1], *p)
            } else {
                0.0
            };
            if walked >= CLEAR && walked <= long - CLEAR {
                samples += 1;
                shown += usize::from(near_step(*p));
            }
        }
    }
    (samples, shown)
}

#[test]
fn the_picture_s_own_edges_lie_within_a_pixel_of_the_visible_edges_and_nowhere_else() {
    let (mut edges, mut worst, mut round_step) = (0usize, 0.0_f64, 0.0_f64);
    let (mut samples, mut shown) = (0usize, 0usize);
    for task in objects() {
        let lines = visible(&task);
        let surfaces: Vec<Option<(u32, bool)>> = (0..SIDE * SIDE)
            .map(|i| task.surface(frame([(i % SIDE) as f64 + 0.5, (i / SIDE) as f64 + 0.5])))
            .collect();
        let shade = shades(&task);
        let mut changed = vec![false; SIDE * SIDE];
        let mut stepped = vec![false; SIDE * SIDE];
        for (a, b, middle) in pairs() {
            if surfaces[a] != surfaces[b] {
                edges += 1;
                let far = off(&lines, middle);
                worst = worst.max(far);
                assert!(
                    far <= NEAR,
                    "{:?}: an edge of the picture {far:.2} px off",
                    task.carve
                );
                (changed[a], changed[b]) = (true, true);
            }
            if (shade[a] - shade[b]).abs() > STEP {
                (stepped[a], stepped[b]) = (true, true);
            }
        }
        // A step of shade away from every edge of the picture lies on a round part, turning.
        for (a, b, _) in pairs() {
            let step = (shade[a] - shade[b]).abs();
            if step > STEP && !changed[a] && !changed[b] {
                let round = surfaces[a].is_some_and(|(_, round)| round);
                assert!(round, "{:?}: a flat face steps", task.carve);
                round_step = round_step.max(step);
            }
        }
        let (looked, held) = shown_along(&lines, &stepped);
        (samples, shown) = (samples + looked, shown + held);
    }
    println!(
        "{edges} pixel pairs across the picture's edges, at most {worst:.2} px off the answer; \
         {shown} of {samples} samples along the visible edges by a step of shade; a round part's \
         turn steps at most {round_step:.3} a pixel"
    );
    assert_eq!(shown, samples, "every visible edge shows");
}
