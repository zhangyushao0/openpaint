//! The stages' sessions, each a warm-up from the kinds already met, then the stage's mix.
//!
//! Stage 1 (V大's week 1): six single lines (one near-straight, then bends of every kind), four
//! groups of combined lines (each arrangement once) and six colours (four low in saturation, every
//! lightness). After S75's estimates (a line about 0.8 minutes, a colour about 1.75) and a group of
//! combined lines taken at about 2: the mix about 23 minutes, the warm-up of four about 5, so a
//! session from the second on is about 28 and the first about 23. The mix keeps a session's first
//! colour where S75's sheet had it (the eleventh pair). Its warm-up takes the stage's families met
//! in turn (lines, combined lines, colours).
//!
//! Stage 2 (V大's week 2): four single shapes, the main exercise, then week 2's colour work, a
//! colour combination and a value scale, then its perspective in turn by the session's number:
//! a box to complete and an ellipse in the first, third, fifth session, a page of free boxes in
//! the second, fourth, sixth (the box two-point, then three-point; the ellipse in a plain frame,
//! then in perspective; the free boxes' horizon in the middle, high, then low). Lines, groups and
//! colours live on in the warm-up. The shapes' tiers follow the session's number: entry and
//! simple first, medium from the third session, hard from the sixth; the user's own pictures take
//! the last place once there are some of a tier the session reaches (its hardest or an easier
//! one, the tier the user chose for each). A shape taken at about 3.5 minutes (a line drawing,
//! basically right, compared and corrected once), a combination at about 4 (the tone, the middle
//! tones, the edges with a big brush), a value scale at about 3, a box to complete at about 2.5
//! (nine edges, ghosted, extended and corrected), an ellipse at about 1, a page of three or four
//! free boxes at about 3: the mix about 24 to 24.5 minutes. Its warm-up is three exercises, at
//! most one of them long (a shape, a combination, a scale or free boxes), so it stays about 5 to
//! 6.5 minutes and a session about 29 to 31: the long families met take the long place in turn
//! by the session's number, the quick ones (lines, groups, colours, boxes, ellipses) the others in
//! turn from a family the number moves on, so over sessions every kind met comes back.
//!
//! The warm-up's exercises are kinds met, each chosen by the seed among its family's; a shape in
//! it a picture of its tier, a combination a crop. A plan's shapes carry their pictures, picked by
//! the seed (`pictures.rs`), its divisions and whole copies compositions (S95), its combinations
//! crops of real pictures (S101: none without a pack holding crops). Stage 3's sessions:
//! `session_three.rs`; stage 4's: `session_four.rs`.

use std::collections::HashSet;

use crate::colour_match::{Lightness, Purity, Tone};
use crate::combined::Combination;
use crate::ellipse::Guide;
use crate::line::LineTier;
use crate::pack::{ItemRef, Tier};
use crate::perspective::{BoxView, Horizon};
use crate::pictures::Pictures;
use crate::plan::{Exercise, Family, Kind};
use crate::random::{Random, derived};
use crate::scale::{CELLS, FIRST_CELLS, FIRST_SCALES, ScaleTone};
use crate::session_four::stage_four;
use crate::session_three::stage_three;

/// Exercises a stage-1 warm-up holds.
pub const WARM_UP: usize = 4;
/// Exercises a stage-2 or stage-3 warm-up holds, at most one of them long.
pub const WARM_UP_TWO: usize = 3;

/// Stage 1's mix: lines, combined lines and colours, in order.
pub(crate) const LINES: [LineTier; 6] = [
    LineTier::Straight,
    LineTier::Arc,
    LineTier::Wave,
    LineTier::Arc,
    LineTier::Hook,
    LineTier::Wave,
];
pub(crate) const COMBINED: [Combination; 4] = [
    Combination::Joined,
    Combination::Crossing,
    Combination::Branched,
    Combination::Near,
];
pub(crate) const COLOURS: [(Purity, Lightness); 6] = [
    (Purity::Muted, Lightness::Middle),
    (Purity::Grey, Lightness::Light),
    (Purity::Middle, Lightness::Dark),
    (Purity::Muted, Lightness::Light),
    (Purity::Vivid, Lightness::Middle),
    (Purity::Grey, Lightness::Dark),
];
/// The families of stage 1's exercises, quick to do; stage 2's quick ones (theirs, boxes and
/// ellipses) and its long ones.
const QUICK: [Family; 3] = [Family::Line, Family::Combined, Family::Colour];
const QUICK_TWO: [Family; 5] = [
    Family::Line,
    Family::Combined,
    Family::Colour,
    Family::Box,
    Family::Ellipse,
];
pub(crate) const LONG: [Family; 4] = [
    Family::Shape,
    Family::Areas,
    Family::Scale,
    Family::FreeBoxes,
];
/// The free boxes' horizons in turn: in the middle first.
pub(crate) const HORIZONS: [Horizon; 3] = [Horizon::Middle, Horizon::High, Horizon::Low];
/// A value scale's cells as a kind holds them: a scale's, a beginner's (`seeded` sizes them).
pub(crate) const FULL: u8 = CELLS as u8;
const SHORT: u8 = FIRST_CELLS as u8;

/// A session's exercises, the first `warm_up` of them its warm-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub exercises: Vec<Exercise>,
    pub warm_up: u32,
}

/// What a session's plan reads: the kinds `met` in earlier sessions, the `pictures` there are,
/// those `used` in earlier sessions and those the last ten showed (`recent`, S95: a composition
/// comes back after ten sessions), and how many value `scales` were met (S97: a beginner's first
/// two are shorter).
#[derive(Debug, Clone, Copy)]
pub struct Before<'a> {
    pub met: &'a [Kind],
    pub pictures: &'a Pictures,
    pub used: &'a [ItemRef],
    pub recent: &'a [ItemRef],
    pub scales: usize,
}

/// Session `number` of `stage` seeded `seed` after `before`: stage 2's for stage 2, stage 3's for
/// stage 3 (`session_three.rs`), stage 4's for stage 4 (`session_four.rs`), else stage 1's.
#[must_use]
pub fn plan(stage: u8, number: u32, seed: u64, before: &Before<'_>) -> Plan {
    match stage {
        2 => stage_two(number, seed, before),
        3 => stage_three(number, seed, before),
        4 => stage_four(number, seed, before),
        _ => stage_one(seed, before.met),
    }
}

/// Stage 1's session seeded `seed` after the kinds `met` in earlier sessions: its warm-up from
/// those of its families (none when nothing was met), then the stage's mix. Exercise `i`'s seed
/// is `derived(seed, i)`.
#[must_use]
pub fn stage_one(seed: u64, met: &[Kind]) -> Plan {
    let met: Vec<Kind> = (met.iter().copied())
        .filter(|k| QUICK.contains(&k.family()))
        .collect();
    let warm_up = warm_up(seed, &met);
    let lines = LINES.map(Kind::Line);
    let combined = COMBINED.map(Kind::Combined);
    let colours = COLOURS.map(|(purity, lightness)| Kind::Colour(Tone { purity, lightness }));
    let kinds: Vec<Kind> = (warm_up.iter().copied())
        .chain(lines)
        .chain(combined)
        .chain(colours)
        .collect();
    Plan {
        // Stage 1 holds no value scale.
        exercises: seeded(seed, kinds.into_iter().map(|kind| (kind, None)), 0),
        warm_up: u32::try_from(warm_up.len()).unwrap_or(0),
    }
}

/// Stage 2's session `number` seeded `seed` after `before`: its warm-up, then four shapes of the
/// session's tiers (the last the user's own when they have some the session reaches), a colour
/// combination of a crop (S101), a value scale and the session's perspective (`perspective`), the
/// scale's tone (grey, then of a hue) in turn by number. Without any shape picture there are no
/// shapes, without a crop no combination.
#[must_use]
pub fn stage_two(number: u32, seed: u64, before: &Before<'_>) -> Plan {
    let tiers = shape_tiers(number);
    let mut picker = Picker::new(seed, before, tiers[tiers.len() - 1]);
    let warm_up = picker.warm_up(warm_up_of(number, seed, before.met, &QUICK_TWO));
    let shapes = picker.shapes(&tiers);
    let turn = number.saturating_sub(1) as usize;
    let combination = picker.crop();
    let scale = Kind::Scale(ScaleTone::ALL[turn % ScaleTone::ALL.len()], FULL);
    let warmed = u32::try_from(warm_up.len()).unwrap_or(0);
    let all = (warm_up.into_iter())
        .chain(shapes)
        .chain(combination)
        .chain((std::iter::once(scale).chain(perspective(turn))).map(|kind| (kind, None)));
    Plan {
        exercises: seeded(seed, all, before.scales),
        warm_up: warmed,
    }
}

/// The pictures a session's shapes copy, picked by the seed: others than those shown before while
/// there are, the last of the session's shapes the user's own of its hardest tier (`reach`) or an
/// easier one when they have one; its compositions others than those of the last ten sessions
/// while there are, none twice while another is left (S95).
pub(crate) struct Picker<'a> {
    before: &'a Before<'a>,
    random: Random,
    /// The crops' own sequence (S101), so the shapes and compositions pick as before.
    crops: Random,
    used: HashSet<ItemRef>,
    reach: Tier,
    /// The session's own pictures so far, and those of the last ten sessions.
    shown: HashSet<ItemRef>,
    recent: HashSet<ItemRef>,
}

impl<'a> Picker<'a> {
    pub(crate) fn new(seed: u64, before: &'a Before<'a>, reach: Tier) -> Self {
        Self {
            before,
            random: Random::new(derived(seed, u64::MAX - 1)),
            crops: Random::new(derived(seed, u64::MAX - 2)),
            used: before.used.iter().cloned().collect(),
            reach,
            shown: HashSet::new(),
            recent: before.recent.iter().cloned().collect(),
        }
    }

    /// `kinds` (divisions, whole copies and value masses) with compositions of their tiers (value
    /// masses' among those whose masses read); none without such a composition.
    pub(crate) fn compositions(&mut self, kinds: &[Kind]) -> Vec<(Kind, Option<ItemRef>)> {
        let mut picked = Vec::new();
        for kind in kinds {
            let (Kind::Division(tier) | Kind::WholeCopy(tier) | Kind::ValueMasses(tier)) = *kind
            else {
                continue;
            };
            let massed = matches!(kind, Kind::ValueMasses(_));
            let pictures = self.before.pictures;
            let shown = (&self.shown, &self.recent);
            let Some((tier, item)) =
                pictures.pick_composition((tier, massed), &mut self.random, shown)
            else {
                continue;
            };
            self.shown.insert(item.clone());
            let kind = match kind {
                Kind::Division(_) => Kind::Division(tier),
                Kind::ValueMasses(_) => Kind::ValueMasses(tier),
                _ => Kind::WholeCopy(tier),
            };
            picked.push((kind, Some(item)));
        }
        picked
    }

    /// A colour combination with its crop (S101): one no earlier session or drill showed while
    /// any is left; none without crops.
    pub(crate) fn crop(&mut self) -> Option<(Kind, Option<ItemRef>)> {
        let pictures = self.before.pictures;
        let (tier, item) = pictures.pick_crop(&mut self.crops, &self.used)?;
        self.used.insert(item.clone());
        Some((Kind::Crop(tier), Some(item)))
    }

    /// `kind` with its picture (a shape's, a combination's crop; none for another kind), the
    /// user's own when `own` and they have one; none when no picture of its tier is left.
    fn pick(&mut self, kind: Kind, own: bool) -> Option<(Kind, Option<ItemRef>)> {
        if kind.family() == Family::Areas {
            return self.crop();
        }
        let Kind::Shape(tier) = kind else {
            return Some((kind, None));
        };
        let pictures = self.before.pictures;
        let personal =
            own.then(|| pictures.pick_personal(self.reach, &mut self.random, &self.used));
        let (tier, item) = match personal.flatten() {
            Some(found) => found,
            None => pictures.pick(tier, &mut self.random, &self.used)?,
        };
        self.used.insert(item.clone());
        Some((Kind::Shape(tier), Some(item)))
    }

    /// A warm-up's `kinds` with their pictures.
    pub(crate) fn warm_up(&mut self, kinds: Vec<Kind>) -> Vec<(Kind, Option<ItemRef>)> {
        (kinds.into_iter())
            .filter_map(|kind| self.pick(kind, false))
            .collect()
    }

    /// Shapes of `tiers` with their pictures, the last the user's own when they have one.
    pub(crate) fn shapes(&mut self, tiers: &[Tier]) -> Vec<(Kind, Option<ItemRef>)> {
        let own = self.before.pictures.has_personal(self.reach);
        (tiers.iter().enumerate())
            .filter_map(|(at, tier)| self.pick(Kind::Shape(*tier), own && at + 1 == tiers.len()))
            .collect()
    }
}

/// Stage 2's perspective in the session after `turn` others: a box to complete and an ellipse,
/// then a page of free boxes, in turn; each kind's variant in turn among its sessions.
fn perspective(turn: usize) -> Vec<Kind> {
    let round = turn / 2;
    if turn.is_multiple_of(2) {
        vec![
            Kind::Box(BoxView::ALL[round % BoxView::ALL.len()]),
            Kind::Ellipse(Guide::ALL[round % Guide::ALL.len()]),
        ]
    } else {
        vec![Kind::FreeBoxes(HORIZONS[round % HORIZONS.len()])]
    }
}

/// The four shapes' tiers of stage 2's session `number`.
const fn shape_tiers(number: u32) -> [Tier; 4] {
    match number {
        0..=2 => [Tier::Entry, Tier::Entry, Tier::Simple, Tier::Simple],
        3..=5 => [Tier::Entry, Tier::Simple, Tier::Simple, Tier::Medium],
        6..=9 => [Tier::Simple, Tier::Simple, Tier::Medium, Tier::Hard],
        _ => [Tier::Simple, Tier::Medium, Tier::Medium, Tier::Hard],
    }
}

/// `kinds` (with their pictures) as exercises, exercise `i` seeded `derived(seed, i)`; each value
/// scale of `CELLS` cells, but `FIRST_CELLS` while fewer than `FIRST_SCALES` were met, counting
/// the plan's own after the `scales` met before it (S97: a beginner's first two scales are short).
pub(crate) fn seeded(
    seed: u64,
    kinds: impl Iterator<Item = (Kind, Option<ItemRef>)>,
    scales: usize,
) -> Vec<Exercise> {
    let mut met = scales;
    kinds
        .enumerate()
        .map(|(index, (kind, picture))| Exercise {
            kind: match kind {
                Kind::Scale(tone, _) => {
                    met += 1;
                    let short = met <= FIRST_SCALES;
                    Kind::Scale(tone, if short { SHORT } else { FULL })
                }
                kind => kind,
            },
            seed: derived(seed, index as u64),
            picture,
        })
        .collect()
}

/// The kinds `met`, each once, in the order of their codes.
fn distinct(met: &[Kind]) -> Vec<Kind> {
    let mut met = met.to_vec();
    met.sort_by_key(|kind| kind.code());
    met.dedup();
    met
}

/// A kind of `family` among `met`, chosen by `random`.
fn of_family(met: &[Kind], family: Family, random: &mut Random) -> Option<Kind> {
    let of: Vec<&Kind> = met.iter().filter(|k| k.family() == family).collect();
    let at = (random.unit() * of.len() as f64) as usize;
    of.get(at.min(of.len().saturating_sub(1)))
        .map(|kind| **kind)
}

/// Stage 1's warm-up: `WARM_UP` of the kinds `met`, the families met taken in turn, each a kind
/// of its family chosen by the seed; then in the families' order.
fn warm_up(seed: u64, met: &[Kind]) -> Vec<Kind> {
    let met = distinct(met);
    let families: Vec<Family> = (Family::ALL.into_iter())
        .filter(|family| met.iter().any(|kind| kind.family() == *family))
        .collect();
    if families.is_empty() {
        return Vec::new();
    }
    let mut random = Random::new(derived(seed, u64::MAX));
    let mut kinds: Vec<Kind> = (0..WARM_UP)
        .filter_map(|turn| of_family(&met, families[turn % families.len()], &mut random))
        .collect();
    kinds.sort_by_key(|kind| kind.family());
    kinds
}

/// Stage 2's or 3's warm-up for session `number`: `WARM_UP_TWO` of the kinds `met`, one of a long
/// family (the long families met in turn by number) and the rest of the `quick` families met in
/// turn from one the number moves on; then in the families' order.
pub(crate) fn warm_up_of(number: u32, seed: u64, met: &[Kind], quick: &[Family]) -> Vec<Kind> {
    let met = distinct(met);
    let present = |families: &[Family]| -> Vec<Family> {
        (families.iter().copied())
            .filter(|family| met.iter().any(|kind| kind.family() == *family))
            .collect()
    };
    let (quick, long) = (present(quick), present(&LONG));
    let turn = number.saturating_sub(1) as usize;
    let mut families = Vec::with_capacity(WARM_UP_TWO);
    if !long.is_empty() {
        families.push(long[turn % long.len()]);
    }
    if !quick.is_empty() {
        let room = WARM_UP_TWO - families.len();
        families.extend((0..room).map(|k| quick[(turn + k) % quick.len()]));
    }
    let mut random = Random::new(derived(seed, u64::MAX));
    let mut kinds: Vec<Kind> = (families.into_iter())
        .filter_map(|family| of_family(&met, family, &mut random))
        .collect();
    kinds.sort_by_key(|kind| kind.family());
    kinds
}
