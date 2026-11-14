//! Practice records in the library's record spaces (D112, `records.rs` their bytes): every
//! sheet read, and what its exercises printed; a new one started with its project and its
//! lesson's givens in one write, the project becoming the open one; a lesson appended with its
//! givens and the sheet's record in one write; a sheet's record replaced whole after each change
//! (its givens stay as they were written); its last lesson cut by the Pomodoro's End with that
//! lesson's givens in one write (`trim`); one removed with its givens and its project.

use openpaint_app::{PortError, ProjectId, ProjectStore, Title};
use openpaint_core::Artwork;
use openpaint_practice::{Given, Givens, Sheet};

use crate::givens::{decode_givens, encode_givens};
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

/// What each exercise of `sheet` printed, in their order, as its lessons' givens were written.
///
/// # Errors
/// Whatever reading the library's records fails with; `Corrupt` when a lesson's givens are
/// missing, do not decode, or miss one of its exercises: the sheet is refused, never remade.
pub fn printed(store: &mut dyn ProjectStore, sheet: &Sheet) -> Result<Vec<Given>, PortError> {
    let corrupt = || PortError::Corrupt {
        what: "practice givens",
    };
    let records = store.records(GIVENS, &sheet.canvas.to_be_bytes())?;
    let mut lessons = Vec::with_capacity(sheet.lessons.len());
    for lesson in 0..sheet.lessons.len() {
        let key = givens_key(sheet.canvas, u16::try_from(lesson).map_err(|_| corrupt())?);
        let (_, value) = (records.iter())
            .find(|(held, _)| *held == key)
            .ok_or_else(corrupt)?;
        lessons.push(decode_givens(value)?);
    }
    Givens::laid(&lessons, sheet).ok_or_else(corrupt)
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

/// A lesson appended to a stored sheet (`Sheet::append`): `sheet`'s record replaced whole and
/// `givens`, what its last lesson prints, written beside it, in one write.
///
/// # Errors
/// Whatever the write fails with; nothing is written then, the sheet stays as it was stored.
pub fn append(
    store: &mut dyn ProjectStore,
    sheet: &Sheet,
    givens: &Givens,
) -> Result<(), PortError> {
    let lesson = u16::try_from(sheet.lessons.len().saturating_sub(1)).unwrap_or(u16::MAX);
    store.put_records(&[
        (PRACTICE, (sheet_key(sheet.id), encode_sheet(sheet))),
        (
            GIVENS,
            (givens_key(sheet.canvas, lesson), encode_givens(givens)),
        ),
    ])?;
    log::info!("practice sheet {} took lesson {lesson}", sheet.id);
    Ok(())
}

/// The Pomodoro's End (S124): `sheet`'s record replaced whole, its last lesson cut to the
/// exercises begun, and that lesson's givens written again holding only the slots its exercises
/// keep, in one write; what was printed for the exercises gone goes with them, and the givens of
/// a lesson End took off whole (one after the sheet's last now) are removed in the same write.
///
/// # Errors
/// Whatever reading the givens or the write fails with; nothing is written then.
pub fn trim(store: &mut dyn ProjectStore, sheet: &Sheet) -> Result<(), PortError> {
    let corrupt = || PortError::Corrupt {
        what: "practice givens",
    };
    let index = sheet.lessons.len().checked_sub(1).ok_or_else(corrupt)?;
    let lesson = u16::try_from(index).map_err(|_| corrupt())?;
    let key = givens_key(sheet.canvas, lesson);
    let records = store.records(GIVENS, &sheet.canvas.to_be_bytes())?;
    let (_, value) = (records.iter())
        .find(|(held, _)| *held == key)
        .ok_or_else(corrupt)?;
    let held: Vec<Vec<u8>> = (0..=lesson)
        .map(|lesson| givens_key(sheet.canvas, lesson))
        .collect();
    let gone: Vec<_> = (records.iter())
        .filter(|(key, _)| !held.contains(key))
        .map(|(key, _)| (GIVENS, key.clone()))
        .collect();
    let mut givens = decode_givens(value)?;
    let kept = &sheet.lessons[index].exercises;
    givens
        .slots
        .retain(|(slot, _)| kept.iter().any(|laid| laid.slot == *slot));
    if givens.slots.len() != kept.len() {
        return Err(corrupt());
    }
    let put = [
        (PRACTICE, (sheet_key(sheet.id), encode_sheet(sheet))),
        (GIVENS, (key, encode_givens(&givens))),
    ];
    store.rewrite_records(&put, &gone)?;
    log::info!(
        "practice sheet {} lesson {lesson} ended at {} exercises, {} lessons' givens gone",
        sheet.id,
        kept.len(),
        gone.len()
    );
    Ok(())
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
