//! Practice records in the library's record spaces (D112, `records.rs` their bytes): every
//! sheet read; a new one started with its project and its lesson's givens in one write, the
//! project becoming the open one; a sheet's record replaced whole after each change (its givens
//! stay as they were written); one removed with its givens and its project.

use openpaint_app::{PortError, ProjectId, ProjectStore, Title};
use openpaint_core::Artwork;
use openpaint_practice::{Givens, Sheet};

use crate::givens::encode_givens;
use crate::records::{GIVENS, PRACTICE, decode_sheet, encode_sheet, givens_key, sheet_key};

/// Every practice sheet, oldest first; a record that cannot be read is left out (logged).
///
/// # Errors
/// Whatever reading the library's records fails with.
pub fn sheets(store: &mut dyn ProjectStore) -> Result<Vec<Sheet>, PortError> {
    let mut sheets = Vec::new();
    for (key, value) in store.records(PRACTICE, &[])? {
        match decode_sheet(&key, &value) {
            Ok(sheet) => sheets.push(sheet),
            Err(error) => log::warn!("a practice record is not read: {error}"),
        }
    }
    Ok(sheets)
}

/// A new practice sheet: `artwork` as a new project titled `title`, which becomes the open one,
/// `sheet` with that project as its canvas, and `givens`, what its lesson in hand printed, in one
/// write. Returns the project.
///
/// # Errors
/// Whatever the write fails with; nothing is written then.
pub fn start(
    store: &mut dyn ProjectStore,
    (artwork, title): (&Artwork, &Title),
    sheet: &Sheet,
    givens: &Givens,
) -> Result<ProjectId, PortError> {
    let lesson = u16::try_from(sheet.lessons.len().saturating_sub(1)).unwrap_or(u16::MAX);
    let records = |id: ProjectId| {
        let named = Sheet {
            canvas: id.get(),
            ..sheet.clone()
        };
        vec![
            (PRACTICE, (sheet_key(sheet.id), encode_sheet(&named))),
            (
                GIVENS,
                (givens_key(id.get(), lesson), encode_givens(givens)),
            ),
        ]
    };
    let id = store.create_owned((artwork, title), &records)?;
    log::info!(
        "practice sheet {} started on project {}",
        sheet.id,
        id.get()
    );
    Ok(id)
}

/// `sheet`'s stored record replaced whole (by its id); its project is saved as any artwork.
///
/// # Errors
/// Whatever the write fails with; the record stays as it was then.
pub fn save(store: &mut dyn ProjectStore, sheet: &Sheet) -> Result<(), PortError> {
    let record = (sheet_key(sheet.id), encode_sheet(sheet));
    store.put_records(&[(PRACTICE, record)])
}

/// `sheet`'s record (by its id), its lessons' givens and its project, everything stored for it,
/// removed for good in one write; every other record stays as it is. A colour copy removed from
/// the practice options, and the test-session chore's (debug builds) undoing of a sheet it
/// started.
///
/// # Errors
/// Whatever the write fails with; nothing is removed then.
pub fn remove(store: &mut dyn ProjectStore, sheet: &Sheet) -> Result<(), PortError> {
    let project = (sheet.canvas != 0).then(|| ProjectId::new(sheet.canvas));
    let mut keys = vec![(PRACTICE, sheet_key(sheet.id))];
    if sheet.canvas != 0 {
        let lessons = 0..u16::try_from(sheet.lessons.len()).unwrap_or(u16::MAX);
        keys.extend(lessons.map(|lesson| (GIVENS, givens_key(sheet.canvas, lesson))));
    }
    store.remove_owned(&keys, project)?;
    log::info!(
        "practice sheet {} and its project {} removed",
        sheet.id,
        sheet.canvas
    );
    Ok(())
}
