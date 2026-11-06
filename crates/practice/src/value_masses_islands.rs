//! An answer's islands merged into the masses round them (`value_masses.rs`): a squinting eye sees
//! no detail under `ISLAND` of the picture as a mass of its own. Its cells of one value that touch
//! side by side are one mass; the smallest under the least size goes into the neighbour it shares
//! the longest border with (the larger, then the first, on a tie), which then joins any neighbour
//! of its own value it now touches; again until every mass is large enough or the picture is one.

use std::cmp::Reverse;
use std::collections::BTreeMap;

/// `values` (a map of `width` by `height` cells, row by row) with every mass under `least` cells
/// merged into its surround.
pub(crate) fn merge_islands(values: &mut [u8], [width, height]: [u32; 2], least: usize) {
    let (w, h) = (width as usize, height as usize);
    let (labels, count) = labelled(values, w, h);
    let mut masses = Masses {
        size: vec![0; count],
        value: vec![0; count],
        borders: vec![BTreeMap::new(); count],
        into: (0..count).collect(),
        alive: vec![true; count],
    };
    for (cell, label) in labels.iter().enumerate() {
        masses.size[*label] += 1;
        masses.value[*label] = values[cell];
    }
    for y in 0..h {
        for x in 0..w {
            let a = labels[y * w + x];
            let right = (x + 1 < w).then(|| labels[y * w + x + 1]);
            let below = (y + 1 < h).then(|| labels[(y + 1) * w + x]);
            for b in [right, below].into_iter().flatten().filter(|b| *b != a) {
                *masses.borders[a].entry(b).or_insert(0) += 1;
                *masses.borders[b].entry(a).or_insert(0) += 1;
            }
        }
    }
    while let Some(small) = (0..count)
        .filter(|m| masses.alive[*m] && masses.size[*m] < least)
        .min_by_key(|m| (masses.size[*m], *m))
    {
        let Some(into) = masses.widest_neighbour(small) else {
            break;
        };
        masses.absorb(small, into);
    }
    for (cell, label) in labels.iter().enumerate() {
        values[cell] = masses.value[masses.root(*label)];
    }
}

/// The masses as they merge: each one's cells, value, borders (neighbour and the cell sides
/// shared), the one it went into and whether it is still its own.
struct Masses {
    size: Vec<usize>,
    value: Vec<u8>,
    borders: Vec<BTreeMap<usize, usize>>,
    into: Vec<usize>,
    alive: Vec<bool>,
}

impl Masses {
    /// The neighbour `mass` shares the longest border with: the larger, then the first, on a tie.
    fn widest_neighbour(&self, mass: usize) -> Option<usize> {
        (self.borders[mass].iter())
            .max_by_key(|(n, shared)| (**shared, self.size[**n], Reverse(**n)))
            .map(|(n, _)| *n)
    }

    /// `small` merged into `into`, which then joins every neighbour of its value it touches now.
    fn absorb(&mut self, small: usize, into: usize) {
        let mut waiting = vec![small];
        while let Some(gone) = waiting.pop() {
            if !self.alive[gone] || gone == into {
                continue;
            }
            self.alive[gone] = false;
            self.into[gone] = into;
            self.size[into] += self.size[gone];
            let borders = std::mem::take(&mut self.borders[gone]);
            self.borders[into].remove(&gone);
            for (neighbour, shared) in borders {
                self.borders[neighbour].remove(&gone);
                if neighbour == into {
                    continue;
                }
                *self.borders[into].entry(neighbour).or_insert(0) += shared;
                *self.borders[neighbour].entry(into).or_insert(0) += shared;
                if self.value[neighbour] == self.value[into] {
                    waiting.push(neighbour);
                }
            }
        }
    }

    /// The mass `mass` went into at last.
    fn root(&self, mut mass: usize) -> usize {
        while self.into[mass] != mass {
            mass = self.into[mass];
        }
        mass
    }
}

/// Each cell's mass (cells of one value touching side by side), and how many masses there are.
pub(crate) fn labelled(values: &[u8], w: usize, h: usize) -> (Vec<usize>, usize) {
    let mut labels = vec![usize::MAX; values.len()];
    let mut count = 0;
    let mut stack = Vec::new();
    for start in 0..values.len() {
        if labels[start] != usize::MAX {
            continue;
        }
        labels[start] = count;
        stack.push(start);
        while let Some(cell) = stack.pop() {
            let (x, y) = (cell % w, cell / w);
            let near = [
                (x > 0).then(|| cell - 1),
                (x + 1 < w).then_some(cell + 1),
                (y > 0).then(|| cell - w),
                (y + 1 < h).then_some(cell + w),
            ];
            for next in near.into_iter().flatten() {
                if labels[next] == usize::MAX && values[next] == values[start] {
                    labels[next] = count;
                    stack.push(next);
                }
            }
        }
        count += 1;
    }
    (labels, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_island_goes_into_its_surround_and_a_large_one_stays() {
        // A 20 x 20 map of middle, a 2 x 2 dark island and a 6 x 6 light one.
        let (w, h) = (20, 20);
        let mut values = vec![1_u8; w * h];
        for (x, y) in (2..4).flat_map(|x| (2..4).map(move |y| (x, y))) {
            values[y * w + x] = 0;
        }
        for (x, y) in (10..16).flat_map(|x| (10..16).map(move |y| (x, y))) {
            values[y * w + x] = 2;
        }
        merge_islands(&mut values, [20, 20], 10);
        assert_eq!(values[2 * w + 2], 1, "the dark island is middle now");
        assert_eq!(values[12 * w + 12], 2, "the light one stays");
    }

    #[test]
    fn an_island_between_two_masses_of_one_value_joins_them() {
        // Dark left and dark right, a thin middle stripe between them: merged, all dark.
        let (w, h) = (21, 10);
        let mut values: Vec<u8> = (0..w * h).map(|at| u8::from(at % w == 10)).collect();
        merge_islands(&mut values, [21, 10], 20);
        assert!(values.iter().all(|v| *v == 0));
    }
}
