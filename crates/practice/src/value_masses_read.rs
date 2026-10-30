//! A value-mass copy read against its answer (`value_masses.rs`, S96). The copy's frame is drawn
//! as the canvas shows it (on the compare worker), and each pixel over the picture is taken for
//! the grey it was painted in: the nearest of the three by lightness, or bare where it lies nearer
//! the paper's white. Where the copy's value differs from the answer's over more than a pen's
//! width (what is left of the differing pixels once a disc as wide as the pen is rolled inside
//! them: a morphological opening), the answer's value is hatched there in its grey; a mass edge
//! off by less than a pen's width is no difference. The share of the picture painted at the
//! answer's value is kept by the first Compare (`Metric::MassesMatched`), shown nowhere.

use crate::metric::{Metric, Metrics};
use crate::path::Point;
use crate::value_masses::{Value, ValueMasses};

/// The pen's width, frame units (5 canvas pixels on a 400-pixel frame): a difference narrower is
/// none.
pub const MASSES_PEN: f64 = 0.0125;
/// Between hatching's lines, frame units.
pub const HATCH_GAP: f64 = 0.02;
/// The shortest stretch of hatching kept, in the grid's steps.
const LEAST_HATCH: usize = 2;

/// A copy as read: the hatching (each stretch's ends in frame units, with the value the answer
/// holds there) and the share of the picture painted at the answer's value.
#[derive(Debug, Clone, PartialEq)]
pub struct MassesCheck {
    pub hatching: Vec<(Value, [Point; 2])>,
    pub matched: f64,
    pub metrics: Metrics,
}

impl ValueMasses {
    /// The copy whose frame's pixels are `rgba` (8-bit RGBA, `size` pixels, covering the frame
    /// exactly), the picture lying at `[top left, size]` in it (frame units).
    #[must_use]
    pub fn read(
        &self,
        (size, rgba): ([u32; 2], &[u8]),
        [origin, extent]: [Point; 2],
    ) -> MassesCheck {
        let unit = f64::from(size[0].max(1));
        let pixel = |v: f64| (v * unit).round().max(0.0) as usize;
        let [x0, y0] = origin.map(pixel);
        let x1 = pixel(origin[0] + extent[0]).min(size[0] as usize);
        let y1 = pixel(origin[1] + extent[1]).min(size[1] as usize);
        let [w, h] = [x1.saturating_sub(x0), y1.saturating_sub(y0)];
        let mut copy = Vec::with_capacity(w * h);
        let mut answer = Vec::with_capacity(w * h);
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let at = 4 * (y * size[0] as usize + x);
                let painted =
                    (rgba.get(at..at + 3)).and_then(|p| Value::painted([p[0], p[1], p[2]]));
                copy.push(painted);
                let share = |v: usize, from: f64, length: f64| {
                    ((v as f64 + 0.5) / unit - from) / length.max(f64::MIN_POSITIVE)
                };
                let p = [
                    share(x, origin[0], extent[0]),
                    share(y, origin[1], extent[1]),
                ];
                answer.push(self.at(p));
            }
        }
        let opened = differences(&copy, &answer, [w, h], MASSES_PEN / 2.0 * unit);
        let at = |[x, y]: Point| [(x0 as f64 + x) / unit, (y0 as f64 + y) / unit];
        let hatching = (hatched(&opened, &answer, [w, h], HATCH_GAP * unit).into_iter())
            .map(|(value, [a, b])| (value, [at(a), at(b)]))
            .collect();
        let same = (copy.iter().zip(&answer))
            .filter(|(c, a)| **c == Some(**a))
            .count();
        let matched = same as f64 / (w * h).max(1) as f64;
        MassesCheck {
            hatching,
            matched,
            metrics: vec![(Metric::MassesMatched, matched)],
        }
    }
}

/// Where `copy` (each step's value painted, none bare) differs from `answer` over a grid of
/// `[w, h]` steps, wider than a disc of `radius` steps: the differing steps opened by the disc.
pub(crate) fn differences(
    copy: &[Option<Value>],
    answer: &[Value],
    [w, h]: [usize; 2],
    radius: f64,
) -> Vec<bool> {
    let differ: Vec<bool> = (copy.iter().zip(answer))
        .map(|(c, a)| *c != Some(*a))
        .collect();
    let reach = radius.floor() as isize;
    let disc: Vec<[isize; 2]> = (-reach..=reach)
        .flat_map(|dy| (-reach..=reach).map(move |dx| [dx, dy]))
        .filter(|[dx, dy]| ((dx * dx + dy * dy) as f64) <= radius * radius)
        .collect();
    let (wi, hi) = (w as isize, h as isize);
    let near = |x: isize, y: isize, [dx, dy]: [isize; 2]| {
        let [nx, ny] = [(x + dx).clamp(0, wi - 1), (y + dy).clamp(0, hi - 1)];
        (ny * wi + nx) as usize
    };
    let mut kept = vec![false; differ.len()];
    for y in 0..hi {
        for x in 0..wi {
            kept[(y * wi + x) as usize] = disc.iter().all(|d| differ[near(x, y, *d)]);
        }
    }
    let mut opened = vec![false; differ.len()];
    for y in 0..hi {
        for x in 0..wi {
            if kept[(y * wi + x) as usize] {
                for d in &disc {
                    opened[near(x, y, *d)] = true;
                }
            }
        }
    }
    opened
}

/// The hatching over the `opened` steps of a grid of `[w, h]`: lines rising to the right `gap`
/// steps apart, each stretch over steps of one answer's value with that value; its ends in the
/// grid's steps (a step's middle at its index and a half).
pub(crate) fn hatched(
    opened: &[bool],
    answer: &[Value],
    [w, h]: [usize; 2],
    gap: f64,
) -> Vec<(Value, [Point; 2])> {
    let mut stretches = Vec::new();
    if w == 0 || h == 0 {
        return stretches;
    }
    let gap = gap.max(1.0);
    let mut line = 0.0;
    while line < (w + h) as f64 {
        let sum = line.round() as usize;
        let mut run: Option<(usize, Value)> = None;
        let first = sum.saturating_sub(h - 1);
        for x in first..=sum.min(w - 1) {
            let y = sum - x;
            let at = y * w + x;
            let here = opened[at].then_some(answer[at]);
            if let Some((start, value)) = run
                && here != Some(value)
            {
                push_stretch(&mut stretches, (start, x - 1, value), sum);
                run = None;
            }
            if let (None, Some(value)) = (run, here) {
                run = Some((x, value));
            }
        }
        if let Some((start, value)) = run {
            push_stretch(&mut stretches, (start, sum.min(w - 1), value), sum);
        }
        line += gap;
    }
    stretches
}

/// The stretch from step `from` to `to` across, on the line where across and down add to `sum`,
/// kept when at least `LEAST_HATCH` long.
fn push_stretch(
    stretches: &mut Vec<(Value, [Point; 2])>,
    (from, to, value): (usize, usize, Value),
    sum: usize,
) {
    if to + 1 < from + LEAST_HATCH {
        return;
    }
    let end = |x: usize| [x as f64 + 0.5, (sum - x) as f64 + 0.5];
    stretches.push((value, [end(from), end(to)]));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_difference_wider_than_the_pen_is_kept_and_a_thinner_one_is_not() {
        let (w, h) = (60, 40);
        // The copy differs over a band 2 steps wide (thinner than a disc of radius 2.5) and a
        // square 12 steps wide.
        let copy: Vec<Option<Value>> = (0..w * h)
            .map(|at| {
                let (x, y) = (at % w, at / w);
                let off = (10..12).contains(&x) || ((30..42).contains(&x) && (10..22).contains(&y));
                Some(if off { Value::Dark } else { Value::Light })
            })
            .collect();
        let answer = vec![Value::Light; w * h];
        let opened = differences(&copy, &answer, [w, h], 2.5);
        assert!(!opened[20 * w + 10], "the thin band is no difference");
        assert!(
            opened[15 * w + 35] && opened[11 * w + 31],
            "the square is, nearly to its corner"
        );
        let hatching = hatched(&opened, &answer, [w, h], 8.0);
        assert!(!hatching.is_empty());
        for (value, ends) in &hatching {
            assert_eq!(*value, Value::Light);
            for [x, y] in ends {
                assert!(
                    (30.0..42.0).contains(x) && (10.0..22.0).contains(y),
                    "{ends:?}"
                );
            }
        }
    }
}
