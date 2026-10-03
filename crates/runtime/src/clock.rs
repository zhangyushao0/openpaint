//! The input clock: platform time as input samples carry it, carried on by the wall clock
//! between batches, so gestures that wait without input (fingers held for a repeating
//! shortcut, the view status fading) see time pass in the same clock their samples use.

use std::time::Instant;

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
