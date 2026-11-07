//! The `key-shifts` contact sheets (`key_shifts.rs`, S107): a row per crop, eight a sheet: the
//! crop, then its five copies, each with the tendency's disc at its top right and what Compare
//! marked on it: an area whose lightness left the crop's outlined red, one that did not move toward
//! the tendency outlined orange (wider, under the red), two merged neighbours' edge blue; the area
//! or pair changed on purpose ringed black at its middle.

use std::path::Path;

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};
use openpaint_practice::{Areas, Point, Rgb8};

use crate::error::HarnessError;
use crate::key_shifts::{Crop, Painted, Row};
use crate::output::write_rgba;
use openpaint_sheet::{CELL, draw_in, fill, number, thumbnail};

const GAP: u32 = 14;
const COLUMNS: u32 = 6;
const ROWS: usize = 8;
const BACKGROUND: Rgba<u8> = Rgba([150, 150, 150, 255]);
const GROUND: Rgba<u8> = Rgba([205, 205, 205, 255]);
const RED: Rgba<u8> = Rgba([225, 25, 25, 255]);
const ORANGE: Rgba<u8> = Rgba([255, 150, 0, 255]);
const BLUE: Rgba<u8> = Rgba([30, 90, 235, 255]);
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
/// The tendency's disc's radius and a target's ring's, in sheet pixels.
const DISC: f64 = 22.0;
const TARGET: f64 = 9.0;

/// Writes `rows` (of `crops`) as sheets `key-shifts-<n>.png` in `dir`.
///
/// # Errors
/// `Io` or `Png` when a sheet cannot be written.
pub(crate) fn write_rows(crops: &[Crop], rows: &[Row], dir: &Path) -> Result<(), HarnessError> {
    std::fs::create_dir_all(dir)?;
    for (page, (crops, rows)) in crops.chunks(ROWS).zip(rows.chunks(ROWS)).enumerate() {
        let count = u32::try_from(rows.len()).unwrap_or(1);
        let [width, height] = [COLUMNS * (CELL + GAP) + GAP, count * (CELL + GAP) + GAP];
        let mut sheet = RgbaImage::from_pixel(width, height, BACKGROUND);
        for (place, (crop, row)) in (0u32..).zip(crops.iter().zip(rows)) {
            let y = GAP + place * (CELL + GAP);
            let tendency = row.tendency.colour();
            cell(
                &mut sheet,
                [GAP, y],
                (crop.size, &crop.rgba),
                None,
                tendency,
            );
            for (column, copy) in (1u32..).zip(&row.copies) {
                let x = GAP + column * (CELL + GAP);
                let marked = Some((&crop.areas, copy));
                cell(
                    &mut sheet,
                    [x, y],
                    (copy.size, &copy.pixels),
                    marked,
                    tendency,
                );
            }
            number(&mut sheet, [GAP, y], page * ROWS + place as usize + 1);
        }
        let path = dir.join(format!("key-shifts-{}.png", page + 1));
        write_rgba(&path, [width, height], sheet.as_raw())?;
        println!("{}: {} crops", path.display(), rows.len());
    }
    Ok(())
}

/// A cell at `[x, y]`: the picture `rgba` of `size`, the tendency's disc, and a copy's marks.
fn cell(
    sheet: &mut RgbaImage,
    [x, y]: [u32; 2],
    (size, rgba): ([u32; 2], &[u8]),
    marked: Option<(&Areas, &Painted)>,
    tendency: Rgb8,
) {
    fill(sheet, [x, y], [CELL, CELL], GROUND);
    let Some(thumb) = thumbnail(size, rgba, FilterType::Triangle) else {
        return;
    };
    let left = x + (CELL - thumb.width()) / 2;
    let top = y + (CELL - thumb.height()) / 2;
    imageops::overlay(sheet, &thumb, i64::from(left), i64::from(top));
    let [w, h] = [thumb.width(), thumb.height()].map(f64::from);
    let at = [f64::from(left), f64::from(top)];
    if let Some((areas, copy)) = marked {
        marks(sheet, (at, [w, h]), areas, copy);
    }
    let [r, g, b] = tendency;
    let centre = [at[0] + w - DISC - 6.0, at[1] + DISC + 6.0];
    disc(sheet, centre, DISC + 2.0, WHITE);
    disc(sheet, centre, DISC, Rgba([r, g, b, 255]));
}

/// What Compare marked on `copy` of `areas`, the picture lying at `at`, `size` large.
fn marks(sheet: &mut RgbaImage, (at, size): (Point, Point), areas: &Areas, copy: &Painted) {
    let place = |p: Point| [at[0] + p[0] * size[0], at[1] + p[1] * size[1]];
    let check = &copy.check;
    for area in &check.unmoved {
        draw_in(sheet, &areas.edges(*area, None), &place, (ORANGE, 7.0));
    }
    for area in &check.off_lightness {
        draw_in(sheet, &areas.edges(*area, None), &place, (RED, 3.0));
    }
    for (a, b) in &check.merged {
        draw_in(sheet, &areas.edges(*a, Some(*b)), &place, (BLUE, 6.0));
    }
    for area in &copy.target {
        let [u, v] = middle(areas, *area);
        let centre = place([u, v]);
        ring(sheet, centre, TARGET);
    }
}

/// The middle of `area`'s cells, in shares of the picture.
fn middle(areas: &Areas, area: usize) -> Point {
    let [w, h] = areas.size;
    let cells: Vec<(usize, usize)> = (areas.map().iter().enumerate())
        .filter(|(_, held)| usize::from(**held) == area)
        .map(|(index, _)| (index % w, index / w))
        .collect();
    let count = cells.len().max(1) as f64;
    let sum = cells.iter().fold([0.0, 0.0], |[x, y], (c, r)| {
        [x + *c as f64 + 0.5, y + *r as f64 + 0.5]
    });
    [sum[0] / count / w as f64, sum[1] / count / h as f64]
}

/// A filled disc.
fn disc(sheet: &mut RgbaImage, [cx, cy]: Point, radius: f64, colour: Rgba<u8>) {
    let reach = radius.ceil() as i64;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            if ((dx * dx + dy * dy) as f64) <= radius * radius {
                let (x, y) = (cx as i64 + dx, cy as i64 + dy);
                if x >= 0 && y >= 0 && (x as u32) < sheet.width() && (y as u32) < sheet.height() {
                    sheet.put_pixel(x as u32, y as u32, colour);
                }
            }
        }
    }
}

/// A black ring on white.
fn ring(sheet: &mut RgbaImage, centre: Point, radius: f64) {
    let circle: Vec<Point> = (0..=32)
        .map(|k| {
            let angle = f64::from(k) / 32.0 * std::f64::consts::TAU;
            [
                centre[0] + radius * angle.cos(),
                centre[1] + radius * angle.sin(),
            ]
        })
        .collect();
    draw_in(sheet, std::slice::from_ref(&circle), &|p| p, (WHITE, 5.0));
    draw_in(sheet, std::slice::from_ref(&circle), &|p| p, (BLACK, 2.0));
}
