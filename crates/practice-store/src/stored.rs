//! Practice records in the library's record space (D112, `records.rs` their bytes): every
//! session, drill and colour copy read; a new one started with its sheet in one write, the sheet a
//! project its record owns that becomes the open one; a record replaced whole after each change;
//! one removed with its sheet.

use openpaint_app::{PortError, ProjectId, ProjectStore, Title};
use openpaint_core::Artwork;
use openpaint_practice::SessionRecord;

use crate::records::{PRACTICE, decode_session, encode_session, practice_key};

/// Every practice session, oldest first; a record that cannot be read is left out (logged).
///
/// # Errors
/// Whatever reading the library's records fails with.
pub fn sessions(store: &mut dyn ProjectStore) -> Result<Vec<SessionRecord>, PortError> {
    let mut sessions = Vec::new();
    for (key, value) in store.records(PRACTICE)? {
        match decode_session(&key, &value) {
            Ok(session) => sessions.push(session),
            Err(error) => log::warn!("a practice record is not read: {error}"),
        }
    }
    Ok(sessions)
}

/// A new practice session: `sheet` as a new project titled `title`, which becomes the open one,
/// and `session` with that project as its canvas, in one write. Returns the project.
///
/// # Errors
/// Whatever the write fails with; nothing is written then.
pub fn start(
    store: &mut dyn ProjectStore,
    (sheet, title): (&Artwork, &Title),
    session: &SessionRecord,
) -> Result<ProjectId, PortError> {
    let record = |id: ProjectId| {
        let named = SessionRecord {
            canvas: id.get(),
            ..session.clone()
        };
        (practice_key(session.id), encode_session(&named))
    };
    let id = store.create_owned((sheet, title), PRACTICE, &record)?;
    log::info!(
        "practice session {} started on project {}",
        session.id,
        id.get()
    );
    Ok(id)
}

/// `session`'s stored record replaced whole (by its id); its sheet is saved as any artwork.
///
/// # Errors
/// Whatever the write fails with; the record stays as it was then.
pub fn save(store: &mut dyn ProjectStore, session: &SessionRecord) -> Result<(), PortError> {
    store.put_record(
        PRACTICE,
        &practice_key(session.id),
        &encode_session(session),
    )
}

/// `session`'s record (by its id) and its sheet, everything stored for it, removed for good in
/// one write; every other record stays as it is. A colour copy removed from the practice
/// options, and the test-session chore's (debug builds) undoing of a session it started.
///
/// # Errors
/// Whatever the write fails with; nothing is removed then.
pub fn remove(store: &mut dyn ProjectStore, session: &SessionRecord) -> Result<(), PortError> {
    let sheet = (session.canvas != 0).then(|| ProjectId::new(session.canvas));
    store.remove_owned(PRACTICE, &practice_key(session.id), sheet)?;
    log::info!(
        "practice session {} and its sheet {} removed",
        session.id,
        session.canvas
    );
    Ok(())
}
