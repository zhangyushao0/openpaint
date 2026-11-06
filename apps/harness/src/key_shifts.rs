//! `key-shifts` (S107): contact sheets on which a key shift's three checks are judged by eye
//! (`openpaint_practice::KeyCheck`). Every crop of the official packs (the combinations' and the
//! compositions' corners under `artifacts/packs/`, or the packs named) is given a tendency as a
//! session gives them, the leans in turn, and painted again five ways at a session frame's size
//! (`key_copies.rs`), each read as Compare reads a copy (`Areas::read`, `KeyCheck::of`):
//!
//! - by rule (`Tendency::shifted`), flat: nothing should be marked;
//! - the same picked and painted by a hand, unevenly: nothing should be marked;
//! - its lightness broken: one area 0.14, 0.09 or 0.06 off in turn (marked, marked, not);
//! - an area left behind, a grey where the crop has one: moved none, a quarter or three fifths of
//!   its way in turn;
//! - two neighbours apart in the crop merged: painted 0, 0.025 or 0.05 apart in turn.
//!
//! A row per crop on `artifacts/key-shifts/key-shifts-<n>.png`, eight a sheet: the crop with its
//! tendency's disc, then the five copies, on each what Compare marks (lightness red, not moved
//! orange, merged neighbours' edge blue) and the area changed on purpose ringed black; a line per
//! copy says what was changed, by how much, and what was marked; the counts close.

use std::path::PathBuf;

use openpaint_practice::{
    Areas, KeyCheck, LIGHTEST, Lean, PictureKind, Random, Rgb8, Tendency, delta_e, derived,
    from_oklab, main_hue, oklab, toward,
};
use openpaint_store::{decode_image, open_pack};

use crate::error::HarnessError;
use crate::key_copies::{by_hand, flat, picked_by_hand, size_of};
use crate::key_sheet::write_rows;
use crate::output::workspace_root;

/// The packs read when none is named.
const PACKS: [&str; 2] = ["openpaint-combinations.pack", "openpaint-compositions.pack"];
const SEED: u64 = 0x5EED_0107;
/// The sheet's paper, as a practice sheet's.
const PAPER: Rgb8 = [255; 3];
/// The deliberate errors' sizes, in turn by the crop's number.
const LIGHTNESS: [f64; 3] = [0.14, 0.09, 0.06];
const BEHIND: [f64; 3] = [0.0, 0.25, 0.6];
const MERGING: [f64; 3] = [0.0, 0.025, 0.05];

/// A crop of a pack: its picture and its areas.
pub(crate) struct Crop {
    pub(crate) id: String,
    pub(crate) size: [u32; 2],
    pub(crate) rgba: Vec<u8>,
    pub(crate) areas: Areas,
}

/// What a copy was made as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Case {
    Rule,
    Hand,
    Lightness,
    Behind,
    Merged,
}

/// A copy painted and read: its pixels at `size`, what Compare marked, the area or pair changed
/// on purpose and by how much.
pub(crate) struct Painted {
    pub(crate) case: Case,
    pub(crate) size: [u32; 2],
    pub(crate) pixels: Vec<u8>,
    pub(crate) check: KeyCheck,
    pub(crate) target: Vec<usize>,
    pub(crate) amount: f64,
}

/// A crop's row: its tendency and its copies.
pub(crate) struct Row {
    pub(crate) tendency: Tendency,
    pub(crate) copies: Vec<Painted>,
}

pub fn run(args: &[&str]) -> Result<(), HarnessError> {
    let packs = workspace_root().join("artifacts").join("packs");
    let paths: Vec<PathBuf> = if args.is_empty() {
        PACKS.iter().map(|name| packs.join(name)).collect()
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    let crops = crops_of(&paths)?;
    if crops.is_empty() {
        return Err(HarnessError::Missing(
            "no crops: build the packs first (cargo practice-pack build apps/harness/packs/...)",
        ));
    }
    let rows: Vec<Row> = (crops.iter().enumerate())
        .map(|(index, crop)| judged(index, crop))
        .collect();
    for (number, (crop, row)) in (1..).zip(crops.iter().zip(&rows)) {
        describe(number, crop, row);
    }
    tally(&rows);
    let dir = workspace_root().join("artifacts").join("key-shifts");
    write_rows(&crops, &rows, &dir)
}

/// The crops of the packs at `paths` that exist, in their order.
fn crops_of(paths: &[PathBuf]) -> Result<Vec<Crop>, HarnessError> {
    let mut crops = Vec::new();
    for path in paths.iter().filter(|path| path.exists()) {
        let bytes = std::fs::read(path)?;
        let (pack, pictures) = open_pack(&bytes).map_err(HarnessError::Port)?;
        for (item, picture) in pack.items.iter().zip(pictures) {
            let (PictureKind::Crop, Some(areas)) = (item.kind, item.areas.as_deref()) else {
                continue;
            };
            let (size, rgba) = decode_image(picture).map_err(|error| HarnessError::Pack {
                item: item.id.clone(),
                problem: format!("its picture: {error:?}"),
            })?;
            crops.push(Crop {
                id: item.id.clone(),
                size,
                rgba,
                areas: areas.clone(),
            });
        }
    }
    Ok(crops)
}

/// Crop `index` given its tendency and painted again the five ways, each read.
fn judged(index: usize, crop: &Crop) -> Row {
    let areas = &crop.areas;
    let mut random = Random::new(derived(SEED, index as u64));
    let lean = Lean::ALL[index % Lean::ALL.len()];
    let tendency = Tendency::chosen(lean, main_hue(areas), &mut random);
    let shifted = tendency.shifted(areas);
    let size = size_of(crop.size);
    let level = index % 3;
    let read = |case: Case, (pixels, target, amount): (Vec<u8>, Vec<usize>, f64)| {
        let copy = areas.read(size, &pixels, PAPER);
        Painted {
            case,
            size,
            check: KeyCheck::of(areas, &copy.colours, tendency),
            pixels,
            target,
            amount,
        }
    };
    let hand = picked_by_hand(&shifted, &mut random);
    let seed = random.next_u64();
    let copies = vec![
        read(Case::Rule, (flat(areas, &shifted, size), Vec::new(), 0.0)),
        read(
            Case::Hand,
            (by_hand(areas, &hand, size, seed), Vec::new(), 0.0),
        ),
        read(
            Case::Lightness,
            lightness_broken(areas, &shifted, size, level),
        ),
        read(Case::Behind, left_behind(areas, &shifted, size, level)),
        read(Case::Merged, merged(areas, &shifted, size, level)),
    ];
    Row { tendency, copies }
}

/// The shift with its largest area of middle lightness made `LIGHTNESS[level]` darker (a light
/// one) or lighter.
fn lightness_broken(
    areas: &Areas,
    shifted: &[Rgb8],
    size: [u32; 2],
    level: usize,
) -> (Vec<u8>, Vec<usize>, f64) {
    let sizes = areas.sizes();
    let middle = |area: &usize| (0.2..0.85).contains(&oklab(shifted[*area])[0]);
    let Some(area) = (0..areas.count()).filter(middle).max_by_key(|a| sizes[*a]) else {
        return (flat(areas, shifted, size), Vec::new(), 0.0);
    };
    let [l, a, b] = oklab(shifted[area]);
    let off = if l > 0.5 {
        -LIGHTNESS[level]
    } else {
        LIGHTNESS[level]
    };
    let mut colours = shifted.to_vec();
    colours[area] = from_oklab([l + off, a, b]);
    (flat(areas, &colours, size), vec![area], off)
}

/// The shift with one area, a grey where the crop has one (else the largest not among the
/// lightest), moved only `BEHIND[level]` of its way.
fn left_behind(
    areas: &Areas,
    shifted: &[Rgb8],
    size: [u32; 2],
    level: usize,
) -> (Vec<u8>, Vec<usize>, f64) {
    let sizes = areas.sizes();
    let lab = |area: usize| oklab(areas.colours[area]);
    let held = |area: &usize| lab(*area)[0] < LIGHTEST;
    let grey = (0..areas.count())
        .filter(held)
        .filter(|area| lab(*area)[1].hypot(lab(*area)[2]) < 0.04)
        .max_by_key(|a| sizes[*a]);
    let Some(area) = grey.or_else(|| (0..areas.count()).filter(held).max_by_key(|a| sizes[*a]))
    else {
        return (flat(areas, shifted, size), Vec::new(), 0.0);
    };
    let share = BEHIND[level];
    let [from, to] = [areas.colours[area], shifted[area]].map(oklab);
    let mut colours = shifted.to_vec();
    colours[area] = from_oklab(std::array::from_fn(|k| from[k] + share * (to[k] - from[k])));
    (flat(areas, &colours, size), vec![area], share)
}

/// The shift with two neighbours apart in the crop (of the nearest lightness) painted
/// `MERGING[level]` apart about their middle.
fn merged(
    areas: &Areas,
    shifted: &[Rgb8],
    size: [u32; 2],
    level: usize,
) -> (Vec<u8>, Vec<usize>, f64) {
    let lightness = |area: usize| oklab(shifted[area])[0];
    let pair = (areas.neighbours.iter())
        .filter(|(a, b)| delta_e(areas.colours[*a], areas.colours[*b]) >= openpaint_practice::APART)
        .min_by(|x, y| {
            let gap = |(a, b): &&(usize, usize)| (lightness(*a) - lightness(*b)).abs();
            gap(x).total_cmp(&gap(y))
        });
    let Some(&(a, b)) = pair else {
        return (flat(areas, shifted, size), Vec::new(), 0.0);
    };
    let [x, y] = [shifted[a], shifted[b]].map(oklab);
    let middle: [f64; 3] = std::array::from_fn(|k| (x[k] + y[k]) / 2.0);
    let apart = (0..3)
        .map(|k| (x[k] - y[k]).powi(2))
        .sum::<f64>()
        .sqrt()
        .max(1e-9);
    let half = MERGING[level] / 2.0;
    let mut colours = shifted.to_vec();
    colours[a] = from_oklab(std::array::from_fn(|k| {
        middle[k] + half * (x[k] - y[k]) / apart
    }));
    colours[b] = from_oklab(std::array::from_fn(|k| {
        middle[k] - half * (x[k] - y[k]) / apart
    }));
    (flat(areas, &colours, size), vec![a, b], MERGING[level])
}

/// Crop `number`'s lines: its tendency, then each copy's change and marks.
fn describe(number: usize, crop: &Crop, row: &Row) {
    let tendency = row.tendency;
    println!(
        "crop {number:>3} {:<34} areas {} main hue {:?} tendency {:?} {}°",
        crop.id,
        crop.areas.count(),
        main_hue(&crop.areas),
        tendency.lean,
        tendency.hue
    );
    let colours: Vec<String> = (crop.areas.colours.iter())
        .map(|colour| {
            let [l, c, h] = openpaint_practice::oklch(*colour);
            format!("{l:.2}/{c:.3}/{h:.0}")
        })
        .collect();
    println!("    areas (L/chroma/hue) {}", colours.join(" "));
    for copy in &row.copies {
        let check = &copy.check;
        let read = crop.areas.read(copy.size, &copy.pixels, PAPER).colours;
        // Each area's lightness off and its move toward the tendency, as Compare read them.
        let moved: Vec<String> = (crop.areas.colours.iter().zip(&read))
            .map(|(original, mine)| {
                mine.map_or_else(
                    || "-".to_owned(),
                    |mine| {
                        let off = oklab(mine)[0] - oklab(*original)[0];
                        format!("{off:+.2}/{:.3}", toward(*original, mine, tendency))
                    },
                )
            })
            .collect();
        println!(
            "    {:<9} target {:?} amount {:+.3} | {} | lightness {:?} unmoved {:?} merged {:?}",
            format!("{:?}", copy.case),
            copy.target,
            copy.amount,
            moved.join(" "),
            check.off_lightness,
            check.unmoved,
            check.merged,
        );
    }
}

/// How many copies of each case were marked as their change asks: none for the rule and the
/// hand; the target, for the others, at each size in turn.
fn tally(rows: &[Row]) {
    for case in [Case::Rule, Case::Hand] {
        let clean = (rows.iter())
            .filter(|row| row.copies.iter().any(|c| c.case == case && clean(&c.check)))
            .count();
        println!("{case:?}: {clean} of {} unmarked", rows.len());
    }
    for case in [Case::Lightness, Case::Behind, Case::Merged] {
        for level in 0..3 {
            let copies: Vec<&Painted> = (rows.iter().enumerate())
                .filter(|(index, _)| index % 3 == level)
                .flat_map(|(_, row)| row.copies.iter().filter(|c| c.case == case))
                .filter(|copy| !copy.target.is_empty())
                .collect();
            let hit = copies.iter().filter(|copy| hits(copy)).count();
            println!(
                "{case:?} size {}: {hit} of {} marked",
                level + 1,
                copies.len()
            );
        }
    }
}

fn clean(check: &KeyCheck) -> bool {
    check.off_lightness.is_empty() && check.unmoved.is_empty() && check.merged.is_empty()
}

/// Whether `copy`'s change was marked by its own check.
fn hits(copy: &Painted) -> bool {
    let check = &copy.check;
    match copy.case {
        Case::Lightness => copy.target.iter().all(|a| check.off_lightness.contains(a)),
        Case::Behind => copy.target.iter().all(|a| check.unmoved.contains(a)),
        Case::Merged => (check.merged.iter()).any(|(a, b)| copy.target == [*a, *b]),
        Case::Rule | Case::Hand => clean(check),
    }
}
