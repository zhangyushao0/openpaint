//! Texture's picture and answer (S154j). The picture is the lit picture (`lit_picture`), clay
//! tinted: the lit sides by their turn to the sun (form shading, soft), the shadow family dark and
//! hard-edged, every halftone a step lighter than the shadow at least, so the eye tells shading
//! from shadow. The shadow family (a form turned away, or its ray to the sun blocked) is a mask;
//! the answer is its blobs, its connected parts once those narrower than `WIDE` pens are opened
//! away (a sliver along a plate's edge is no shadow to paint). At 稳 the half toward the light
//! keeps only the biggest third of its blobs: the answer itself sparse there.

use crate::kinds::objects::object_picture::lit_picture;
use crate::kinds::textures::field::{Bumps, Texture, tried, tries};
use crate::kinds::textures::rays::Strip;
use crate::kinds::tools::DARKEST_PEN;
use crate::kinds::value_masses::value_masses_read::wider;
use crate::maths::path::Point;

/// The shadow's shade, the lit sides' least and how much their turn to the sun adds (0 black, 1
/// white).
const SHADOW: f64 = 0.3;
const LIT: f64 = 0.5;
const TURN: f64 = 0.75;
/// The clay's tint.
const TINT: [f64; 3] = [1.0, 0.965, 0.92];
/// The grid the rules read the shadows on, across the strip; the most and least shadows a strip
/// holds.
const RULES: u32 = 200;
const FEWEST: usize = 12;
const MOST: usize = 30;
/// Every shadow of the answer at least this many pens wide.
const WIDE: f64 = 1.0;

/// A strip of small forms to tell by their shadows, generated the same for the same variant and
/// seed.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureTask {
    pub texture: Texture,
    pub(crate) strip: Strip,
}

/// The answer's shadows on a grid of `size` pixels over the strip: the shadow family's mask, each
/// blob's pixels (row by row) and whether the answer asks for it (all but at 稳 the small ones
/// toward the light).
#[derive(Debug, Clone, PartialEq)]
pub struct TextureShadows {
    pub size: [usize; 2],
    pub shadow: Vec<bool>,
    pub blobs: Vec<Vec<usize>>,
    pub asked: Vec<bool>,
}

impl TextureShadows {
    /// Blob `k`'s middle (frame units).
    #[must_use]
    pub fn middle(&self, k: usize) -> Point {
        let unit = self.size[0].max(self.size[1]) as f64;
        let [mut x, mut y] = [0.0, 0.0];
        for at in &self.blobs[k] {
            x += (at % self.size[0]) as f64 + 0.5;
            y += (at / self.size[0]) as f64 + 0.5;
        }
        let count = self.blobs[k].len().max(1) as f64;
        [x / count / unit, y / count / unit]
    }

    /// Each pixel whether the answer asks for its shadow.
    #[must_use]
    pub fn answer(&self) -> Vec<bool> {
        let mut held = vec![false; self.size[0] * self.size[1]];
        for (blob, asked) in self.blobs.iter().zip(&self.asked) {
            for at in blob.iter().filter(|_| *asked) {
                held[*at] = true;
            }
        }
        held
    }
}

impl TextureTask {
    /// The task `texture` and `seed` make.
    #[must_use]
    pub fn generate(texture: Texture, seed: u64) -> Self {
        let mut made = Self {
            texture,
            strip: tried(texture, seed, 0),
        };
        for k in 1..tries() {
            if made.keeps() {
                break;
            }
            made.strip = tried(texture, seed, k);
        }
        made
    }

    /// The surface the eye sees at the frame point `p`, its shade, and whether it is lit.
    pub(crate) fn shade(&self, p: Point) -> Option<(u32, f64, bool)> {
        let seen = self.strip.seen(p)?;
        let lit = self.strip.lit(&seen);
        let turned = crate::maths::vector::dot(seen.normal, self.strip.sun);
        let shade = if lit {
            (LIT + TURN * turned).min(1.0)
        } else {
            SHADOW
        };
        Some((seen.surface * 2 + u32::from(lit), shade, lit))
    }

    /// The strip's picture, `side` pixels wide and half as high (RGBA, opaque).
    #[must_use]
    pub fn picture(&self, side: u32) -> Vec<u8> {
        let surface = |p: Point| self.shade(p).map(|(id, shade, _)| (id, shade));
        let square = lit_picture((side, [[0.0, 0.0], [1.0, 0.5]], TINT), surface);
        let rows = (side / 2) as usize * side as usize * 4;
        square[..rows].to_vec()
    }

    /// The answer's shadows on a grid `size` pixels over the strip.
    #[must_use]
    pub fn shadows(&self, [w, h]: [u32; 2]) -> TextureShadows {
        let size = [w.max(1) as usize, h.max(1) as usize];
        let unit = size[0].max(size[1]) as f64;
        let middle = |v: usize| (v as f64 + 0.5) / unit;
        let dark: Vec<bool> = (0..size[0] * size[1])
            .map(|at| {
                let p = [middle(at % size[0]), middle(at / size[0])];
                self.shade(p).is_some_and(|(_, _, lit)| !lit)
            })
            .collect();
        let opened = wider(&dark, size, WIDE / 2.0 * DARKEST_PEN * unit);
        let blobs = parts(&opened, size);
        let mut shadows = TextureShadows {
            size,
            shadow: dark,
            asked: vec![true; blobs.len()],
            blobs,
        };
        if self.texture.bumps == Bumps::Dense {
            self.sparse(&mut shadows);
        }
        shadows
    }

    /// `shadows` toward the light kept to the biggest third of them.
    fn sparse(&self, shadows: &mut TextureShadows) {
        let right = self.texture.from_right;
        let mut lit_side: Vec<usize> = (0..shadows.blobs.len())
            .filter(|k| (shadows.middle(*k)[0] >= 0.5) == right)
            .collect();
        lit_side.sort_by_key(|k| std::cmp::Reverse(shadows.blobs[*k].len()));
        for k in lit_side.iter().skip(lit_side.len().div_ceil(3)) {
            shadows.asked[*k] = false;
        }
    }

    /// Whether it keeps every rule: `FEWEST` to `MOST` shadows (each `WIDE` pens wide, as the
    /// answer opens them), none lying along the frame's edge over half its outline.
    pub(crate) fn keeps(&self) -> bool {
        let shadows = self.shadows([RULES, RULES / 2]);
        let grid = [RULES as usize, RULES as usize / 2];
        let inside = (shadows.blobs.iter()).all(|blob| !along_edge(blob, grid));
        (FEWEST..=MOST).contains(&shadows.blobs.len()) && inside
    }
}

/// Whether more than half the outline of `blob` (pixels of a grid `[w, h]`) lies on the grid's
/// edge.
fn along_edge(blob: &[usize], [w, h]: [usize; 2]) -> bool {
    let held: std::collections::HashSet<usize> = blob.iter().copied().collect();
    let (mut outline, mut edge) = (0usize, 0usize);
    for at in blob {
        let (x, y) = (at % w, at / w);
        let on_edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
        let inner = [(x > 0).then(|| at - 1), (x + 1 < w).then(|| at + 1)]
            .into_iter()
            .chain([(y > 0).then(|| at - w), (y + 1 < h).then(|| at + w)])
            .all(|n| n.is_some_and(|n| held.contains(&n)));
        if on_edge || !inner {
            outline += 1;
            edge += usize::from(on_edge);
        }
    }
    2 * edge > outline
}

/// The connected parts of `held` (a grid `[w, h]`, neighbours of eight), each its pixels.
pub(crate) fn parts(held: &[bool], [w, h]: [usize; 2]) -> Vec<Vec<usize>> {
    let mut seen = vec![false; held.len()];
    let mut parts = Vec::new();
    for start in 0..held.len() {
        if !held[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        let (mut part, mut front) = (Vec::new(), vec![start]);
        while let Some(at) = front.pop() {
            part.push(at);
            let (x, y) = ((at % w) as isize, (at / w) as isize);
            for (dx, dy) in (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (dx, dy))) {
                let [nx, ny] = [x + dx, y + dy];
                if !(0..w as isize).contains(&nx) || !(0..h as isize).contains(&ny) {
                    continue;
                }
                let n = ny as usize * w + nx as usize;
                if held[n] && !seen[n] {
                    seen[n] = true;
                    front.push(n);
                }
            }
        }
        part.sort_unstable();
        parts.push(part);
    }
    parts
}
