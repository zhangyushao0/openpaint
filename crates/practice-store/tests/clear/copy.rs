//! The clear on a copy of a real library (S152): run by hand on an extracted backup's `files`,
//! never on a backup itself (`OPENPAINT_LIBRARY_COPY`).

use std::collections::BTreeSet;
use std::path::Path;

use openpaint_app::{ProjectId, ProjectStore, Title};
use openpaint_practice_store::{Packs, blocks, clear, sheets};
use openpaint_store::Library;

use crate::common::records;
use crate::{Records, without_practice};

/// Every file in `dir` with its bytes, by name; none for a folder missing.
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(String, Vec<u8>)> = (entries.flatten())
        .map(|entry| {
            let bytes = std::fs::read(entry.path()).expect("a pack's bytes");
            (entry.file_name().to_string_lossy().into_owned(), bytes)
        })
        .collect();
    found.sort();
    found
}

/// How many records `all` holds in each of practice's spaces and of the projects' keyspace.
fn counted(all: &Records) -> String {
    let space = |byte: u8| {
        (all.iter())
            .filter(|(keyspace, key, _)| keyspace == "library" && key.first() == Some(&byte))
            .count()
    };
    let projects = all.iter().filter(|(keyspace, ..)| keyspace == "projects");
    format!(
        "p {} g {} t {} b {}, projects' records {}",
        space(b'p'),
        space(b'g'),
        space(b't'),
        space(b'b'),
        projects.count()
    )
}

/// The projects the sheets' records in `before` name in their heads, every lesson's givens
/// naming one of them.
fn sheet_heads(before: &Records) -> BTreeSet<u64> {
    let in_space = |key: &[u8], byte: u8| key.first() == Some(&byte);
    let heads: BTreeSet<u64> = (before.iter())
        .filter(|(keyspace, key, value)| {
            keyspace == "library" && in_space(key, b'p') && value.first() == Some(&0x53)
        })
        .filter_map(|(_, _, value)| Some(u64::from_le_bytes(value.get(1..9)?.try_into().ok()?)))
        .collect();
    let given: BTreeSet<u64> = (before.iter())
        .filter(|(keyspace, key, _)| {
            keyspace == "library" && in_space(key, b'g') && key.len() == 11
        })
        .filter_map(|(_, key, _)| Some(u64::from_be_bytes(key.get(1..9)?.try_into().ok()?)))
        .collect();
    let (by_heads, by_givens) = (heads.len(), given.len());
    println!("sheets' projects by their records {by_heads}, by their givens {by_givens}");
    assert!(
        given.is_subset(&heads),
        "every lesson's givens of a sheet with a record"
    );
    heads
}

/// The projects `library` lists but `sheets`, with their titles and OpenPaint files.
fn artworks(library: &mut Library, sheets: &BTreeSet<u64>) -> Vec<(ProjectId, Title, Vec<u8>)> {
    let listed = library.list().expect("listed");
    let kept = listed.into_iter().filter(|p| !sheets.contains(&p.id.get()));
    kept.map(|p| {
        (
            p.id,
            p.title,
            library.artwork_file(p.id).expect("an artwork's file"),
        )
    })
    .collect()
}

/// The clear on a copy of a real library (S152's proof on the author's backup): the folder
/// `OPENPAINT_LIBRARY_COPY` names holds `library` and `packs` as a backup's `files` does. Run by
/// hand on an extracted copy, never on a backup itself: every record not practice's keeps its
/// bytes, every artwork reads back as before, the packs stay, practice is gone, the first lesson
/// is next.
#[test]
#[ignore = "a copy of a real library, named by OPENPAINT_LIBRARY_COPY"]
fn the_clear_on_a_copy_of_a_real_library() {
    let copy = std::env::var("OPENPAINT_LIBRARY_COPY").expect("the copy's folder");
    let (dir, packs) = (
        Path::new(&copy).join("library"),
        Path::new(&copy).join("packs"),
    );
    let (before, packs_before) = (records(&dir), files(&packs));
    println!("before: {}", counted(&before));
    let heads = sheet_heads(&before);
    let mut library = Library::at(&dir);
    let kept = artworks(&mut library, &heads);
    let read = sheets(&mut library).expect("read").len();
    println!("artworks {}, sheets read by this reader {read}", kept.len());
    let timing = blocks(&mut library).expect("the Pomodoro's").timing;

    let cleared = clear(&mut library).expect("cleared");
    println!("{cleared:?}");
    let projects: Vec<u64> = cleared.projects.iter().map(|id| id.get()).collect();
    assert_eq!(projects, heads.iter().copied().collect::<Vec<_>>());
    assert_eq!(
        artworks(&mut library, &heads),
        kept,
        "the artworks alone, as before"
    );
    for (id, ..) in &kept {
        library.artwork(*id).expect("the new reader reads it");
    }
    crate::no_practice_left(&mut library, timing);
    let kept_packs = Packs::at(&packs).list().expect("the packs").len();
    drop(library);

    let after = records(&dir);
    println!("after: {}; packs {kept_packs}", counted(&after));
    let expected = without_practice(&before, &projects);
    assert_eq!(after, expected, "every other record's bytes");
    assert_eq!(
        files(&packs),
        packs_before,
        "the packs and own pictures as they were"
    );
}
