//! `openpaint-packs` (`cargo practice-pack` from the repository's root): practice packs built
//! from curated source lists (ROADMAP S78), in a workspace of its own outside the gate (S111).
//! The repository keeps the lists (`tools/packs/lists/`), never a picture; a pack and its
//! downloads live under `artifacts/packs/` (untracked).
//!
//! - `build <list> [out] [--sheet]`: the pack the list names (`build.rs`), written to `out` or
//!   `artifacts/packs/<pack id>.pack`, one line per item; with `--sheet` a contact sheet per tier
//!   beside it (`<pack id>-<tier>.png`), numbered as the lines. A list of shapes loads the
//!   outlines' segmentation model through the same cache (`segment.rs`, 176 MB on the first
//!   build); a list of compositions never does.
//! - `search <cleveland|met|commons> <query> [count]`: candidates and their contact sheet for the
//!   curator (`search.rs`).
#![forbid(unsafe_code)]

mod build;
mod commons;
mod crops;
mod error;
mod fetch;
mod full;
mod libraries;
mod list;
mod nga;
mod rijks;
#[cfg(test)]
mod scratch;
mod search;
mod segment;
mod sheet;
mod sources;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_crops;
#[cfg(test)]
mod tests_libraries;
#[cfg(test)]
mod tests_sources;
mod trim;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use openpaint_practice::{PackItem, Tier};
use openpaint_practice_store::encode_pack;
use openpaint_store::decode_image;

use crate::error::PackError;
use build::{Built, Massed, Outlined};
use crops::{describe_crops, write_crop_sheets};
use fetch::Fetcher;
use list::{Source, SourceList};
use segment::Segmenter;
use sheet::{Mark, Picture, write_sheet};

const USAGE: &str = "practice-pack build <list.toml> [out.pack] [--sheet]\n       \
                     practice-pack search <cleveland|met|commons> <query> [count]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The repository's root: the builder lives at `tools/packs`, two levels down.
pub fn repo_root() -> &'static Path {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.ancestors().nth(2).unwrap_or(manifest)
}

fn run(args: &[&str]) -> Result<(), PackError> {
    match args {
        ["build", rest @ ..] => build_command(rest),
        ["search", source, query, rest @ ..] => {
            let source = Source::of_name(source).ok_or(PackError::Missing(USAGE))?;
            let count = match rest {
                [] => 30,
                [count] => count.parse().map_err(|_| PackError::Missing(USAGE))?,
                _ => return Err(PackError::Missing(USAGE)),
            };
            search::run(
                &Fetcher::new(packs_dir().join("cache")),
                source,
                query,
                count,
            )
        }
        _ => Err(PackError::Missing(USAGE)),
    }
}

/// Where packs, their sheets and the download cache live.
fn packs_dir() -> PathBuf {
    repo_root().join("artifacts").join("packs")
}

fn build_command(rest: &[&str]) -> Result<(), PackError> {
    let sheets = rest.contains(&"--sheet");
    let paths: Vec<&str> = rest.iter().copied().filter(|a| *a != "--sheet").collect();
    let (list, out) = match paths[..] {
        [list] => (list, None),
        [list, out] => (list, Some(PathBuf::from(out))),
        _ => return Err(PackError::Missing(USAGE)),
    };
    let list = SourceList::parse(&fs::read_to_string(list)?)?;
    let fetcher = Fetcher::new(packs_dir().join("cache"));
    let segmenter = (list.kind.outlined())
        .then(|| Segmenter::load(&fetcher))
        .transpose()?;
    let built = build::build(&list, &fetcher, &|size, rgba| match &segmenter {
        Some(segmenter) => segmenter.matte(size, rgba),
        None => Err(PackError::Missing("no outline is derived for this kind")),
    })?;
    let out = out.unwrap_or_else(|| packs_dir().join(format!("{}.pack", built.pack.id)));
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)?;
    }
    describe(&built);
    if sheets {
        let dir = out.parent().unwrap_or(Path::new("."));
        write_sheets(&built, dir)?;
    }
    if let Some((item, Outlined::Refused(refusal))) = (built.pack.items.iter())
        .zip(&built.outlines)
        .find(|(_, outlined)| matches!(outlined, Outlined::Refused(_)))
    {
        return Err(PackError::Pack {
            item: item.id.clone(),
            problem: format!(
                "its outline was refused ({refusal}); mark it `no_outline` with a reason"
            ),
        });
    }
    let bytes = encode_pack(&built.pack, &built.pictures).map_err(PackError::Port)?;
    fs::write(&out, &bytes)?;
    println!(
        "{}: {} items, {} bytes",
        out.display(),
        built.pack.items.len(),
        bytes.len()
    );
    Ok(())
}

/// One line per item, numbered within its tier as its sheet numbers it, with its outline (kept,
/// none as the list says, refused) or its value masses (kept, none as the list says), then the
/// tiers' counts.
fn describe(built: &Built) {
    let pack = &built.pack;
    for tier in Tier::ALL {
        let items = (pack
            .items
            .iter()
            .zip(built.outlines.iter().zip(&built.masses)))
        .filter(|(item, _)| item.tier == tier && !item.kind.cropped());
        for (number, (item, (outlined, massed))) in items.enumerate() {
            let licence = item.credit.as_ref().map_or("", |c| c.licence.name());
            let outline = match (outlined, massed) {
                (Outlined::Kept, _) => "outline".to_owned(),
                (Outlined::Listed { why, .. }, _) => format!("none: {why}"),
                (Outlined::Refused(refusal), _) => format!("REFUSED: {refusal}"),
                (Outlined::Never, Massed::Kept) => "value masses".to_owned(),
                (Outlined::Never, Massed::Listed { why, .. }) => format!("no masses: {why}"),
                (Outlined::Never, Massed::Never) => "no outline: a composition".to_owned(),
            };
            println!(
                "{:<6} {:>3}  {:<30} {:>4} x {:<4} {:<9}  {} | {outline}",
                tier.name(),
                number + 1,
                item.id,
                item.size[0],
                item.size[1],
                licence,
                item.title,
            );
        }
    }
    let counts: Vec<String> = Tier::ALL
        .iter()
        .filter_map(|tier| {
            let whole = |item: &&PackItem| item.tier == *tier && !item.kind.cropped();
            let count = pack.items.iter().filter(whole).count();
            (count > 0).then(|| format!("{} {count}", tier.name()))
        })
        .collect();
    let kept = (built.outlines.iter())
        .filter(|o| **o == Outlined::Kept)
        .count();
    let massed = (built.masses.iter())
        .filter(|m| **m == Massed::Kept)
        .count();
    describe_crops(&built.crops);
    let crops = (built.crops.iter())
        .filter(|crop| crop.left_out.is_none())
        .count();
    println!(
        "{}; {kept} outlines, {massed} value masses, {crops} crops",
        counts.join(", ")
    );
}

/// A contact sheet per tier of the built pack in `dir`, `<pack id>-<tier>.png`, each picture
/// with its outline in red, or the one the derivation made of an item listed without one in
/// blue; a composition beside its value masses, their edges red, or blue where listed without.
fn write_sheets(built: &Built, dir: &Path) -> Result<(), PackError> {
    let pack = &built.pack;
    for tier in Tier::ALL {
        let mut cells = Vec::new();
        let items =
            (pack.items.iter().zip(&built.pictures)).zip(built.outlines.iter().zip(&built.masses));
        for ((item, bytes), (outlined, massed)) in items {
            if item.tier == tier && !item.kind.cropped() {
                let (size, rgba) = decode_image(bytes).map_err(|error| PackError::Pack {
                    item: item.id.clone(),
                    problem: format!("its built picture: {error}"),
                })?;
                let outline = match outlined {
                    Outlined::Kept => item.outline.clone().map(|o| (o, Mark::Kept)),
                    Outlined::Listed {
                        derived: Ok(outline),
                        ..
                    } => Some((outline.clone(), Mark::Dropped)),
                    Outlined::Listed { .. } | Outlined::Refused(_) | Outlined::Never => None,
                };
                let masses = match massed {
                    Massed::Kept => item.masses.as_deref().map(|m| (m.clone(), Mark::Kept)),
                    Massed::Listed { made, .. } => Some((made.clone(), Mark::Dropped)),
                    Massed::Never => None,
                };
                cells.push(Picture {
                    size,
                    rgba,
                    outline,
                    masses,
                    areas: None,
                });
            }
        }
        if !cells.is_empty() {
            let path = dir.join(format!("{}-{}.png", pack.id, tier.name()));
            write_sheet(&cells, 1, &path)?;
            println!("{}: {} pictures", path.display(), cells.len());
        }
    }
    if !built.crops.is_empty() {
        write_crop_sheets(&built.crops, &pack.id, dir)?;
    }
    Ok(())
}
