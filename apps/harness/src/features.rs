//! `features <library dir>` (S58): what each artwork of a library uses, so the render core's
//! rewrite covers what the author paints with first. Opens the library through the store as the
//! app does (an older schema is upgraded in place), so point it at a copy. Per artwork and over
//! all: layers (hidden, locked, Alpha Lock, clipping), groups, blend modes, masks; strokes by
//! brush (a recipe equal to a built-in's, else "edited or older") and by tool, smudges, strokes
//! inside a selection; fills, transform records, pictures (pixel patches: inserted pictures and
//! S20's transforms), adjustments, filters, cleared selections and nested records (Merge Down,
//! pasted copies), counted inside nested records too. Counts only: no titles.

use std::collections::BTreeMap;
use std::path::Path;

use openpaint_app::{BrushLibrary, ProjectStore};
use openpaint_core::{
    Adjustment, Artwork, BlendMode, BrushRecipe, Deposit, RegionEdit, Stroke, StrokeList,
    StrokeTool,
};
use openpaint_store::Library;

use crate::error::HarnessError;

/// What one artwork (or all of them) uses.
#[derive(Debug, Default, Clone)]
struct Uses {
    layers: usize,
    hidden: usize,
    locked: usize,
    alpha_locked: usize,
    clipping: usize,
    groups: usize,
    masks: usize,
    blends: BTreeMap<&'static str, usize>,
    brushes: BTreeMap<String, usize>,
    tools: BTreeMap<&'static str, usize>,
    on_masks: usize,
    clipped_strokes: usize,
    samples: usize,
    fills: usize,
    records: usize,
    pictures: usize,
    adjustments: usize,
    filters: usize,
    cleared: usize,
    nested: usize,
}

pub fn run(dir: &str) -> Result<(), HarnessError> {
    if !Path::new(dir).is_dir() {
        return Err(HarnessError::Missing("features needs a library directory"));
    }
    let built_in = BrushLibrary::built_in()
        .map_err(|_| HarnessError::Missing("the built-in brushes did not load"))?;
    let known: Vec<(String, BrushRecipe)> = (built_in.brushes().iter())
        .map(|brush| (brush.name().to_owned(), (**brush.recipe()).clone()))
        .collect();
    let mut library = Library::at(dir);
    let listed = library.list().map_err(HarnessError::Port)?;
    let mut all = Uses::default();
    for summary in &listed {
        let id = summary.id.get();
        let artwork = match library.load(summary.id) {
            Ok(project) => project.artwork,
            Err(error) => {
                println!("artwork {id}: not opened ({error})");
                continue;
            }
        };
        let uses = of(&artwork, &known);
        let canvas = artwork.canvas();
        println!("artwork {id}: {} x {}", canvas.width(), canvas.height());
        print(&uses);
        add(&mut all, &uses);
    }
    println!("all {} artworks:", listed.len());
    print(&all);
    Ok(())
}

/// What `artwork` uses, its strokes' brushes named by `known`.
fn of(artwork: &Artwork, known: &[(String, BrushRecipe)]) -> Uses {
    let mut uses = Uses {
        layers: artwork.layers().len(),
        groups: artwork.groups().len(),
        ..Uses::default()
    };
    for layer in artwork.layers() {
        let props = layer.props();
        uses.hidden += usize::from(!props.visible());
        uses.locked += usize::from(props.locked());
        uses.alpha_locked += usize::from(props.alpha_locked());
        uses.clipping += usize::from(props.clipping());
        *uses.blends.entry(blend(props.blend())).or_default() += 1;
        content(&mut uses, (layer.strokes(), layer.edits()), known);
        if let Some(mask) = layer.mask() {
            uses.masks += 1;
            uses.on_masks += mask.strokes().iter().count();
            content(&mut uses, (mask.strokes(), mask.edits()), known);
        }
    }
    uses
}

/// Counts the strokes and edits of one layer, mask or nested record, nested records' within.
fn content(
    uses: &mut Uses,
    (strokes, edits): (&StrokeList, &[RegionEdit]),
    known: &[(String, BrushRecipe)],
) {
    for stroke in strokes.iter() {
        count_stroke(uses, stroke, known);
    }
    for edit in edits {
        match edit {
            RegionEdit::Fill(_) => uses.fills += 1,
            RegionEdit::Transform(transform) if transform.patch().is_some() => uses.pictures += 1,
            RegionEdit::Transform(_) => uses.records += 1,
            RegionEdit::Adjust(adjust) => match adjust.adjustment() {
                Adjustment::Blur(_) | Adjustment::Sharpen(_) => uses.filters += 1,
                Adjustment::Hsb(_) | Adjustment::Curves(_) => uses.adjustments += 1,
            },
            RegionEdit::Clear(_) => uses.cleared += 1,
            RegionEdit::Nested(nested) => {
                uses.nested += 1;
                let layer = nested.layer();
                content(uses, (layer.strokes(), layer.edits()), known);
                if let Some(mask) = layer.mask() {
                    uses.masks += 1;
                    content(uses, (mask.strokes(), mask.edits()), known);
                }
            }
        }
    }
}

fn count_stroke(uses: &mut Uses, stroke: &Stroke, known: &[(String, BrushRecipe)]) {
    let style = stroke.style();
    let recipe = &**style.recipe();
    let brush = known
        .iter()
        .find(|(_, built_in)| built_in == recipe)
        .map_or_else(
            || format!("other {}", engine(recipe)),
            |(name, _)| name.clone(),
        );
    *uses.brushes.entry(brush).or_default() += 1;
    let tool = match style.tool() {
        StrokeTool::Paint => "paint",
        StrokeTool::Erase => "erase",
        StrokeTool::Smudge => "smudge",
    };
    *uses.tools.entry(tool).or_default() += 1;
    uses.clipped_strokes += usize::from(stroke.clip().is_some());
    uses.samples += stroke.samples().len();
}

/// What the evaluator does for a recipe no built-in has (an edited brush, or a built-in as an
/// older build drew it): stamps, or a continuous stroke covering, building up, or on paper.
fn engine(recipe: &BrushRecipe) -> String {
    let shape = recipe.shape();
    if shape.is_stamped() {
        return format!("(stamped {:?})", shape.stamp());
    }
    match recipe.deposit() {
        Deposit::Cover => "(continuous, covering)".to_owned(),
        Deposit::Accumulate(built) if built.paper().is_some() => "(continuous, paper)".to_owned(),
        Deposit::Accumulate(built) => format!("(continuous, building up {:?})", built.tip()),
    }
}

const fn blend(mode: BlendMode) -> &'static str {
    match mode {
        BlendMode::Normal => "Normal",
        BlendMode::Multiply => "Multiply",
        BlendMode::Screen => "Screen",
        BlendMode::Overlay => "Overlay",
    }
}

fn add(all: &mut Uses, one: &Uses) {
    let sum = |a: &mut usize, b: usize| *a += b;
    sum(&mut all.layers, one.layers);
    sum(&mut all.hidden, one.hidden);
    sum(&mut all.locked, one.locked);
    sum(&mut all.alpha_locked, one.alpha_locked);
    sum(&mut all.clipping, one.clipping);
    sum(&mut all.groups, one.groups);
    sum(&mut all.masks, one.masks);
    sum(&mut all.on_masks, one.on_masks);
    sum(&mut all.clipped_strokes, one.clipped_strokes);
    sum(&mut all.samples, one.samples);
    sum(&mut all.fills, one.fills);
    sum(&mut all.records, one.records);
    sum(&mut all.pictures, one.pictures);
    sum(&mut all.adjustments, one.adjustments);
    sum(&mut all.filters, one.filters);
    sum(&mut all.cleared, one.cleared);
    sum(&mut all.nested, one.nested);
    for (blend, count) in &one.blends {
        *all.blends.entry(blend).or_default() += count;
    }
    for (brush, count) in &one.brushes {
        *all.brushes.entry(brush.clone()).or_default() += count;
    }
    for (tool, count) in &one.tools {
        *all.tools.entry(tool).or_default() += count;
    }
}

fn print(uses: &Uses) {
    let list = |map: Vec<(String, usize)>| -> String {
        let words: Vec<String> = map.iter().map(|(k, v)| format!("{k} {v}")).collect();
        words.join(", ")
    };
    let strokes: usize = uses.tools.values().sum();
    println!(
        "  layers {} (hidden {}, locked {}, Alpha Lock {}, clipping {}), groups {}, masks {}, \
         modes: {}",
        uses.layers,
        uses.hidden,
        uses.locked,
        uses.alpha_locked,
        uses.clipping,
        uses.groups,
        uses.masks,
        list(
            uses.blends
                .iter()
                .map(|(k, v)| ((*k).to_owned(), *v))
                .collect()
        )
    );
    println!(
        "  strokes {strokes} ({} samples; on masks {}, inside a selection {}): {}",
        uses.samples,
        uses.on_masks,
        uses.clipped_strokes,
        list(
            uses.tools
                .iter()
                .map(|(k, v)| ((*k).to_owned(), *v))
                .collect()
        )
    );
    println!(
        "  by brush: {}",
        list(uses.brushes.iter().map(|(k, v)| (k.clone(), *v)).collect())
    );
    println!(
        "  fills {}, transform records {}, pictures {}, adjustments {}, filters {}, cleared {}, \
         nested records {}",
        uses.fills,
        uses.records,
        uses.pictures,
        uses.adjustments,
        uses.filters,
        uses.cleared,
        uses.nested
    );
}
