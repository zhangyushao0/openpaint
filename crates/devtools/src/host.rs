//! What a scenario sees of what the app asks its host after each frame (S124): the chimes the
//! host would have played (`Runtime::take_chime`, taken after every frame as Android takes it)
//! and whether it keeps the screen on now (`Runtime::screen_on`). The runner has no idle loop to
//! honour `Runtime::wake_in` with: its steps drive the frames, and `later` moves the clock.
//!
//! ```json
//! { "host": { "chimes": 1, "screen_on": false } }
//! ```

use openpaint_runtime::Runtime;
use serde::Deserialize;

use crate::error::ScenarioError;

/// A `host` step: the chimes played since the scenario began, restarts included, and whether
/// the screen is kept on after the last frame. Both are given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostExpect {
    pub chimes: u32,
    pub screen_on: bool,
}

/// What the host was asked so far.
#[derive(Debug, Default)]
pub(crate) struct Hosted {
    chimes: u32,
    screen_on: bool,
}

impl Hosted {
    /// After a frame: a chime taken, the screen's flag as the app wants it.
    pub(crate) fn frame(&mut self, runtime: &mut Runtime) {
        if runtime.take_chime() {
            self.chimes += 1;
        }
        self.screen_on = runtime.screen_on();
    }

    /// `expected` against what the host was asked.
    pub(crate) fn expect(&self, expected: HostExpect) -> Result<(), ScenarioError> {
        let seen = HostExpect {
            chimes: self.chimes,
            screen_on: self.screen_on,
        };
        if seen == expected {
            return Ok(());
        }
        Err(ScenarioError::Unexpected {
            what: format!("the host was asked {seen:?}, not {expected:?}"),
            available: Vec::new(),
        })
    }
}
