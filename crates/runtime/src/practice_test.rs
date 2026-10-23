//! For debug builds only (`cargo device-input practice-test start|end`, DECISIONS D102): a test
//! practice session (or a test colour copy, `start copy`) started beside any open one, and
//! removed afterwards with its sheet, so a practice slice walks its session on the tablet while
//! the author's sessions, copies, records and the Today card's progress stay as they were.
//!
//! The test session is the one Start practice would start now (the next of the log: its number,
//! its warm-up from the kinds met), started from the gallery even while another is open. It is
//! named by its id, its sheet, its seed and when it started (`PracticeTest`), which the chore
//! keeps on the dev machine between the two calls: `end` removes the session holding all four
//! and nothing else, also after the app restarted, and a name another session could hold (a
//! library restored since) matches none.

use std::fmt;
use std::str::FromStr;

use openpaint_app::{CopyFrom, ProjectId};
use openpaint_practice::SessionRecord;

use crate::display::CacheJob;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::practice::{PracticeJob, copy_job, start_job};
use crate::storage::Job;

/// A test session's name: its id, its sheet's project, its seed and when it started (seconds
/// since 1970 UTC). Written `id:sheet:seed:started`, the seed in hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeTest {
    pub id: u64,
    pub sheet: u64,
    pub seed: u64,
    pub started: u64,
}

impl PracticeTest {
    fn of(session: &SessionRecord) -> Self {
        Self {
            id: session.id,
            sheet: session.canvas,
            seed: session.seed,
            started: session.started,
        }
    }

    fn names(&self, session: &SessionRecord) -> bool {
        *self == Self::of(session) && session.canvas != 0
    }
}

impl fmt::Display for PracticeTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            id,
            sheet,
            seed,
            started,
        } = self;
        write!(f, "{id}:{sheet}:{seed:016x}:{started}")
    }
}

impl FromStr for PracticeTest {
    type Err = RuntimeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = RuntimeError::NotNow("a test session written id:sheet:seed:started");
        let parts: Vec<&str> = text.split(':').collect();
        let [id, sheet, seed, started] = parts[..] else {
            return Err(bad);
        };
        let number = |part: &str, radix| u64::from_str_radix(part, radix).map_err(|_| bad.clone());
        Ok(Self {
            id: number(id, 10)?,
            sheet: number(sheet, 10)?,
            seed: number(seed, 16)?,
            started: number(started, 10)?,
        })
    }
}

impl Runtime {
    /// For debug builds only: the next practice session of `stage` (none: the stage practised)
    /// started from the gallery, beside any open one; its sheet opens once the store wrote it
    /// (`test_practice` names it then). Returns its id.
    ///
    /// # Errors
    /// `NotNow` when the gallery does not show; a sheet that cannot be made.
    pub fn start_test_practice(&mut self, stage: Option<u8>) -> Result<u64, RuntimeError> {
        if self.screen.gallery_mut().is_none() {
            return Err(RuntimeError::NotNow("the gallery"));
        }
        let now = self.frame_clock.wall_seconds();
        let id = self.practice.next_id();
        let stage = stage.unwrap_or_else(|| self.practice.stage());
        let job = start_job(&self.practice, stage, &mut self.seeds, now)?;
        self.storage.submit(job);
        Ok(id)
    }

    /// For debug builds only: a test colour copy of a pack's picture begun from the gallery
    /// beside the author's; its sheet opens once the store wrote it (`test_practice` names it
    /// then, `end_test_practice` removes it as a session). Returns its id.
    ///
    /// # Errors
    /// `NotNow` when the gallery does not show or no pack holds a picture.
    pub fn start_test_copy(&mut self) -> Result<u64, RuntimeError> {
        if self.screen.gallery_mut().is_none() {
            return Err(RuntimeError::NotNow("the gallery"));
        }
        let now = self.frame_clock.wall_seconds();
        let id = self.practice.next_id();
        let job = copy_job(&self.practice, CopyFrom::Pack, &mut self.seeds, now)?;
        self.storage.submit(job);
        Ok(id)
    }

    /// Test session (or colour copy) `id`'s name, once its sheet is stored and shows.
    #[must_use]
    pub fn test_practice(&self, id: u64) -> Option<PracticeTest> {
        let session = self.practice.session(id).filter(|s| s.canvas != 0)?;
        let shows = self.projection().practice().is_some() && !self.storing();
        shows.then(|| PracticeTest::of(session))
    }

    /// For debug builds only: the test session `test` removed from the gallery, its record and
    /// its sheet; the log forgets it once the store did (`holds_practice`).
    ///
    /// # Errors
    /// `NotNow` when the gallery does not show, or no session holds `test`'s name.
    pub fn end_test_practice(&mut self, test: &PracticeTest) -> Result<(), RuntimeError> {
        if self.screen.gallery_mut().is_none() {
            return Err(RuntimeError::NotNow("the gallery"));
        }
        let session = (self.practice.session(test.id))
            .filter(|session| test.names(session))
            .cloned()
            .ok_or(RuntimeError::NotNow(
                "a session of that name (`names_practice`)",
            ))?;
        self.cache
            .submit(CacheJob::Delete(ProjectId::new(session.canvas)));
        self.storage
            .submit(Job::Practice(PracticeJob::Remove(session)));
        Ok(())
    }

    /// Whether the practice log holds session `id`.
    #[must_use]
    pub fn holds_practice(&self, id: u64) -> bool {
        self.practice.session(id).is_some()
    }

    /// Whether the practice log holds a session of the name `test`.
    #[must_use]
    pub fn names_practice(&self, test: &PracticeTest) -> bool {
        self.practice
            .session(test.id)
            .is_some_and(|s| test.names(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_written_and_read_back_whole() {
        let test = PracticeTest {
            id: 13,
            sheet: 41,
            seed: 0x5EED_0045_0000_0079,
            started: 1_791_290_000,
        };
        let written = test.to_string();
        assert_eq!(written, "13:41:5eed004500000079:1791290000");
        assert_eq!(written.parse::<PracticeTest>(), Ok(test));
        for bad in ["", "13:41:5eed", "13:41:zz:1", "13:41:1:2:3", "-1:41:1:2"] {
            assert!(bad.parse::<PracticeTest>().is_err(), "{bad}");
        }
    }
}
