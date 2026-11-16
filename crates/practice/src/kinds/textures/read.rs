//! A texture copy read against the answer's shadows (S154j), as value masses read theirs: each
//! pixel of the copy's frame dark (the darkest pen's, nearest it by lightness) or bare. A shadow of
//! the answer is matched where the copy's dark covers half of it; the copy's dark lying off the
//! shadow family grown by a pen's width is a mark over light (an outline drawn round a form, a
//! shadow invented), each connected part of it a pen's dab or larger: a shadow the answer leaves
//! out (a sliver, at 稳 a small one toward the light) painted is no mark. Both are direct readings
//! of a generated scene, but what to leave out is the learner's (Drawabox: fewer toward the
//! light), so they are marks and the eye judges (D123).

use crate::colour_areas::areas::{Areas, GRID};
use crate::kinds::light_on_form::answer::Grey;
use crate::kinds::textures::shadows::{TextureShadows, TextureTask, parts};
use crate::kinds::tools::{DARKEST, DARKEST_PEN};
use crate::maths::path::Point;

/// A shadow is matched where the copy covers this share of it.
const MATCHED: f64 = 0.5;
/// The least mark over light, a share of a pen's square.
const DAB: f64 = 0.5;

/// A copy as read: the middles of the shadows it missed and the outlines of its marks over light
/// (frame units), how many of the answer's shadows it matched of how many, how many marks lay over
/// light.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureCheck {
    pub missed: Vec<Point>,
    pub over: Vec<Vec<Point>>,
    pub matched: usize,
    pub shadows: usize,
    pub marks_over: usize,
}

impl TextureTask {
    /// The copy whose frame's pixels are `rgba` (8-bit RGBA, `size` pixels, covering the frame),
    /// and the answer's shadows as a picture of that size (the darkest where asked, clear
    /// elsewhere).
    #[must_use]
    pub fn read(&self, (size, rgba): ([u32; 2], &[u8])) -> (TextureCheck, Vec<u8>) {
        let shadows = self.shadows(size);
        let [w, h] = shadows.size;
        let unit = w.max(h) as f64;
        let dark: Vec<bool> = (0..w * h)
            .map(|at| {
                rgba.get(4 * at..4 * at + 3)
                    .and_then(|p| Grey::painted([p[0], p[1], p[2]], &[Grey::Darkest]))
                    .is_some()
            })
            .collect();
        let answer = shadows.answer();
        let asked = (shadows.blobs.iter().zip(&shadows.asked)).filter(|(_, asked)| **asked);
        let mut missed = Vec::new();
        let (mut matched, mut count) = (0, 0);
        for (k, (blob, _)) in asked.enumerate() {
            count += 1;
            let covered = blob.iter().filter(|at| dark[**at]).count();
            if covered as f64 >= MATCHED * blob.len() as f64 {
                matched += 1;
            } else {
                missed.push(k);
            }
        }
        let missed: Vec<Point> = {
            let asked: Vec<usize> = (0..shadows.blobs.len())
                .filter(|k| shadows.asked[*k])
                .collect();
            missed
                .into_iter()
                .map(|k| shadows.middle(asked[k]))
                .collect()
        };
        let grown = grown(
            &shadows.shadow,
            [w, h],
            (DARKEST_PEN * unit).round() as usize,
        );
        let off: Vec<bool> = dark.iter().zip(&grown).map(|(d, g)| *d && !*g).collect();
        let least = DAB * (DARKEST_PEN * unit).powi(2);
        let over: Vec<Vec<usize>> = (parts(&off, [w, h]).into_iter())
            .filter(|mark| mark.len() as f64 >= least)
            .collect();
        let check = TextureCheck {
            missed,
            over: outlines(&over, &shadows),
            matched,
            shadows: count,
            marks_over: over.len(),
        };
        let picture = (answer.iter())
            .flat_map(|held| {
                let [r, g, b] = DARKEST;
                [r, g, b, if *held { 255 } else { 0 }]
            })
            .collect();
        (check, picture)
    }
}

/// `held` (a grid `[w, h]`) grown `reach` steps every way: each step with one held within a
/// square of that reach about it.
fn grown(held: &[bool], [w, h]: [usize; 2], reach: usize) -> Vec<bool> {
    let pass = |source: &[bool], across: bool| -> Vec<bool> {
        (0..w * h)
            .map(|at| {
                let (x, y) = (at % w, at / w);
                let (here, length) = if across { (x, w) } else { (y, h) };
                let from = here.saturating_sub(reach);
                let to = (here + reach).min(length.saturating_sub(1));
                (from..=to).any(|k| source[if across { y * w + k } else { k * w + x }])
            })
            .collect()
    };
    pass(&pass(held, true), false)
}

/// The outlines of `marks` (pixels of `shadows`' grid) in frame units, drawn round the cells of a
/// map `GRID` across that they touch.
fn outlines(marks: &[Vec<usize>], shadows: &TextureShadows) -> Vec<Vec<Point>> {
    if marks.is_empty() {
        return Vec::new();
    }
    let [w, h] = shadows.size;
    let cells = [GRID, (GRID * h).div_ceil(w).clamp(1, GRID)];
    let mut map = vec![0_u8; cells[0] * cells[1]];
    for at in marks.iter().flatten() {
        let (x, y) = (at % w, at / w);
        map[(y * cells[1] / h) * cells[0] + x * cells[0] / w] = 1;
    }
    let unit = w.max(h) as f64;
    let extent = [w as f64 / unit, h as f64 / unit];
    Areas::of_map(cells, map, vec![[255; 3], DARKEST])
        .map(|areas| areas.edges_in_frame(1, None, extent))
        .unwrap_or_default()
}
