//! For debug builds only (`cargo device-input practice-test start|end`, DECISIONS D102): a test
//! lesson (the next one, a skill's at a level, a unit's full lesson, the first lesson; or a test
//! colour copy, `start copy`) started beside any open one, and removed afterwards with its sheet,
//! so a practice slice walks its lesson on the tablet while the author's lessons, copies, records
//! and the path's dots stay as they were. A test lesson's sheet is a `Purpose::Test` sheet (a
//! day's in all else, never continued); a test copy stays a copy.
//!
//! The test lesson is started from the home even while another is open, as the log starts it
//! (`PracticeLog::start`). It is named by its id, its sheet and when it started
//! (`PracticeTest`), which the chore keeps on the dev machine between the two calls: `end`
//! removes the lesson holding all three and nothing else, also after the app restarted, and a
//! name another sheet could hold (a library restored since) matches none.

use std::fmt;
use std::str::FromStr;

use openpaint_app::ProjectId;
use openpaint_practice::{Purpose, Sheet, Skill, SkillLevel};
use openpaint_practice_app::Start;

use crate::display::CacheJob;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::practice::{PracticeJob, copy_job, lesson_job, seed};
use crate::storage::Job;

/// What a skill is named (`practice-test start lesson <skill>`, a scenario's `lesson` step: its
/// key, `Skill::key`), as the refusal of another name says it.
const SKILLS: &str = "a skill of line, combined, colour, shape, combination, scale, box, \
                      free-boxes, ellipse, estimates, points-in-space, solids, objects, division, \
                      value-masses, whole-copy or key-shift, at level 1, 2 or 3";

/// The lesson of the skill keyed `key` at `level` (1 to 3), or at the learner's level without
/// one.
///
/// # Errors
/// `NotNow` for a key no skill has, or a level past 3.
pub fn skill_start(key: &str, level: Option<u8>) -> Result<Start, RuntimeError> {
    let skill = Skill::of_key(key).ok_or(RuntimeError::NotNow(SKILLS))?;
    match level {
        None => Ok(Start::Skill(skill)),
        Some(level) => {
            let at = usize::from(level.checked_sub(1).ok_or(RuntimeError::NotNow(SKILLS))?);
            let level = SkillLevel::ALL
                .get(at)
                .ok_or(RuntimeError::NotNow(SKILLS))?;
            Ok(Start::At(skill, *level))
        }
    }
}

/// A test lesson's name: its id, its sheet's project and when it started (seconds since
/// 1970 UTC). Written `id:sheet:started`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeTest {
    pub id: u64,
    pub sheet: u64,
    pub started: u64,
}

impl PracticeTest {
    fn of(session: &Sheet) -> Self {
        Self {
            id: session.id,
            sheet: session.canvas,
            started: session.started,
        }
    }

    fn names(&self, session: &Sheet) -> bool {
        *self == Self::of(session) && session.canvas != 0
    }
}

impl fmt::Display for PracticeTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { id, sheet, started } = self;
        write!(f, "{id}:{sheet}:{started}")
    }
}

impl FromStr for PracticeTest {
    type Err = RuntimeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let bad = RuntimeError::NotNow("a test lesson written id:sheet:started");
        let parts: Vec<&str> = text.split(':').collect();
        let [id, sheet, started] = parts[..] else {
            return Err(bad);
        };
        let number = |part: &str| part.parse::<u64>().map_err(|_| bad.clone());
        Ok(Self {
            id: number(id)?,
            sheet: number(sheet)?,
            started: number(started)?,
        })
    }
}

impl Runtime {
    /// For debug builds only: a test lesson as `start` names it (`skill_start` for a skill's),
    /// started from the home beside any open one on a test's sheet; its sheet opens once the
    /// store wrote it (`test_practice` names it then). A full lesson without a unit given is the
    /// learner's unit's. Returns its id.
    ///
    /// # Errors
    /// `NotNow` when the home does not show, or nothing is there to start (a skill whose
    /// pictures are not kept, a unit not built); a sheet that cannot be made.
    pub fn start_test_lesson(&mut self, start: Start) -> Result<u64, RuntimeError> {
        if self.screen.gallery_mut().is_none() {
            return Err(RuntimeError::NotNow("the home"));
        }
        let now = self.frame_clock.wall_seconds();
        let id = self.practice.next_id();
        let start = (start, Purpose::Test);
        let job = lesson_job(&self.practice, start, &mut self.seeds, now)?;
        self.storage.submit(job);
        Ok(id)
    }

    /// For debug builds only: the full lesson of `unit`, else of the learner's unit (the one the
    /// path offers), as `start_test_lesson`.
    ///
    /// # Errors
    /// As `start_test_lesson`.
    pub fn start_test_full(&mut self, unit: Option<u8>) -> Result<u64, RuntimeError> {
        let unit = unit.unwrap_or_else(|| self.practice.path().unit);
        self.start_test_lesson(Start::Full(unit))
    }

    /// For debug builds only: a test colour copy of an official pack's picture not copied before
    /// (picked by a seed: a test chooses no picture) begun from the home beside the author's;
    /// its sheet opens once the store wrote it (`test_practice` names it then,
    /// `end_test_practice` removes it as a lesson). Returns its id.
    ///
    /// # Errors
    /// `NotNow` when the home does not show or no official pack holds a picture.
    pub fn start_test_copy(&mut self) -> Result<u64, RuntimeError> {
        if self.screen.gallery_mut().is_none() {
            return Err(RuntimeError::NotNow("the home"));
        }
        let now = self.frame_clock.wall_seconds();
        let id = self.practice.next_id();
        let picture = (self.practice.fresh_picture(seed(&mut self.seeds)))
            .ok_or(RuntimeError::NotNow("a pack's picture to copy"))?;
        let job = copy_job(&self.practice, &picture, &mut self.seeds, now)?;
        self.storage.submit(job);
        Ok(id)
    }

    /// Test lesson (or colour copy) `id`'s name, once its sheet is stored and shows.
    #[must_use]
    pub fn test_practice(&self, id: u64) -> Option<PracticeTest> {
        let session = self.practice.session(id).filter(|s| s.canvas != 0)?;
        let shows = self.practice().is_some() && !self.storing();
        shows.then(|| PracticeTest::of(session))
    }

    /// For debug builds only: the test lesson `test` removed from the home, its record and its
    /// sheet; the log forgets it once the store did (`holds_practice`).
    ///
    /// # Errors
    /// `NotNow` when the home does not show, or no sheet holds `test`'s name.
    pub fn end_test_practice(&mut self, test: &PracticeTest) -> Result<(), RuntimeError> {
        if self.screen.gallery_mut().is_none() {
            return Err(RuntimeError::NotNow("the home"));
        }
        let session = (self.practice.session(test.id))
            .filter(|session| test.names(session))
            .cloned()
            .ok_or(RuntimeError::NotNow(
                "a lesson of that name (`names_practice`)",
            ))?;
        self.cache
            .submit(CacheJob::Delete(ProjectId::new(session.canvas)));
        self.storage
            .submit(Job::Practice(PracticeJob::Remove(session)));
        Ok(())
    }

    /// Whether the practice log holds sheet `id`.
    #[must_use]
    pub fn holds_practice(&self, id: u64) -> bool {
        self.practice.session(id).is_some()
    }

    /// Whether the practice log holds a sheet of the name `test`.
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
            started: 1_791_290_000,
        };
        let written = test.to_string();
        assert_eq!(written, "13:41:1791290000");
        assert_eq!(written.parse::<PracticeTest>(), Ok(test));
        for bad in ["", "13:41", "13:41:zz", "13:41:1:2", "-1:41:1"] {
            assert!(bad.parse::<PracticeTest>().is_err(), "{bad}");
        }
    }

    #[test]
    fn every_skill_has_one_plain_name_the_refusal_lists() {
        for skill in Skill::all() {
            let name = skill.key();
            assert_eq!(skill_start(name, None), Ok(Start::Skill(skill)));
            assert_eq!(
                skill_start(name, Some(3)),
                Ok(Start::At(skill, SkillLevel::Third))
            );
            assert!(SKILLS.contains(name), "{name}");
        }
        assert!(skill_start("line", Some(0)).is_err());
        assert!(skill_start("line", Some(4)).is_err());
        assert!(skill_start("stage", None).is_err());
    }
}
