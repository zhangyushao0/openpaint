//! The steps that touch the screen (a tap, a drag, a press, a stroke, fingers, a hover), their
//! spots resolved on the screen the last frame laid out, each batch one platform event and one
//! frame; and the steps that look, once motion settled.

use openpaint_runtime::{MotionBatch, Runtime};

use super::{FRAME_MS, Runner};
use crate::error::ScenarioError;
use crate::report::{Record, record, view_check};
use crate::scenario::Step;
use crate::target::{self, Key, Spot};
use crate::{gesture, pen};

impl Runner<'_> {
    /// A step that touches the screen, its spots resolved on the screen the last frame laid
    /// out, once the canvas's background work is done (`fill_held`): a stroke drawn into a view
    /// tile that is ready shows a step apart from one drawn whole after it, so whether the raster
    /// worker had finished a tile when the pen came down moved s98-alike's screens (S129).
    pub(super) fn contact(
        &mut self,
        runtime: &mut Runtime,
        step: &Step,
    ) -> Result<Vec<Record>, ScenarioError> {
        self.fill_held(runtime)?;
        let now = self.clock_ms;
        let batches = match step {
            Step::Tap { at, tool } => gesture::tap(*tool, self.spot(runtime, at)?, now),
            Step::Hover { at, pen } => {
                pen::held(gesture::hover(self.spot(runtime, at)?, now), *pen)
            }
            Step::Drag { from, to, tool } => {
                let (from, to) = (self.spot(runtime, from)?, self.spot(runtime, to)?);
                let points = [[from[0], from[1], 1.0], [to[0], to[1], 1.0]];
                gesture::path(*tool, &points, now)
            }
            Step::Press {
                path,
                tool,
                pressure,
                before_lift,
                canvas_before_lift,
                interrupted,
            } => {
                let keys = self.keys(runtime, path)?;
                let batches = gesture::press(*tool, &keys, *pressure, now);
                let before = before_lift.as_ref().map(|name| (name, *canvas_before_lift));
                return self.press(runtime, &batches, (before, *interrupted));
            }
            Step::Stroke {
                points,
                lift,
                palm,
                sparse,
                hold_ms,
                pen,
                tilt,
                azimuth,
            } => {
                let path = (points.as_slice(), *sparse, *hold_ms);
                let lean = [*tilt, *azimuth];
                pen::held(gesture::stroke(path, *lift, *palm, lean, now), *pen)
            }
            Step::Touch { from, to, ms } => {
                let from = self.spots(runtime, from)?;
                let to = match to {
                    Some(to) => self.spots(runtime, to)?,
                    None => from.clone(),
                };
                gesture::fingers(&from, &to, *ms, now)
            }
            _ => return Err(ScenarioError::Missing("not a step that touches")),
        };
        self.gesture(runtime, &batches)?;
        Ok(Vec::new())
    }

    /// `spot` in window pixels on the screen the last frame laid out. Window pixels on a
    /// control are noted: the step should name it.
    fn spot(&mut self, runtime: &Runtime, spot: &Spot) -> Result<[f32; 2], ScenarioError> {
        let map = runtime.controls();
        let at = target::resolve(map, spot)?;
        if matches!(spot, Spot::Point(_))
            && let Some(instead) = target::named_instead(map, at)
        {
            self.unnamed.push(format!("step {}: {instead}", self.index));
        }
        Ok(at)
    }

    fn spots(&mut self, runtime: &Runtime, spots: &[Spot]) -> Result<Vec<[f32; 2]>, ScenarioError> {
        spots.iter().map(|spot| self.spot(runtime, spot)).collect()
    }

    /// A press's keys as `[x, y, ms]`.
    fn keys(&mut self, runtime: &Runtime, keys: &[Key]) -> Result<Vec<[f32; 3]>, ScenarioError> {
        let mut out = Vec::with_capacity(keys.len());
        for key in keys {
            let (spot, ms) = match key {
                Key::Point([x, y, ms]) => (Spot::Point([*x, *y]), *ms),
                Key::At { at, ms } => (at.clone(), *ms),
            };
            let [x, y] = self.spot(runtime, &spot)?;
            out.push([x, y, ms]);
        }
        Ok(out)
    }

    /// A step that looks: once motion settled, or a screenshot `after_ms` the last input.
    pub(super) fn look(
        &mut self,
        runtime: &mut Runtime,
        step: &Step,
    ) -> Result<Option<Record>, ScenarioError> {
        match step {
            Step::Screenshot {
                after_ms: Some(ms), ..
            } => self.advance_to(runtime, self.input_ms + i64::from(*ms))?,
            _ => self.settle(runtime)?,
        }
        if matches!(step, Step::View { .. }) {
            return Ok(view_check(runtime, step));
        }
        record(runtime, step, &self.written).map(Some)
    }

    /// Each batch is one platform event followed by one frame at the batch's time.
    pub(super) fn gesture(
        &mut self,
        runtime: &mut Runtime,
        batches: &[MotionBatch],
    ) -> Result<(), ScenarioError> {
        for batch in batches {
            let at = gesture::end_ms(std::slice::from_ref(batch), self.clock_ms);
            self.clock_ms = self.clock_ms.max(at);
            runtime.motion(batch);
            self.frame(runtime)?;
        }
        self.input_ms = self.clock_ms;
        self.clock_ms = gesture::end_ms(batches, self.clock_ms).max(self.clock_ms) + FRAME_MS;
        Ok(())
    }
}
