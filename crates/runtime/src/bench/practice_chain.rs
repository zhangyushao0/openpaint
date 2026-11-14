//! `bench practice` across lessons (S124: a lesson is one skill's, the full lesson gone): where
//! no exercise of the kinds a phase wants lies ahead on the sheet but a later skill of the lesson
//! in hand's unit teaches one, the lesson in hand is taken to its end by Next and that skill's
//! lesson laid on the same sheet in place, as a Pomodoro chains lessons (`practice::one_more`).
//! The walk's phases thus meet unit 1's line, combined lines and colours, or unit 3's estimates,
//! points in space and solids, one lesson after another; a phase no later skill teaches skips
//! nothing.

use openpaint_practice::{Skill, Unit};
use openpaint_practice_app::{PracticeIntent, Start};

use super::Watch;
use super::practice::key_of;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::practice::one_more;
use crate::screen::Screen;

impl Runtime {
    /// The first of `keys` an exercise at or after the one shown on the open sheet is keyed by.
    pub(super) fn bench_ahead(&self, keys: &[&str]) -> Option<&'static str> {
        let practice = self.practice()?;
        let from = practice.current.unwrap_or(practice.sheet.kinds.len());
        (practice.sheet.kinds.iter().skip(from))
            .map(|held| key_of(*held))
            .find(|key| keys.contains(key))
    }

    /// Lessons laid until an exercise of one of `keys` lies ahead (`bench_chain`); that key, or
    /// none when no later skill of the unit teaches one or the sheet takes no more.
    pub(super) fn bench_reach(
        &mut self,
        keys: &[&str],
    ) -> Result<Option<&'static str>, RuntimeError> {
        loop {
            if let Some(key) = self.bench_ahead(keys) {
                return Ok(Some(key));
            }
            if !self.bench_chain(keys)? {
                return Ok(None);
            }
        }
    }

    /// The lesson in hand ended by Next and the first later skill of its unit teaching one of
    /// `keys` laid, once its lesson shows; false when none is left or its lesson does not lay.
    fn bench_chain(&mut self, keys: &[&str]) -> Result<bool, RuntimeError> {
        let skill = self.practice().and_then(|p| p.lesson?.row.skill);
        let Some((skill, unit)) = skill.and_then(|skill| Some((skill, Unit::teaching(skill)?)))
        else {
            return Ok(false);
        };
        let later = (unit.skills.iter().copied())
            .skip_while(|held| *held != skill)
            .skip(1)
            .find(|later| keys.iter().any(|key| later.teaches(key)));
        let Some(later) = later else {
            return Ok(false);
        };
        let mut watch = Watch::paced(self);
        while self.practice().is_some_and(|p| p.current.is_some()) {
            let next = PracticeIntent::Next(self.bench_viewport()?);
            self.bench_practise(vec![next], &mut watch)?;
        }
        if !self.bench_lay(later)? {
            return Ok(false);
        }
        self.bench_settle()?;
        self.bench_rest()?;
        Ok(true)
    }

    /// `skill`'s lesson laid in place on the open sheet, frames until it shows; false when it
    /// does not start or fit.
    fn bench_lay(&mut self, skill: Skill) -> Result<bool, RuntimeError> {
        let viewport = self.bench_viewport()?;
        let now = self.frame_clock.wall_seconds();
        let Screen::Canvas(canvas) = &mut self.screen else {
            return Err(RuntimeError::NotNow("a practice sheet open"));
        };
        let app = (&mut self.practice, &mut self.seeds, &mut self.storage);
        if !one_more(canvas, app, (Start::Skill(skill), now, &viewport)) {
            return Ok(false);
        }
        let shown = |runtime: &Self| {
            (runtime.practice()).is_some_and(|p| {
                let laid = p
                    .lesson
                    .is_some_and(|lesson| lesson.row.skill == Some(skill));
                laid && p.current.is_some()
            })
        };
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, |runtime| shown(runtime) && !runtime.storing())?;
        Ok(shown(self))
    }
}
