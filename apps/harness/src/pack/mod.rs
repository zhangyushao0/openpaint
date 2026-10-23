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

use openpaint_practice::{Pack, Tier};
use openpaint_store::{decode_image, encode_pack};

use crate::error::HarnessError;
use crate::output::workspace_root;
use fetch::Fetcher;
use list::{Source, SourceList};
use sheet::{Picture, write_sheet};

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
    let (pack, pictures) = build::build(&list, &Fetcher::new(packs_dir().join("cache")))?;
    let bytes = encode_pack(&pack, &pictures).map_err(HarnessError::Port)?;
    let out = out.unwrap_or_else(|| packs_dir().join(format!("{}.pack", pack.id)));
    if let Some(dir) = out.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&out, &bytes)?;
    describe(&pack);
    println!(
        "{}: {} items, {} bytes",
        out.display(),
        pack.items.len(),
        bytes.len()
    );
    if sheets {
        let dir = out.parent().unwrap_or(Path::new("."));
        write_sheets(&pack, &pictures, dir)?;
    }
    Ok(())
}

/// One line per item, numbered within its tier as its sheet numbers it, then the tiers' counts.
fn describe(pack: &Pack) {
    for tier in Tier::ALL {
        let items = pack.items.iter().filter(|item| item.tier == tier);
        for (number, item) in items.enumerate() {
            println!(
                "{:<6} {:>3}  {:<30} {:>4} x {:<4} {:<9}  {} | {}",
                tier.name(),
                number + 1,
                item.id,
                item.size[0],
                item.size[1],
                item.licence.name(),
                item.title,
                item.creator
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
    println!("{}", counts.join(", "));
}

/// A contact sheet per tier of `pack` in `dir`: `<pack id>-<tier>.png`.
fn write_sheets(pack: &Pack, pictures: &[Vec<u8>], dir: &Path) -> Result<(), HarnessError> {
    for tier in Tier::ALL {
        let mut cells = Vec::new();
        for (item, bytes) in pack.items.iter().zip(pictures) {
            if item.tier == tier {
                let (size, rgba) = decode_image(bytes).map_err(|error| HarnessError::Pack {
                    item: item.id.clone(),
                    problem: format!("its built picture: {error}"),
                })?;
                cells.push(Picture { size, rgba });
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
