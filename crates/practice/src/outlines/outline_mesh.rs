//! The ground near the object (`outline_flood.rs`): the smooth ground's colour on a coarse mesh,
//! cells of `CELL` grid pixels each the mean of the smooth ground in it, the cells the object
//! covers filled from those round them as a stretched membrane would (Laplace's equation, by
//! relaxation), read back between the cells' middles. So a halo of light behind the object or a
//! sweep's bend, which no model read from the picture's edge holds, is the ground's here too.

/// A mesh cell's side, in grid pixels, and the share of it the smooth ground must cover to read
/// the cell.
const CELL: usize = 16;
const READ: f64 = 0.25;
/// Relaxation rounds filling the covered cells.
const ROUNDS: usize = 400;

/// The ground's Oklab colour at each grid pixel, from the colours `lab` where `ground` holds.
pub(crate) fn mesh(
    [width, height]: [usize; 2],
    lab: &[[f64; 3]],
    ground: &[bool],
) -> Vec<[f64; 3]> {
    let [across, down] = [width.div_ceil(CELL), height.div_ceil(CELL)];
    let mut sums = vec![([0.0f64; 3], 0usize, 0usize); across * down];
    for (i, colour) in lab.iter().enumerate() {
        let cell = (i / width / CELL) * across + (i % width) / CELL;
        sums[cell].2 += 1;
        if ground[i] {
            for (s, c) in sums[cell].0.iter_mut().zip(colour) {
                *s += c;
            }
            sums[cell].1 += 1;
        }
    }
    let known: Vec<bool> = (sums.iter())
        .map(|(_, read, all)| *read as f64 >= READ * *all as f64 && *read > 0)
        .collect();
    let mut cells: Vec<[f64; 3]> = (sums.iter())
        .map(|(sum, read, _)| sum.map(|s| s / (*read).max(1) as f64))
        .collect();
    let mean = mean_of(&cells, &known);
    for (cell, known) in cells.iter_mut().zip(&known) {
        if !known {
            *cell = mean;
        }
    }
    relax(&mut cells, &known, [across, down]);
    (0..width * height)
        .map(|i| between(&cells, [across, down], [i % width, i / width]))
        .collect()
}

/// The mean of the known cells (grey when none is).
fn mean_of(cells: &[[f64; 3]], known: &[bool]) -> [f64; 3] {
    let count = known.iter().filter(|k| **k).count();
    if count == 0 {
        return [0.5, 0.0, 0.0];
    }
    let mut sum = [0.0; 3];
    for (cell, _) in cells.iter().zip(known).filter(|(_, k)| **k) {
        for (s, c) in sum.iter_mut().zip(cell) {
            *s += c;
        }
    }
    sum.map(|s| s / count as f64)
}

/// The unknown cells relaxed towards the mean of their neighbours, the known held.
fn relax(cells: &mut [[f64; 3]], known: &[bool], [across, down]: [usize; 2]) {
    for _ in 0..ROUNDS {
        let before = cells.to_vec();
        for i in (0..cells.len()).filter(|&i| !known[i]) {
            let (x, y) = (i % across, i / across);
            let neighbours = [
                (x > 0).then(|| i - 1),
                (x + 1 < across).then(|| i + 1),
                (y > 0).then(|| i - across),
                (y + 1 < down).then(|| i + across),
            ];
            let mut sum = [0.0; 3];
            let mut count = 0.0;
            for n in neighbours.into_iter().flatten() {
                for (s, c) in sum.iter_mut().zip(before[n]) {
                    *s += c;
                }
                count += 1.0;
            }
            cells[i] = sum.map(|s| s / count);
        }
    }
}

/// The mesh read at grid pixel `(x, y)`, bilinearly between the cells' middles.
fn between(cells: &[[f64; 3]], [across, down]: [usize; 2], [x, y]: [usize; 2]) -> [f64; 3] {
    let place = |p: usize, cells: usize| {
        let at = ((p as f64 + 0.5) / CELL as f64 - 0.5).clamp(0.0, (cells - 1) as f64);
        let low = at.floor() as usize;
        (low, (low + 1).min(cells - 1), at - low as f64)
    };
    let (x0, x1, tx) = place(x, across);
    let (y0, y1, ty) = place(y, down);
    let cell = |cx: usize, cy: usize| cells[cy * across + cx];
    let mut out = [0.0; 3];
    for (c, value) in out.iter_mut().enumerate() {
        let top = cell(x0, y0)[c] * (1.0 - tx) + cell(x1, y0)[c] * tx;
        let bottom = cell(x0, y1)[c] * (1.0 - tx) + cell(x1, y1)[c] * tx;
        *value = top * (1.0 - ty) + bottom * ty;
    }
    out
}
