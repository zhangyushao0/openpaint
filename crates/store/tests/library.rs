//! The library on disk: what is saved opens again exactly, a save touches only the records that
//! changed, unreadable data is refused with a typed error, and a failed call is retried by the
//! next one.

mod common;

use std::sync::Arc;

use common::{fresh_dir, records, untitled, write_records};
use openpaint_app::{BrushLibrary, PortError, ProjectStore};
use openpaint_core::{
    Artwork, ArtworkCommand, CanvasPoint, CanvasSize, Color, LayerCommand, MaskCommand,
    StrokeSample, StrokeStyle,
};
use openpaint_store::Library;

/// The built-in pencil (curves, deposit and paper all stored) at `size`, black.
fn pencil(size: f32) -> StrokeStyle {
    let library = BrushLibrary::built_in().expect("built-in brushes");
    let recipe = Arc::clone(library.brushes()[0].recipe());
    StrokeStyle::new(recipe, size, 1.0, Color::BLACK).expect("style")
}

fn blank() -> Artwork {
    Artwork::new(CanvasSize::new(320, 200).expect("size"))
}

/// `artwork` with one more stroke on the active layer, from `x` rightwards over `samples`.
fn draw(artwork: &Artwork, x: f32, samples: u16) -> Artwork {
    let style = pencil(12.0);
    let sample = |i: u16| {
        let point = CanvasPoint::new(x + f32::from(i), 20.0 + f32::from(i % 7)).expect("point");
        StrokeSample::new(point, 0.5, 0.2, -1.0, u32::from(i) * 4).expect("sample")
    };
    let mut session = artwork.begin_stroke(style, sample(0)).expect("begin");
    let rest: Vec<StrokeSample> = (1..samples).map(sample).collect();
    session.extend(&rest).expect("extend");
    session.finish().artwork
}

fn reopened(dir: &std::path::Path) -> Artwork {
    Library::at(dir)
        .open_last()
        .expect("opens")
        .expect("a project")
        .artwork
}

/// How many records of `kind` (`b's'` strokes, `b'f'` fills) the library holds.
fn kind_records(dir: &std::path::Path, kind: u8) -> usize {
    records(dir)
        .iter()
        .filter(|(keyspace, key, _)| keyspace == "projects" && key.len() == 25 && key[8] == kind)
        .count()
}

fn stroke_records(dir: &std::path::Path) -> usize {
    kind_records(dir, b's')
}

#[test]
fn fills_are_saved_by_their_own_records_and_open_again_exactly() {
    let dir = fresh_dir("fills");
    let one = draw(&blank(), 10.0, 20);
    let mut inside = vec![false; 320 * 200];
    inside[30 * 320 + 40..][..100].fill(true);
    inside[31 * 320 + 40..][..100].fill(true);
    let mask = openpaint_core::RegionMask::from_pixels(one.canvas(), &inside).expect("mask");
    let red = Color::rgb(1.0, 0.0, 0.0).expect("red");
    let filled = one.fill_region(mask, red).expect("fill").artwork;
    let later = draw(&filled, 60.0, 20);
    let mut library = Library::at(&dir);
    let id = library.create(&one, &untitled()).expect("create");
    for (version, strokes, fills) in [(&filled, 1, 1), (&later, 2, 1), (&one, 1, 0)] {
        library.save(id, version).expect("save");
        drop(library);
        assert_eq!(reopened(&dir).to_parts(), version.to_parts());
        assert_eq!(
            (stroke_records(&dir), kind_records(&dir, b'f')),
            (strokes, fills)
        );
        library = Library::at(&dir);
        assert!(library.open_last().expect("reopen").is_some());
    }
}

#[test]
fn masks_are_saved_by_their_own_records_and_deleting_one_removes_them() {
    let dir = fresh_dir("masks");
    let one = draw(&blank(), 10.0, 20);
    let layer = one.active();
    let mask = |command| one_more(&one, command);
    let added = mask(MaskCommand::Add(layer));
    let painted = draw(&added, 30.0, 20);
    let inverted = one_more(&painted, MaskCommand::Invert(layer));
    let off = one_more(
        &inverted,
        MaskCommand::SetEnabled {
            layer,
            enabled: false,
        },
    );
    let deleted = one_more(&off, MaskCommand::Delete(layer));
    let mut library = Library::at(&dir);
    let id = library.create(&one, &untitled()).expect("create");
    for (version, strokes) in [
        (&added, 1),
        (&painted, 2),
        (&inverted, 2),
        (&off, 2),
        (&deleted, 1),
        (&painted, 2),
    ] {
        library.save(id, version).expect("save");
        drop(library);
        assert_eq!(reopened(&dir).to_parts(), version.to_parts());
        assert_eq!(stroke_records(&dir), strokes);
        library = Library::at(&dir);
        assert!(library.open_last().expect("reopen").is_some());
    }
    drop(library);
    assert!(reopened(&dir).on_mask(), "the mask stays the target");
}

fn one_more(artwork: &Artwork, command: MaskCommand) -> Artwork {
    let applied = artwork.apply(ArtworkCommand::Mask(command));
    applied.expect("mask command").artwork
}

#[test]
fn an_empty_library_has_no_last_project() {
    let dir = fresh_dir("empty");
    assert_eq!(Library::at(&dir).open_last(), Ok(None));
}

#[test]
fn every_saved_version_opens_again_exactly() {
    let dir = fresh_dir("versions");
    let one = draw(&blank(), 10.0, 40);
    let two = draw(&one, 50.0, 300);
    let branch = draw(&one, 90.0, 5);
    let layered = branch
        .apply(ArtworkCommand::Layer(LayerCommand::Create))
        .expect("create")
        .artwork;
    let on_top = draw(&layered, 120.0, 12);
    let bottom = on_top.layers()[0].id();
    let deleted = on_top
        .apply(ArtworkCommand::Layer(LayerCommand::Delete(bottom)))
        .expect("delete")
        .artwork;
    let mut library = Library::at(&dir);
    let id = library.create(&one, &untitled()).expect("create");
    for (version, strokes) in [
        (&two, 2),
        (&one, 1),
        (&branch, 2),
        (&on_top, 3),
        (&deleted, 1),
    ] {
        library.save(id, version).expect("save");
        drop(library);
        assert_eq!(reopened(&dir).to_parts(), version.to_parts());
        assert_eq!(stroke_records(&dir), strokes, "no stale stroke records");
        library = Library::at(&dir);
        assert!(library.open_last().expect("reopen").is_some());
    }
}

#[test]
fn layer_edits_open_again_exactly_and_an_undone_delete_restores_the_strokes() {
    let dir = fresh_dir("layers");
    let layer = |artwork: &Artwork, command| {
        artwork
            .apply(ArtworkCommand::Layer(command))
            .expect("layer command")
            .artwork
    };
    let bottom_drawn = draw(&draw(&blank(), 10.0, 30), 40.0, 30);
    let bottom = bottom_drawn.active();
    let two = draw(&layer(&bottom_drawn, LayerCommand::Create), 80.0, 20);
    let top = two.active();
    let hidden = layer(
        &two,
        LayerCommand::SetVisible {
            layer: bottom,
            visible: false,
        },
    );
    let faded = layer(
        &hidden,
        LayerCommand::SetOpacity {
            layer: top,
            opacity: 0.35,
        },
    );
    let lowered = layer(
        &faded,
        LayerCommand::Reorder {
            layer: top,
            index: 0,
        },
    );
    let chosen = layer(&lowered, LayerCommand::Select(bottom));
    let deleted = layer(&chosen, LayerCommand::Delete(bottom));
    let mut library = Library::at(&dir);
    let id = library.create(&bottom_drawn, &untitled()).expect("create");
    // The last one is undo after the delete: the deleted layer's strokes come back.
    for (version, strokes) in [
        (&two, 3),
        (&hidden, 3),
        (&faded, 3),
        (&lowered, 3),
        (&chosen, 3),
        (&deleted, 1),
        (&chosen, 3),
    ] {
        library.save(id, version).expect("save");
        drop(library);
        assert_eq!(reopened(&dir).to_parts(), version.to_parts());
        assert_eq!(stroke_records(&dir), strokes);
        library = Library::at(&dir);
        assert!(library.open_last().expect("reopen").is_some());
    }
}

#[test]
fn a_save_without_a_known_base_rewrites_the_project() {
    let dir = fresh_dir("rewrite");
    let one = draw(&blank(), 10.0, 20);
    let three = draw(&draw(&one, 40.0, 20), 70.0, 20);
    let mut first = Library::at(&dir);
    let id = first.create(&three, &untitled()).expect("create");
    drop(first);
    // A library that never read the project saves an older version over it.
    let mut second = Library::at(&dir);
    second.save(id, &one).expect("save");
    drop(second);
    assert_eq!(reopened(&dir).to_parts(), one.to_parts());
    assert_eq!(stroke_records(&dir), 1);
}

fn corrupted(name: &str, change: impl Fn(&mut Vec<(String, Vec<u8>, Vec<u8>)>)) -> PortError {
    let source = fresh_dir(&format!("{name}-source"));
    let artwork = draw(&draw(&blank(), 10.0, 20), 40.0, 20);
    Library::at(&source)
        .create(&artwork, &untitled())
        .expect("create");
    let mut stored = records(&source);
    change(&mut stored);
    let dir = fresh_dir(name);
    write_records(&dir, &stored);
    Library::at(&dir).open_last().expect_err("refused")
}

fn head(records: &mut [(String, Vec<u8>, Vec<u8>)]) -> &mut Vec<u8> {
    let head = records
        .iter_mut()
        .find(|(keyspace, key, _)| keyspace == "projects" && key.len() == 9);
    &mut head.expect("head record").2
}

#[test]
fn unreadable_records_are_refused_with_a_reason() {
    let newer = corrupted("newer", |records| head(records)[0] = 24);
    assert_eq!(
        newer,
        PortError::NewerSchema {
            found: 24,
            newest: 23
        }
    );
    let newer_library = corrupted("newer-library", |records| {
        let schema = records.iter_mut().find(|(_, key, _)| key == b"schema");
        schema.expect("schema record").2 = 24u32.to_le_bytes().to_vec();
    });
    assert_eq!(
        newer_library,
        PortError::NewerSchema {
            found: 24,
            newest: 23
        }
    );
    let short = corrupted("short", |records| {
        head(records).pop();
    });
    assert_eq!(
        short,
        PortError::Corrupt {
            what: "artwork head"
        }
    );
    let missing = corrupted("missing", |records| {
        let last = records.iter().rposition(|(_, key, _)| key.len() == 25);
        records.remove(last.expect("a stroke record"));
    });
    assert_eq!(
        missing,
        PortError::Corrupt {
            what: "stroke records"
        }
    );
    let invalid = corrupted("invalid", |records| {
        // The canvas width: zero breaks an artwork rule, not the encoding.
        head(records)[4..8].copy_from_slice(&0u32.to_le_bytes());
    });
    assert!(matches!(invalid, PortError::Invalid(_)), "{invalid}");
}

#[test]
fn the_toolbox_is_stored_apart_and_an_unreadable_one_never_blocks_an_artwork() {
    let dir = fresh_dir("editor");
    let mut library = Library::at(&dir);
    let artwork = draw(&blank(), 10.0, 5);
    library.create(&artwork, &untitled()).expect("create");
    assert_eq!(library.editor(), Ok(None));
    let choice = openpaint_app::BrushChoice {
        brush: "pencil".into(),
        brushes: Vec::new(),
    };
    let state = openpaint_app::EditorState {
        tool: openpaint_app::Tool::Eraser,
        paint: choice.clone(),
        erase: choice.clone(),
        smudge: choice,
        color: [0.5, 0.25, 1.0],
        recent: vec![[0.5, 0.25, 1.0]],
        buttons: openpaint_app::ButtonMap {
            primary: openpaint_app::ButtonAction::Redo,
            secondary: openpaint_app::ButtonAction::Nothing,
            tertiary: openpaint_app::ButtonAction::Eyedropper,
        },
        swatches: vec![[0.0, 0.0, 0.0]],
        library: Vec::new(),
        sets: [vec!["pencil".into()], Vec::new(), Vec::new(), Vec::new()],
        recent_brushes: vec!["pencil".into()],
    };
    library.save_editor(&state).expect("toolbox");
    assert_eq!(library.editor(), Ok(Some(state)));
    drop(library);
    let mut stored = records(&dir);
    let editor = stored.iter_mut().find(|(_, key, _)| key == b"editor");
    editor.expect("the toolbox record").2.truncate(3);
    let damaged = fresh_dir("editor-damaged");
    write_records(&damaged, &stored);
    let mut library = Library::at(&damaged);
    let corrupt = PortError::Corrupt {
        what: "editor state",
    };
    assert_eq!(library.editor(), Err(corrupt));
    let reopened = reopened_with(&mut library);
    assert_eq!(reopened.to_parts(), artwork.to_parts());
}

#[test]
fn a_failed_call_is_retried_by_the_next_one() {
    let dir = fresh_dir("locked");
    let mut holder = Library::at(&dir);
    let id = holder
        .create(&draw(&blank(), 10.0, 5), &untitled())
        .expect("create");
    // Another library on the same directory cannot open it while the first holds it.
    let mut other = Library::at(&dir);
    let later = draw(&reopened_with(&mut holder), 30.0, 5);
    let refused = other.save(id, &later);
    assert!(
        matches!(refused, Err(PortError::Device { .. })),
        "{refused:?}"
    );
    drop(holder);
    other.save(id, &later).expect("the retry opens the library");
    drop(other);
    assert_eq!(reopened(&dir).to_parts(), later.to_parts());
}

fn reopened_with(library: &mut Library) -> Artwork {
    library
        .open_last()
        .expect("opens")
        .expect("a project")
        .artwork
}
