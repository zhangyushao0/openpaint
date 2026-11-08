//! The practice records' upgrade (D112), the one way old practice records are made current:
//! registered with the library by `library_at`, which every host and tool opens a library with,
//! it runs each time the library opens, after the store's format check and before any call.
//!
//! Every record written before 1.7.0, which keeps no time, is given the spans its compares' times
//! tell (`SessionRecord::time_of_compares`, S89); one holding a value scale as 1.5.0 to 1.14.0
//! stored it gets the scale's cells in its kind (S97); each is written again as this version
//! writes it, all in one batch. One written before 1.17.0 has its sheet laid anew as the packs
//! hold its pictures (none: no pack), its sheet and record in a batch of their own (`frames.rs`,
//! S99); while the packs cannot be listed, or its batch fails, it waits as it was (logged) and the
//! library opens all the same. A record that cannot be read is left as it is (`sessions` leaves
//! it out).

use std::path::PathBuf;

use openpaint_app::PortError;
use openpaint_store::{Library, Upgrade, Upgrading};

use crate::frames;
use crate::packs::Packs;
use crate::records::{PRACTICE, decode_session, encode_session};

/// The library in `dir` as the app opens it: with the practice records' upgrade, which lays old
/// sheets anew by the pictures the packs in `packs` hold (none: no pack). Every host and tool
/// opens a library through it, so no old record is left unread.
#[must_use]
pub fn library_at(dir: impl Into<PathBuf>, packs: Option<Packs>) -> Library {
    Library::at(dir).with_upgrade(Box::new(Records { packs }))
}

/// The practice records' upgrade, with the packs whose pictures old sheets are laid anew by.
struct Records {
    packs: Option<Packs>,
}

impl Upgrade for Records {
    fn upgrade(&self, library: &Upgrading<'_>) -> Result<(), PortError> {
        let mut upgraded = Vec::new();
        let mut unlaid = Vec::new();
        for (key, value) in library.records(PRACTICE)? {
            let Ok((mut session, owed)) = decode_session(&key, &value) else {
                continue;
            };
            if owed.time {
                session.time = session.time_of_compares();
            }
            if owed.frames {
                unlaid.push(session);
            } else if owed.time || owed.scales {
                upgraded.push((key, encode_session(&session)));
            }
        }
        if !upgraded.is_empty() {
            library.put(PRACTICE, &upgraded)?;
            log::info!(
                "{} practice records upgraded (their time, their value scales' cells)",
                upgraded.len()
            );
        }
        if unlaid.is_empty() {
            return Ok(());
        }
        let held = match self.packs.as_ref().map(Packs::list).transpose() {
            Ok(held) => held.unwrap_or_default(),
            Err(error) => {
                log::warn!("practice sheets not laid anew: the packs are not listed: {error}");
                return Ok(());
            }
        };
        for session in &unlaid {
            // A sheet that cannot be laid anew now waits as it was (its batch writes nothing), and
            // the library opens all the same.
            match frames::relay(library, session, &held) {
                Ok(()) => log::info!("practice record {} laid anew (S99)", session.id),
                Err(error) => log::warn!("practice record {} not laid anew: {error}", session.id),
            }
        }
        Ok(())
    }
}
