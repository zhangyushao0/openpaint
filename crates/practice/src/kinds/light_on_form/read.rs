//! A light-on-form copy read against the model (S153, D123: the app judges the families only). The
//! copy's frame is drawn as the canvas shows it (on the compare worker), and each pixel is taken
//! for the grey of the level it was painted in, the nearest by lightness, or bare nearer the
//! paper's white (`Grey::painted`, as value masses read theirs). Where its family differs from the
//! model's over more than the grey pens' width (the differing pixels opened by a disc as wide as
//! the pen, value masses' `wider`), it is hatched in the grey it should be: a soft terminator's band
//! takes either family. The families decide: accurate when nothing is hatched, almost while the
//! right family (painted, nothing hatched) covers `NEAR` of the form, its shadows and the ground
//! `STAKE` round them at least (where the families are at stake: the lit ground beyond, most of the
//! frame, reads right whatever is painted on the form), again below; incomplete while more than
//! `BARE_LEFT` of the frame is left bare. That share is kept by the first Compare
//! (`Metric::LightFamilies`). The inner
//! tones are help only (their borders are the model's choice): a halftone painted light, two faces
//! painted the wrong way round, a reflected light as dark as the core or the core lighter than it,
//! a contact not darkest, each hatched in the grey it should be, never moving the result.
//!
//! The reason (D121) names the family finding most off: the terminator painted towards the light
//! or the shadow on the form, or the cast shadow too long, too short or turned against the pin's
//! shadow, a little where the result is almost, clearly where again.

use std::collections::HashMap;

use crate::compositions::value_masses::Value;
use crate::kinds::light_on_form::answer::{Grey, Inner, Mass, Shading};
use crate::kinds::light_on_form::scene::{LightTask, Tones};
use crate::kinds::tools::DARKEST_PEN;
use crate::kinds::value_masses::value_masses_read::{
    BARE_LEFT, HATCH_GAP, VALUE_PEN, hatched, wider,
};
use crate::marks::card::{Arg, Say};
use crate::marks::compared::{Outcome, Verdict};
use crate::maths::path::Point;
use crate::records::metric::{Metric, Metrics};

/// The right family's share of the form and its shadows at which a copy whose families differ
/// somewhere is almost.
pub const NEAR: f64 = 0.9;
/// A copy's cast shadow under this share of the answer's is too short, wherever it lies.
const CAST_SHORT: f64 = 0.5;
/// How far round the form and its shadows the ground is at stake (frame units).
const STAKE: f64 = 0.08;
/// Two flat faces whose turns to the light differ by this are painted in their order.
const FACES_APART: f64 = 0.2;

/// A copy as read: the hatching (each stretch's ends in frame units with the grey it should be),
/// whether its families hold (nothing of them hatched), the share of the form and its shadows in
/// its right family (painted, not hatched), the share of the frame left bare over more than the
/// pen's width, the reason, and what the first Compare keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct LightCheck {
    pub hatching: Vec<(Grey, [Point; 2])>,
    pub families: bool,
    pub share: f64,
    pub bare: f64,
    pub reason: Option<Say>,
    pub metrics: Metrics,
}

impl LightCheck {
    /// Incomplete while more than `BARE_LEFT` is left bare; else the app's: accurate where the
    /// families hold, almost while `NEAR` of the form and its shadows is in its right family, else
    /// again.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        if self.bare > BARE_LEFT {
            return Verdict::Incomplete;
        }
        Verdict::App(self.outcome())
    }

    fn outcome(&self) -> Outcome {
        if self.families {
            Outcome::Clean
        } else if self.share >= NEAR {
            Outcome::Near
        } else {
            Outcome::Redo
        }
    }
}

impl LightTask {
    /// The copy whose frame's pixels are `rgba` (8-bit RGBA, `size` pixels, covering the frame).
    #[must_use]
    pub fn read(&self, (size, rgba): ([u32; 2], &[u8])) -> LightCheck {
        let [w, h] = size.map(|side| side as usize);
        let unit = f64::from(size[0].max(size[1]).max(1));
        let map = self.map(size);
        let tones = self.lit.tones.tones();
        let copy: Vec<Option<Grey>> = (0..w * h)
            .map(|at| {
                let pixel = rgba.get(4 * at..4 * at + 3)?;
                Grey::painted([pixel[0], pixel[1], pixel[2]], tones)
            })
            .collect();
        let pairs = || copy.iter().zip(&map);
        let wrong: Vec<bool> = pairs()
            .map(|(c, s)| matches!((c, s), (Some(c), Some(s)) if !s.soft && c.family() != s.family))
            .collect();
        let opened = wider(&wrong, [w, h], VALUE_PEN / 2.0 * unit);
        // Where the families are at stake: the form, the shadows and the ground `STAKE` round them
        // (the lit ground beyond reads right whatever was painted on the form).
        let held: Vec<bool> = (map.iter())
            .map(|s| s.is_some_and(|s| s.on_form || s.cast()))
            .collect();
        let stake = spread(&held, [w, h], (STAKE * unit).round() as usize);
        // Right where painted and not hatched: a difference narrower than the pen is none.
        let staked = || (copy.iter().zip(&opened).zip(&stake)).filter(|(_, at)| **at);
        let right = staked()
            .filter(|((c, wrong), _)| c.is_some() && !**wrong)
            .count();
        let share = right as f64 / staked().count().max(1) as f64;
        let bare: Vec<bool> = copy.iter().map(Option::is_none).collect();
        let bare = wider(&bare, [w, h], VALUE_PEN / 2.0 * unit)
            .iter()
            .filter(|b| **b)
            .count();
        let bare = bare as f64 / (w * h).max(1) as f64;
        let should: Vec<Grey> = map
            .iter()
            .map(|s| s.map_or(Grey::Light, |s| s.tone))
            .collect();
        let mut hatching = hatch(&opened, &should, [w, h], unit);
        let order = out_of_order(self.lit.tones, &copy, &map);
        let order: Vec<Option<Grey>> = (order.into_iter().zip(&opened))
            .map(|(tone, family)| tone.filter(|_| !family))
            .collect();
        hatching.extend(hatch_order(&order, [w, h], unit));
        let families = !opened.iter().any(|o| *o);
        let mut check = LightCheck {
            hatching,
            families,
            share,
            bare,
            reason: None,
            metrics: vec![(Metric::LightFamilies, share)],
        };
        if !families {
            let found = Found {
                map: &map,
                copy: &copy,
                opened: &opened,
                size: [w, h],
            };
            check.reason = Some(self.reason(&found, check.outcome() == Outcome::Near));
        }
        check
    }
}

/// `held` (a grid of `[w, h]`) spread `reach` steps every way: each step with one held within a
/// square of that reach about it.
fn spread(held: &[bool], [w, h]: [usize; 2], reach: usize) -> Vec<bool> {
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

/// The hatching over the `opened` pixels of a grid of `[w, h]` (`unit` to a frame unit), each
/// stretch in the grey `should` holds there.
fn hatch(
    opened: &[bool],
    should: &[Grey],
    [w, h]: [usize; 2],
    unit: f64,
) -> Vec<(Grey, [Point; 2])> {
    let at = |[x, y]: Point| [x / unit, y / unit];
    let value = |tone: Grey| match tone {
        Grey::Darkest | Grey::Dark => Value::Dark,
        Grey::Middle => Value::Middle,
        Grey::Light => Value::Light,
    };
    let tone = |value: Value| match value {
        Value::Dark => Grey::Dark,
        Value::Middle => Grey::Middle,
        Value::Light => Grey::Light,
    };
    let values: Vec<Value> = should.iter().map(|t| value(*t)).collect();
    let mut stretches = Vec::new();
    for darkest in [false, true] {
        let only: Vec<bool> = (opened.iter().zip(should))
            .map(|(o, t)| *o && (*t == Grey::Darkest) == darkest)
            .collect();
        for (value, ends) in hatched(&only, &values, [w, h], HATCH_GAP * unit) {
            let shown = if darkest { Grey::Darkest } else { tone(value) };
            stretches.push((shown, ends.map(at)));
        }
    }
    stretches
}

/// The inner tones' findings hatched, each opened by a disc as wide as the darkest pen.
fn hatch_order(order: &[Option<Grey>], [w, h]: [usize; 2], unit: f64) -> Vec<(Grey, [Point; 2])> {
    let mut stretches = Vec::new();
    for tone in Grey::ALL {
        let held: Vec<bool> = order.iter().map(|t| *t == Some(tone)).collect();
        let opened = wider(&held, [w, h], DARKEST_PEN / 2.0 * unit);
        if opened.iter().any(|o| *o) {
            let should = vec![tone; order.len()];
            stretches.extend(hatch(&opened, &should, [w, h], unit));
        }
    }
    stretches
}

/// Each pixel's inner tone out of order at `tones` (熟练, 稳), with the grey it should be: a
/// halftone painted light, a contact not darkest, a reflected light darkest; a flat face painted
/// darker than one turned less to the light; the core lighter than the reflected light.
fn out_of_order(tones: Tones, copy: &[Option<Grey>], map: &[Option<Shading>]) -> Vec<Option<Grey>> {
    let mut out: Vec<Option<Grey>> = vec![None; copy.len()];
    if tones == Tones::Two {
        return out;
    }
    let pairs = || copy.iter().zip(map).enumerate();
    for (at, (c, s)) in pairs() {
        let (Some(c), Some(s)) = (c, s) else { continue };
        out[at] = match s.inner {
            Inner::Halftone if *c == Grey::Light => Some(Grey::Middle),
            Inner::Contact if *c == Grey::Dark => Some(Grey::Darkest),
            Inner::Reflected if *c == Grey::Darkest => Some(Grey::Dark),
            _ => None,
        };
    }
    faces_out_of_order(copy, map, &mut out);
    let of = |inner: Inner| -> Vec<Grey> {
        (pairs())
            .filter_map(|(_, (c, s))| (s.is_some_and(|s| s.inner == inner)).then_some(*c)?)
            .collect()
    };
    if let (Some(core), Some(reflected)) =
        (majority(&of(Inner::Core)), majority(&of(Inner::Reflected)))
        && core > reflected
    {
        for (at, (c, s)) in pairs() {
            if s.is_some_and(|s| s.inner == Inner::Core) && c.is_some_and(|c| c > reflected) {
                out[at] = Some(Grey::Darkest);
            }
        }
    }
    out
}

/// Into `out`, each pixel of a lit flat face of the copy painted darker than a face turned less to
/// the light, each face as most of it is painted, with the grey it should be.
fn faces_out_of_order(copy: &[Option<Grey>], map: &[Option<Shading>], out: &mut [Option<Grey>]) {
    let mut faces: HashMap<u32, (f64, Vec<Grey>)> = HashMap::new();
    for (c, s) in copy.iter().zip(map) {
        if let (Some(c), Some(s)) = (c, s)
            && s.on_form
            && !s.round
            && s.family == Mass::Light
        {
            faces
                .entry(s.surface)
                .or_insert((s.turned, Vec::new()))
                .1
                .push(*c);
        }
    }
    let most: Vec<(u32, f64, Grey)> = (faces.into_iter())
        .filter_map(|(surface, (turned, tones))| Some((surface, turned, majority(&tones)?)))
        .collect();
    for (surface, turned, tone) in &most {
        let darker = (most.iter())
            .filter(|(_, other, its)| *turned >= other + FACES_APART && its > tone)
            .map(|(_, _, its)| *its)
            .max();
        let Some(brighter) = darker else { continue };
        for (at, (c, s)) in copy.iter().zip(map).enumerate() {
            if let (Some(c), Some(s)) = (c, s)
                && s.surface == *surface
                && *c < brighter
            {
                out[at] = Some(s.tone);
            }
        }
    }
}

/// The tone most of `tones` hold (the darker on a tie); none for none.
fn majority(tones: &[Grey]) -> Option<Grey> {
    let count = |tone: &Grey| tones.iter().filter(|t| *t == tone).count();
    (Grey::ALL.iter())
        .filter(|tone| count(tone) > 0)
        .max_by_key(|tone| (count(tone), std::cmp::Reverse(**tone)))
        .copied()
}

/// What a reason is read from: the model and the copy at each pixel, where the families differ
/// opened, and the grid's size.
struct Found<'a> {
    map: &'a [Option<Shading>],
    copy: &'a [Option<Grey>],
    opened: &'a [bool],
    size: [usize; 2],
}

impl LightTask {
    /// The reason naming the family finding most off, a little where `near`.
    fn reason(&self, found: &Found<'_>, near: bool) -> Say {
        let [w, _] = found.size;
        let unit = found.size[0].max(found.size[1]) as f64;
        let cells = || (found.map.iter().zip(found.copy).zip(found.opened)).enumerate();
        let (mut darker, mut lighter, mut ground) = (0_usize, 0_usize, 0_usize);
        for (_, ((s, c), opened)) in cells() {
            let (Some(s), Some(c), true) = (s, c, opened) else {
                continue;
            };
            match (s.on_form, s.family, c.family()) {
                (false, _, _) => ground += 1,
                (true, Mass::Light, Mass::Shadow) => darker += 1,
                (true, _, _) => lighter += 1,
            }
        }
        let what = if darker + lighter >= ground {
            if darker >= lighter {
                "terminator-light"
            } else {
                "terminator-shadow"
            }
        } else {
            // The copy's cast shadow's middle against the answer's, along the pin's shadow, and
            // how much of it there is.
            let middle = |cast: &dyn Fn(&Option<Shading>, &Option<Grey>) -> bool| {
                let (mut sum, mut count) = ([0.0; 2], 0.0);
                for (at, ((s, c), _)) in cells() {
                    if cast(s, c) {
                        sum = [
                            sum[0] + (at % w) as f64 / unit,
                            sum[1] + (at / w) as f64 / unit,
                        ];
                        count += 1.0;
                    }
                }
                (sum.map(|v| v / f64::max(count, 1.0)), count)
            };
            let answer = middle(&|s, _| s.is_some_and(|s| s.cast()));
            let copy = middle(&|s, c| {
                s.is_some_and(|s| !s.on_form) && c.is_some_and(|c| c.family() == Mass::Shadow)
            });
            self.cast_off(answer, copy)
        };
        let degree = if near { "little" } else { "clearly" };
        Say {
            key: "light-off",
            args: vec![Arg::Word(what), Arg::Word(degree)],
        }
    }

    /// Which way the copy's cast shadow (its middle and how many pixels) lies off the answer's:
    /// too short where it holds under `CAST_SHORT` of it, else by its middle along the pin's
    /// shadow (too long or too short) or across it (turned).
    fn cast_off(&self, (answer, all): (Point, f64), (copy, held): (Point, f64)) -> &'static str {
        if held < CAST_SHORT * all {
            return "cast-short";
        }
        let [foot, _, tip] = self.pinned;
        let way = [tip[0] - foot[0], tip[1] - foot[1]];
        let long = way[0].hypot(way[1]).max(f64::MIN_POSITIVE);
        let off = [copy[0] - answer[0], copy[1] - answer[1]];
        let along = (off[0] * way[0] + off[1] * way[1]) / long;
        let across = (off[1] * way[0] - off[0] * way[1]) / long;
        match (along.abs() >= across.abs(), along > 0.0) {
            (true, true) => "cast-long",
            (true, false) => "cast-short",
            (false, _) => "cast-turned",
        }
    }
}
