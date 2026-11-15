//! `contrasts` (S112a): contact sheets on which abstract analysis's five views are judged by eye
//! (`openpaint_practice::Contrasts`), each view's go or no-go before the kind is built. Every
//! composition of the official compositions pack (`artifacts/packs/openpaint-compositions.pack`,
//! or the packs named) as the screen shows it (`shown_picture`), a row each, eight a sheet: the
//! picture (its number and tier), then its views, each labelled: VALUE 明度, CHROMA 纯度, HUE 色相
//! and EDGES 虚实 (the picture under its veil) and SQUINT 眯眼; on
//! `artifacts/contrasts/contrasts-<n>.png`. A line per picture: its id, tier and the share of it
//! each veil leaves clear.

use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use openpaint_practice::{ContrastView, Contrasts, PictureKind, Tier};
use openpaint_practice_store::{open_pack, shown_picture};
use openpaint_sheet::{fill, label};

use crate::error::HarnessError;
use crate::output::{workspace_root, write_rgba};

/// The packs read when none is named.
const PACKS: [&str; 1] = ["openpaint-compositions.pack"];
/// A cell's side and the gap between cells, in sheet pixels: six cells a row in about 2,400.
const SIDE: u32 = 384;
const GAP: u32 = 10;
const COLUMNS: u32 = 6;
const ROWS: usize = 8;
const BACKGROUND: Rgba<u8> = Rgba([150, 150, 150, 255]);
const GROUND: Rgba<u8> = Rgba([205, 205, 205, 255]);

/// A composition of a pack as the screen shows it, and its contrasts.
struct Composition {
    id: String,
    tier: Tier,
    size: [u32; 2],
    rgba: Vec<u8>,
    contrasts: Contrasts,
}

pub fn run(args: &[&str]) -> Result<(), HarnessError> {
    let packs = workspace_root().join("artifacts").join("packs");
    let paths: Vec<PathBuf> = if args.is_empty() {
        PACKS.iter().map(|name| packs.join(name)).collect()
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    let compositions = compositions_of(&paths)?;
    if compositions.is_empty() {
        return Err(HarnessError::Missing(
            "no compositions: build the packs first (cargo practice-pack build apps/harness/packs/...)",
        ));
    }
    for (number, composition) in (1..).zip(&compositions) {
        describe(number, composition);
    }
    let dir = workspace_root().join("artifacts").join("contrasts");
    write_pages(&compositions, &dir)
}

/// The compositions of the packs at `paths` that exist, in their order.
fn compositions_of(paths: &[PathBuf]) -> Result<Vec<Composition>, HarnessError> {
    let mut compositions = Vec::new();
    for path in paths.iter().filter(|path| path.exists()) {
        let bytes = std::fs::read(path)?;
        let (pack, pictures) = open_pack(&bytes).map_err(HarnessError::Port)?;
        for (item, file) in pack.items.iter().zip(pictures) {
            if item.kind != PictureKind::Composition {
                continue;
            }
            let shown = shown_picture(file).map_err(|error| HarnessError::Pack {
                item: item.id.clone(),
                problem: format!("its picture: {error:?}"),
            })?;
            compositions.push(Composition {
                id: item.id.clone(),
                tier: item.tier,
                contrasts: Contrasts::of(shown.size, &shown.pixels),
                size: shown.size,
                rgba: shown.pixels,
            });
        }
    }
    Ok(compositions)
}

/// Composition `number`'s line: its id, tier and how much of it each veil leaves clear.
fn describe(number: usize, composition: &Composition) {
    let clear: Vec<String> = (ContrastView::ALL.iter())
        .filter_map(|view| {
            let cells = composition.contrasts.strength(*view)?;
            let clear = cells.iter().filter(|strength| **strength >= 0.5).count();
            let share = clear as f64 / cells.len().max(1) as f64;
            Some(format!("{} {:>3.0} %", word(*view), 100.0 * share))
        })
        .collect();
    println!(
        "{number:>3} {:<40} {:<6} clear: {}",
        composition.id,
        format!("{:?}", composition.tier),
        clear.join("  ")
    );
}

/// Writes `compositions` as sheets `contrasts-<n>.png` in `dir`.
fn write_pages(compositions: &[Composition], dir: &Path) -> Result<(), HarnessError> {
    std::fs::create_dir_all(dir)?;
    for (page, rows) in compositions.chunks(ROWS).enumerate() {
        let count = u32::try_from(rows.len()).unwrap_or(1);
        let [width, height] = [COLUMNS * (SIDE + GAP) + GAP, count * (SIDE + GAP) + GAP];
        let mut sheet = RgbaImage::from_pixel(width, height, BACKGROUND);
        for (place, composition) in (0u32..).zip(rows) {
            let y = GAP + place * (SIDE + GAP);
            let number = page * ROWS + place as usize + 1;
            row(&mut sheet, y, composition, number);
        }
        let path = dir.join(format!("contrasts-{}.png", page + 1));
        write_rgba(&path, [width, height], sheet.as_raw())?;
        println!("{}: {} compositions", path.display(), rows.len());
    }
    Ok(())
}

/// Composition `number`'s row at `y`: the picture, then its five views.
fn row(sheet: &mut RgbaImage, y: u32, composition: &Composition, number: usize) {
    let Some(picture) = RgbaImage::from_raw(
        composition.size[0],
        composition.size[1],
        composition.rgba.clone(),
    ) else {
        return;
    };
    let fitted = fitted(composition.size);
    let thumb = imageops::resize(&picture, fitted[0], fitted[1], FilterType::Triangle);
    let tier = format!("{:?}", composition.tier).to_uppercase();
    cell(sheet, [GAP, y], &thumb, &format!("{number} {tier}"));
    let cells = composition.contrasts.size;
    for (column, view) in (1u32..).zip(ContrastView::ALL) {
        let x = GAP + column * (SIDE + GAP);
        let Some(view_picture) =
            RgbaImage::from_raw(cells[0], cells[1], composition.contrasts.picture(view))
        else {
            continue;
        };
        let view_picture =
            imageops::resize(&view_picture, fitted[0], fitted[1], FilterType::Triangle);
        let shown = if view == ContrastView::Squint {
            view_picture
        } else {
            let mut veiled = thumb.clone();
            imageops::overlay(&mut veiled, &view_picture, 0, 0);
            veiled
        };
        cell(sheet, [x, y], &shown, word(view));
    }
}

/// A cell at `[x, y]`: `picture` in its middle, `text` at its corner.
fn cell(sheet: &mut RgbaImage, [x, y]: [u32; 2], picture: &RgbaImage, text: &str) {
    fill(sheet, [x, y], [SIDE, SIDE], GROUND);
    let left = x + (SIDE - picture.width()) / 2;
    let top = y + (SIDE - picture.height()) / 2;
    imageops::overlay(sheet, picture, i64::from(left), i64::from(top));
    label(sheet, [x, y], text);
}

/// A picture of `size` as large as a cell shows it.
fn fitted([width, height]: [u32; 2]) -> [u32; 2] {
    let scale = f64::from(SIDE) / f64::from(width.max(height).max(1));
    [width, height].map(|length| ((f64::from(length) * scale).round() as u32).clamp(1, SIDE))
}

/// A view's label on the sheet.
fn word(view: ContrastView) -> &'static str {
    match view {
        ContrastView::Lightness => "VALUE",
        ContrastView::Chroma => "CHROMA",
        ContrastView::Hue => "HUE",
        ContrastView::Edges => "EDGES",
        ContrastView::Squint => "SQUINT",
    }
}
