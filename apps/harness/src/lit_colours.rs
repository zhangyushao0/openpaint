//! `lit-colours` (S154j): contact sheets on which colour under light's rules are validated by eye,
//! as key shifts' were (S107). A row a scene, every variant, three seeds each (five where its
//! object's or lamp's hue is drawn): the model's render (one answer), the model painted flat
//! region by region (it breaks no rule), then a copy made wrong on purpose for each rule the level
//! holds (`ColourTask::wrong`), each labelled with the rules its reading breaks; eight rows a sheet
//! on `artifacts/lit-colours/lit-colours-<n>.png`. A line per copy: its scene, the rule it is made
//! for and the rules read broken. Fails when the render or the flat model breaks a rule, or a
//! wrong copy does not break the rule it is made for.

use std::path::Path;

use image::{Rgba, RgbaImage};
use openpaint_practice::{ColourTask, LitColour, Palette, Rgb8, Rule};
use openpaint_sheet::{fill, label};

use crate::error::HarnessError;
use crate::output::{workspace_root, write_rgba};

/// A cell's side and the gap between cells, sheet pixels; rows a sheet; seeds a variant, and a
/// variant whose hue is drawn.
const SIDE: u32 = 256;
const GAP: u32 = 10;
const ROWS: usize = 8;
const SEEDS: u64 = 3;
const HUED: u64 = 5;
const BACKGROUND: Rgba<u8> = Rgba([150, 150, 150, 255]);
/// The sheet's paper, which no copy here leaves.
const PAPER: Rgb8 = [255; 3];

/// A cell: its picture and its label.
struct Cell {
    rgba: Vec<u8>,
    words: String,
}

pub fn run(_args: &[&str]) -> Result<(), HarnessError> {
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut wrong = 0;
    for task in scenes() {
        let (row, missed) = row(&task);
        wrong += missed;
        rows.push(row);
    }
    let dir = workspace_root().join("artifacts").join("lit-colours");
    write_pages(&rows, &dir)?;
    if wrong > 0 {
        return Err(HarnessError::Missing(
            "a render or a flat model breaks a rule, or a wrong copy misses its own",
        ));
    }
    Ok(())
}

/// Every variant, `SEEDS` each, `HUED` where its hue is drawn.
fn scenes() -> Vec<ColourTask> {
    (LitColour::ALL.into_iter())
        .flat_map(|variant| {
            let hued = matches!(variant.palette, Palette::Local | Palette::Bounce);
            let seeds = if hued { HUED } else { SEEDS };
            (0..seeds).map(move |seed| ColourTask::generate(variant, seed))
        })
        .collect()
}

/// The rules `task`'s copy `rgba` (`SIDE` square) breaks, each once.
fn broken(task: &ColourTask, rgba: &[u8]) -> Vec<Rule> {
    let read = task.read([SIDE; 2], rgba, PAPER);
    let mut rules: Vec<Rule> = task
        .broken(&read)
        .into_iter()
        .map(|(rule, _)| rule)
        .collect();
    rules.sort_by_key(|rule| Rule::ALL.iter().position(|r| r == rule));
    rules.dedup();
    rules
}

/// `rules` as a label.
fn words(rules: &[Rule]) -> String {
    let word = |rule: &Rule| match rule {
        Rule::Darker => "DARKER",
        Rule::LightColour => "LIGHT",
        Rule::ShadowColour => "SHADOW",
        Rule::LocalKept => "LOCAL",
        Rule::Bounce => "BOUNCE",
        Rule::Value => "VALUE",
    };
    if rules.is_empty() {
        "NONE".to_owned()
    } else {
        rules.iter().map(word).collect::<Vec<_>>().join(" ")
    }
}

/// `task`'s row: its render, its flat model and its wrong copies, and how many of them read
/// wrongly.
fn row(task: &ColourTask) -> (Vec<Cell>, usize) {
    let name = format!("{:?}", task.variant);
    let render = task.render(SIDE);
    let flat = task.flat(&[], SIDE);
    let mut missed = 0;
    let mut cells = Vec::new();
    for (picture, kind) in [(render, "MODEL"), (flat, "FLAT")] {
        let rules = broken(task, &picture);
        missed += usize::from(!rules.is_empty());
        println!("{name} {kind}: {}", words(&rules));
        let words = format!("{kind} {}", words(&rules));
        cells.push(Cell {
            rgba: picture,
            words,
        });
    }
    for rule in task.rules() {
        let Some(wrong) = task.wrong(rule) else {
            continue;
        };
        let picture = task.flat(&wrong, SIDE);
        let rules = broken(task, &picture);
        missed += usize::from(!rules.contains(&rule));
        println!("{name} made {rule:?}: {}", words(&rules));
        cells.push(Cell {
            rgba: picture,
            words: words(&rules),
        });
    }
    (cells, missed)
}

/// Writes `rows` as sheets `lit-colours-<n>.png` in `dir`.
fn write_pages(rows: &[Vec<Cell>], dir: &Path) -> Result<(), HarnessError> {
    std::fs::create_dir_all(dir)?;
    let columns = rows.iter().map(Vec::len).max().unwrap_or(1) as u32;
    for (page, rows) in rows.chunks(ROWS).enumerate() {
        let count = u32::try_from(rows.len()).unwrap_or(1);
        let [width, height] = [columns * (SIDE + GAP) + GAP, count * (SIDE + GAP) + GAP];
        let mut sheet = RgbaImage::from_pixel(width, height, BACKGROUND);
        for (place, cells) in (0u32..).zip(rows) {
            let y = GAP + place * (SIDE + GAP);
            for (column, cell) in (0u32..).zip(cells) {
                let x = GAP + column * (SIDE + GAP);
                put(&mut sheet, [x, y], cell);
            }
        }
        let path = dir.join(format!("lit-colours-{}.png", page + 1));
        write_rgba(&path, [width, height], sheet.as_raw())?;
        println!("{}: {} scenes", path.display(), rows.len());
    }
    Ok(())
}

/// `cell` at `[x, y]`, its label at its corner.
fn put(sheet: &mut RgbaImage, [x, y]: [u32; 2], cell: &Cell) {
    fill(sheet, [x, y], [SIDE, SIDE], BACKGROUND);
    let (pixels, _) = cell.rgba.as_chunks::<4>();
    for (at, [r, g, b, _]) in pixels.iter().enumerate() {
        let (column, row) = (at as u32 % SIDE, at as u32 / SIDE);
        sheet.put_pixel(x + column, y + row, Rgba([*r, *g, *b, 255]));
    }
    label(sheet, [x, y], &cell.words);
}
