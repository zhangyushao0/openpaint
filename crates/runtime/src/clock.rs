//! The input clock: platform time as input samples carry it, carried on by the wall clock
//! between batches, so gestures that wait without input (fingers held for a repeating
//! shortcut, the view status fading) see time pass in the same clock their samples use. And the
//! frame clock motion runs on (D44): real elapsed time from the start, or a time the harness
//! holds fixed so scenarios are deterministic.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use openpaint_app::InputBatch;

#[derive(Debug)]
pub(crate) struct InputClock {
    /// The newest sample time seen and when it arrived.
    last: Option<(i64, Instant)>,
    started: Instant,
}

impl Default for InputClock {
    fn default() -> Self {
        Self {
            last: None,
            started: Instant::now(),
        }
    }
}

impl InputClock {
    /// Takes the newest sample time of `batch`.
    pub(crate) fn saw(&mut self, batch: &InputBatch) {
        let newest = batch
            .contacts
            .iter()
            .flat_map(|contact| &contact.samples)
            .map(|sample| sample.time_ms)
            .max();
        if let Some(newest) = newest {
            let later = self.last.is_none_or(|(last, _)| newest >= last);
            if later {
                self.last = Some((newest, Instant::now()));
            }
        }
    }

    /// Milliseconds now: the newest sample's time plus the wall time since it arrived.
    pub(crate) fn now_ms(&self) -> i64 {
        let (base, since) = self.last.unwrap_or((0, self.started));
        let elapsed = i64::try_from(since.elapsed().as_millis()).unwrap_or(i64::MAX);
        base.saturating_add(elapsed)
    }
}

/// The clock motion runs on.
#[derive(Debug)]
pub(crate) struct FrameClock {
    started: Instant,
    /// Milliseconds the harness holds the clock at, instead of the wall clock.
    fixed: Option<i64>,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            fixed: None,
        }
    }
}

impl FrameClock {
    /// From now on the clock reads `ms` until the next call.
    pub(crate) const fn fix(&mut self, ms: i64) {
        self.fixed = Some(ms);
    }

    /// Seconds now.
    pub(crate) fn seconds(&self) -> f64 {
        match self.fixed {
            Some(ms) => ms as f64 / 1000.0,
            None => self.started.elapsed().as_secs_f64(),
        }
    }

    /// The wall clock in seconds since 1970 UTC, for dating kept versions; while the harness
    /// holds the clock, `FIXED_WALL` plus the held time, so scenarios date them alike.
    pub(crate) fn wall_seconds(&self) -> u64 {
        match self.fixed {
            Some(ms) => FIXED_WALL + u64::try_from(ms / 1000).unwrap_or(0),
            None => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
        }
    }
}

/// 2026-10-05 12:00 UTC: the wall clock's start while the harness holds the clock.
const FIXED_WALL: u64 = 1_791_201_600;
