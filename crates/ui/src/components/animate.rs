//! Motion kept by the shared components between frames: one spring per id in egui's memory,
//! stepped by the time egui's input carries (the frame loop's clock, a fixed one in scenarios),
//! never per frame. While anything moves, frames keep coming (`moving`); idle frames stay idle.

use egui::Id;

use crate::design::motion::{Feel, Spring};

/// Where egui's memory keeps the pass in which something last moved.
const MOVING: &str = "motion moving";

#[derive(Debug, Clone, Copy)]
struct Tracked {
    spring: Spring,
    target: f32,
    /// The input time of its last step, in seconds.
    time: f64,
}

/// The value of the spring `id` moving towards `target` with `feel`, now. A new spring starts
/// at rest on its target; one at rest starts moving from this frame on, so a long idle time
/// before never jumps it.
pub fn animate(ctx: &egui::Context, id: Id, target: f32, feel: Feel) -> f32 {
    animate_from(ctx, id, target, target, feel)
}

/// As `animate`, a new spring starting at `from`.
pub fn animate_from(ctx: &egui::Context, id: Id, from: f32, target: f32, feel: Feel) -> f32 {
    let now = ctx.input(|input| input.time);
    let (value, resting) = ctx.data_mut(|data| {
        let tracked = data.get_temp_mut_or_insert_with(id, || Tracked {
            spring: Spring::at(from),
            target: from,
            time: now,
        });
        let dt = if tracked.spring.at_rest(tracked.target) {
            0.0
        } else {
            (now - tracked.time) as f32
        };
        tracked.time = now;
        tracked.target = target;
        tracked.spring.step(target, feel, 1.0, dt);
        (tracked.spring.x, tracked.spring.at_rest(target))
    });
    if !resting {
        keep_moving(ctx);
    }
    value
}

#[derive(Debug, Clone, Copy)]
struct Placed {
    place: f32,
    /// Distance from where it shows to its place.
    offset: Spring,
    time: f64,
}

/// Settle: how far from `place` the thing `id` shows now, after it moved there from where it
/// was (a row after a reorder, an add or a delete). Its place takes contacts at once; only the
/// drawing makes room. Something seen for the first time shows at its place.
pub fn settle(ctx: &egui::Context, id: Id, place: f32, feel: Feel) -> f32 {
    let now = ctx.input(|input| input.time);
    let (offset, resting) = ctx.data_mut(|data| {
        let placed = data.get_temp_mut_or_insert_with(id, || Placed {
            place,
            offset: Spring::at(0.0),
            time: now,
        });
        let dt = if placed.offset.at_rest(0.0) {
            0.0
        } else {
            (now - placed.time) as f32
        };
        placed.offset.x += placed.place - place;
        placed.place = place;
        placed.time = now;
        placed.offset.step(0.0, feel, 1.0, dt);
        (placed.offset.x, placed.offset.at_rest(0.0))
    });
    if !resting {
        keep_moving(ctx);
    }
    offset
}

/// Settle's memory without motion: `id` shows exactly at `place` now (a row the hand carries),
/// so a later `settle` slides from there.
pub fn hold_at(ctx: &egui::Context, id: Id, place: f32) {
    let now = ctx.input(|input| input.time);
    ctx.data_mut(|data| {
        data.insert_temp(
            id,
            Placed {
                place,
                offset: Spring::at(0.0),
                time: now,
            },
        );
    });
}

/// Something moves this frame: the next frame is wanted at once.
pub(crate) fn keep_moving(ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| data.insert_temp(Id::new(MOVING), pass));
    ctx.request_repaint();
}

/// True when something moved in this pass; ask before the pass ends.
#[must_use]
pub fn moving(ctx: &egui::Context) -> bool {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|data| data.get_temp::<u64>(Id::new(MOVING))) == Some(pass)
}

/// The input time now, in seconds.
pub(crate) fn now(ctx: &egui::Context) -> f64 {
    ctx.input(|input| input.time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::motion::ACKNOWLEDGE;

    fn pass(ctx: &egui::Context, time: f64, run: impl FnMut(&egui::Context)) {
        let input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        let mut run = run;
        let mut output = ctx.run_ui(input, |ui| run(ui.ctx()));
        output.textures_delta.clear();
    }

    #[test]
    fn a_spring_follows_elapsed_time_and_frames_stop_once_it_rests() {
        let ctx = egui::Context::default();
        let id = Id::new("ring");
        let mut seen = Vec::new();
        for (time, target) in [
            (0.0, 0.0),
            (10.0, 1.0),
            (10.05, 1.0),
            (10.1, 1.0),
            (12.0, 1.0),
        ] {
            pass(&ctx, time, |ctx| {
                seen.push((animate(ctx, id, target, ACKNOWLEDGE), moving(ctx)));
            });
        }
        assert_eq!(seen[0], (0.0, false), "a new spring rests on its target");
        assert_eq!(seen[1], (0.0, true), "the idle time before does not count");
        assert!(seen[2].0 > 0.0 && seen[3].0 > seen[2].0 && seen[3].1);
        assert!(seen[4].0 > seen[3].0);
        let mut last = (0.0, true);
        for step in 1..40 {
            pass(&ctx, 12.0 + f64::from(step) * 0.05, |ctx| {
                last = (animate(ctx, id, 1.0, ACKNOWLEDGE), moving(ctx));
            });
        }
        assert_eq!(last, (1.0, false), "at rest: no more frames");
    }
}
