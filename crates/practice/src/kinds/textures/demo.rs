//! Texture's demonstration (S154j), on an example strip of the exercise's own forms (its fixed
//! seed's), its picture in the original's frame: a form looked at and its outline drawn in pencil,
//! then crossed out (an outline is no texture); only the shadow it throws filled with the darkest;
//! every other shadow, nearest first (at 稳 the dense side's; then toward the light only the
//! biggest); Compare: the answer's shadows faint over the copy, a shadow the hand left out ringed.

use std::any::Any;
use std::sync::Arc;

use crate::demos::demo::{Demo, DemoLayout, Drawn, Marker, On, Pictured, Script, Shaded};
use crate::kinds::light_on_form::demo::{CELLS, bands};
use crate::kinds::textures::field::{Bumps, Texture};
use crate::kinds::textures::shadows::{TextureShadows, TextureTask};
use crate::kinds::tools::DARKEST;
use crate::maths::path::Point;
use crate::maths::random::derived;

/// The example's seed; the share of a form's shadow's reach its pencil outline lies off it.
const SEED: u64 = 0xDE30_5EED_0154;
const AROUND: f64 = 1.15;

/// The example strip's picture, square, the strip in its top half (the frame shows that half).
#[derive(Debug)]
struct Strip(TextureTask);

impl Pictured for Strip {
    fn picture(&self, side: u32) -> Vec<u8> {
        let mut pixels = self.0.picture(side);
        pixels.resize((side * side * 4) as usize, 0);
        pixels
    }

    fn same(&self, other: &dyn Any) -> bool {
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| other.0 == self.0)
    }
}

/// Texture's demonstration for `texture`'s forms.
pub(crate) fn texture(texture: Texture) -> Demo {
    let task = TextureTask::generate(
        Texture {
            from_right: false,
            ..texture
        },
        derived(SEED, 0),
    );
    let shadows = task.shadows([CELLS as u32, CELLS as u32 / 2]);
    let asked: Vec<usize> = (0..shadows.blobs.len())
        .filter(|k| shadows.asked[*k])
        .collect();
    // The one looked at: the largest; the one the hand leaves out: the smallest asked.
    let size = |k: &usize| shadows.blobs[*k].len();
    let largest = asked.iter().copied().max_by_key(size).unwrap_or(0);
    let left_out = asked.iter().copied().min_by_key(size).unwrap_or(0);
    let mut script = Script::new();
    script.given(On::Original, Marker::Given, Drawn::Shaded);
    script.step();
    crossed_out(&mut script, spread(&shadows, largest));
    script.step();
    script.together(filled(&shadows, &[largest], Marker::Ink));
    // The rest nearest first; at 稳 those toward the light (the left: the example's sun's side)
    // a step of their own.
    let sparse = texture.bumps == Bumps::Dense;
    let rest: Vec<usize> = (asked.iter().copied())
        .filter(|k| *k != largest && *k != left_out)
        .collect();
    let (first, then): (Vec<usize>, Vec<usize>) =
        (rest.into_iter()).partition(|k| !sparse || shadows.middle(*k)[0] >= 0.5);
    script.step();
    script.together(filled(&shadows, &first, Marker::Ink));
    if sparse {
        script.step();
        script.together(filled(&shadows, &then, Marker::Ink));
    }
    let mut compared = filled(&shadows, &asked, Marker::Faint);
    let missed = Drawn::Ring(shadows.middle(left_out));
    compared.push((On::Copy, Marker::Answer, missed));
    let mut demo = script.compare(DemoLayout::Pair, compared);
    demo.shaded = Some(Shaded(Arc::new(Strip(task))));
    demo.view = Some([[0.0, 0.0], [1.0, 0.5]]);
    demo
}

/// The blobs `ks` of `shadows` as bands of the darkest in `pen`, on the How's square grid.
fn filled(shadows: &TextureShadows, ks: &[usize], pen: Marker) -> Vec<(On, Marker, Drawn)> {
    let mut marks = Vec::new();
    for k in ks {
        let mut held = vec![false; CELLS * CELLS];
        for at in &shadows.blobs[*k] {
            held[*at] = true;
        }
        marks.extend((bands(&held, DARKEST).into_iter()).map(|drawn| (On::Copy, pen, drawn)));
    }
    marks
}

/// A form's outline round its shadow's middle and reach, drawn in pencil and crossed out.
fn crossed_out(script: &mut Script, (middle, reach): (Point, f64)) {
    let outline: Vec<Point> = (0..=32)
        .map(|k| {
            let turn = std::f64::consts::TAU * f64::from(k) / 32.0;
            let r = AROUND * reach;
            [middle[0] + r * turn.cos(), middle[1] + r * turn.sin()]
        })
        .collect();
    script.draw(On::Copy, Marker::Pencil, Drawn::Line(outline));
    script.draw(On::Copy, Marker::Pencil, Drawn::Cross(middle));
}

/// Blob `k`'s middle and how far it reaches from it (frame units).
fn spread(shadows: &TextureShadows, k: usize) -> (Point, f64) {
    let middle = shadows.middle(k);
    let unit = shadows.size[0].max(shadows.size[1]) as f64;
    let reach = (shadows.blobs[k].iter())
        .map(|at| {
            let p = [
                (at % shadows.size[0]) as f64 + 0.5,
                (at / shadows.size[0]) as f64 + 0.5,
            ];
            (p[0] / unit - middle[0]).hypot(p[1] / unit - middle[1])
        })
        .fold(0.0, f64::max);
    (middle, reach)
}
