//! `practice-pack` (also `cargo practice-pack`): practice packs built from curated source lists
//! (ROADMAP S78). The repository keeps the lists (`apps/harness/packs/`), never a picture; a pack
//! and its downloads live under `artifacts/packs/` (untracked).
//!
//! - `build <list> [out] [--sheet]`: the pack the list names (`build.rs`), written to `out` or
//!   `artifacts/packs/<pack id>.pack`, one line per item; with `--sheet` a contact sheet per tier
//!   beside it (`<pack id>-<tier>.png`), numbered as the lines.
//! - `search <cleveland|met|commons> <query> [count]`: candidates and their contact sheet for the
//!   curator (`search.rs`).

mod build;
mod commons;
mod fetch;
mod list;
mod search;
mod sheet;
mod sources;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use openpaint_practice::Tier;
use openpaint_store::{decode_image, encode_pack};

use crate::error::HarnessError;
use crate::output::workspace_root;
use build::{Built, Outlined};
use fetch::Fetcher;
use list::{Source, SourceList};
use sheet::{Mark, Picture, write_sheet};

const USAGE: &str = "practice-pack build <list.toml> [out.pack] [--sheet]\n       \
                     practice-pack search <cleveland|met|commons> <query> [count]";

pub fn run(args: &[&str]) -> Result<(), HarnessError> {
    match args {
        ["build", rest @ ..] => build_command(rest),
        ["search", source, query, rest @ ..] => {
            let source = Source::of_name(source).ok_or(HarnessError::Missing(USAGE))?;
            let count = match rest {
                [] => 30,
                [count] => count.parse().map_err(|_| HarnessError::Missing(USAGE))?,
                _ => return Err(HarnessError::Missing(USAGE)),
            };
            search::run(
                &Fetcher::new(packs_dir().join("cache")),
                source,
                query,
                count,
            )
        }
        _ => Err(HarnessError::Missing(USAGE)),
    }
}

/// Where packs, their sheets and the download cache live.
fn packs_dir() -> PathBuf {
    workspace_root().join("artifacts").join("packs")
}

fn build_command(rest: &[&str]) -> Result<(), HarnessError> {
    let sheets = rest.contains(&"--sheet");
    let paths: Vec<&str> = rest.iter().copied().filter(|a| *a != "--sheet").collect();
    let (list, out) = match paths[..] {
        [list] => (list, None),
        [list, out] => (list, Some(PathBuf::from(out))),
        _ => return Err(HarnessError::Missing(USAGE)),
    };
    let list = SourceList::parse(&fs::read_to_string(list)?)?;
    let built = build::build(&list, &Fetcher::new(packs_dir().join("cache")))?;
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
        return Err(HarnessError::Pack {
            item: item.id.clone(),
            problem: format!(
                "its outline was refused ({refusal}); mark it `no_outline` with a reason"
            ),
        });
    }
    let bytes = encode_pack(&built.pack, &built.pictures).map_err(HarnessError::Port)?;
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
/// none as the list says, refused), then the tiers' counts.
fn describe(built: &Built) {
    let pack = &built.pack;
    for tier in Tier::ALL {
        let items = (pack.items.iter().zip(&built.outlines)).filter(|(item, _)| item.tier == tier);
        for (number, (item, outlined)) in items.enumerate() {
            let licence = item.credit.as_ref().map_or("", |c| c.licence.name());
            let outline = match outlined {
                Outlined::Kept => "outline".to_owned(),
                Outlined::Listed { why, .. } => format!("none: {why}"),
                Outlined::Refused(refusal) => format!("REFUSED: {refusal}"),
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
        .map(|tier| {
            let count = pack.items.iter().filter(|item| item.tier == *tier).count();
            format!("{} {count}", tier.name())
        })
        .collect();
    let kept = (built.outlines.iter())
        .filter(|o| **o == Outlined::Kept)
        .count();
    println!("{}; {kept} outlines", counts.join(", "));
}

/// A contact sheet per tier of the built pack in `dir`, `<pack id>-<tier>.png`, each picture
/// with its outline in red, or the one the derivation made of an item listed without one in
/// blue.
fn write_sheets(built: &Built, dir: &Path) -> Result<(), HarnessError> {
    let pack = &built.pack;
    for tier in Tier::ALL {
        let mut cells = Vec::new();
        let items = pack.items.iter().zip(&built.pictures).zip(&built.outlines);
        for ((item, bytes), outlined) in items {
            if item.tier == tier {
                let (size, rgba) = decode_image(bytes).map_err(|error| HarnessError::Pack {
                    item: item.id.clone(),
                    problem: format!("its built picture: {error}"),
                })?;
                let outline = match outlined {
                    Outlined::Kept => item.outline.clone().map(|o| (o, Mark::Kept)),
                    Outlined::Listed {
                        derived: Ok(outline),
                        ..
                    } => Some((outline.clone(), Mark::Dropped)),
                    Outlined::Listed { .. } | Outlined::Refused(_) => None,
                };
                cells.push(Picture {
                    size,
                    rgba,
                    outline,
                });
            }
        }
        if !cells.is_empty() {
            let path = dir.join(format!("{}-{}.png", pack.id, tier.name()));
            write_sheet(&cells, &path)?;
            println!("{}: {} pictures", path.display(), cells.len());
        }
    }
    Ok(())
}
