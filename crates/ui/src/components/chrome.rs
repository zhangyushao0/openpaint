//! The chrome's paper and its motion, for every surface that comes and goes (D44). Islands and
//! the panels growing out of their buttons are drawn by the renderer's chrome pass as one
//! silhouette with one ink outline (`openpaint_render::chrome_shape`); every other surface (a
//! dialog, a pill, the loupe, the reference window) is its own piece of paper. A surface only
//! declares each frame whether it is open, where it comes from and where it lies finished; this
//! module moves it (emerge and retract, a panel switch as fade through) and `chrome_frame`
//! draws it, its content clipped to its shape and faded in near the end of the growth. The
//! content is always laid out at its finished place, so taps reach the final state at once; a
//! closing surface takes no contact.
//!
//! Each pass: `begin`, then the islands and surfaces, then `finish`.

use egui::layers::ShapeIdx;
use egui::{Id, LayerId, Order, Rect};

use crate::components::animate::{keep_moving, now};
use crate::components::chrome_frame::Frame;
use crate::components::surface::{Growth, RING};
use crate::design::CORNER;
use crate::design::motion::{LONGEST_FRAME, SWITCH_DELAY};

/// Where the chrome keeps its motion in egui's memory, and its layer.
const STATE: &str = "chrome motion";
pub(crate) const LAYER: &str = "chrome";

/// Where a surface comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    /// A panel, out of `button` on the island `island`.
    Button { island: Id, button: Rect },
    /// Paper from `rect`, its corners from `radius.0` to `radius.1`; a dialog veils the screen.
    Rect {
        rect: Rect,
        radius: (f32, f32),
        veil: bool,
    },
    /// Paper from a circle of its own height at its left end, middle or right end.
    Within(egui::Align),
}

/// How a declared surface shows in this pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Showing {
    /// Share of the content shown.
    pub(crate) reveal: f32,
    /// How far the surface has grown, 0 to 1.
    pub(crate) grown: f32,
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) id: Id,
    pub(crate) source: Source,
    pub(crate) order: Order,
    pub(crate) target: Rect,
    pub(crate) growth: Growth,
    /// Declared open in this pass.
    wanted: bool,
    /// Laid out in this pass: its layer and, for paper, the shape reserved for it.
    pub(crate) laid: Option<(LayerId, Option<ShapeIdx>)>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct State {
    /// Input time of the last pass, in seconds.
    time: Option<f64>,
    clock: u64,
    pub(crate) entries: Vec<Entry>,
    /// This pass's islands.
    pub(crate) islands: Vec<(Id, Rect)>,
}

fn state<R>(ctx: &egui::Context, f: impl FnOnce(&mut State) -> R) -> R {
    ctx.data_mut(|data| f(data.get_temp_mut_or_default::<State>(Id::new(STATE))))
}

/// Starts a pass: the chrome's layer goes under every other surface in front.
pub(crate) fn begin(ctx: &egui::Context) {
    egui::Area::new(Id::new(LAYER))
        .order(Order::Foreground)
        .interactable(false)
        .fade_in(false)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |_| {});
    state(ctx, |state| {
        state.islands.clear();
        for entry in &mut state.entries {
            entry.wanted = false;
            entry.laid = None;
        }
    });
}

/// A new screen: what the old one showed goes with it, with no motion.
pub(crate) fn forget(ctx: &egui::Context) {
    state(ctx, |state| state.entries.clear());
}

/// The island `id` lies at `rect` in this pass; its paper is the chrome pass's.
pub(crate) fn island(ctx: &egui::Context, id: Id, rect: Rect) {
    state(ctx, |state| state.islands.push((id, rect)));
}

/// The surface `id`, coming from `source`, is `open` in this pass. Returns how it shows, or
/// `None` when it is closed and gone: then nothing of it is drawn.
pub(crate) fn declare(
    ctx: &egui::Context,
    id: Id,
    source: Source,
    order: Order,
    open: bool,
) -> Option<Showing> {
    state(ctx, |state| {
        let known = state.entries.iter().position(|entry| entry.id == id);
        let index = match (known, open) {
            (Some(index), _) => index,
            (None, false) => return None,
            (None, true) => {
                state.entries.push(Entry {
                    id,
                    source,
                    order,
                    target: Rect::NOTHING,
                    growth: closed(),
                    wanted: false,
                    laid: None,
                });
                state.entries.len() - 1
            }
        };
        let entry = &mut state.entries[index];
        entry.source = source;
        entry.wanted = open;
        Some(Showing {
            reveal: entry.growth.reveal(),
            grown: entry.growth.grown(),
        })
    })
}

/// The declared surface `id` was laid out finished at `target` on `layer`; paper keeps the
/// shape `paper` reserved under its content.
pub(crate) fn laid_out(
    ctx: &egui::Context,
    id: Id,
    target: Rect,
    layer: LayerId,
    paper: Option<ShapeIdx>,
) {
    state(ctx, |state| {
        if let Some(entry) = state.entries.iter_mut().find(|entry| entry.id == id) {
            entry.target = target;
            entry.laid = Some((layer, paper));
        }
    });
}

/// A growth closed and at rest at its source.
fn closed() -> Growth {
    let mut growth = Growth::new(0, 0.0);
    growth.close(0, false);
    growth
}

/// Ends a pass: moves every surface by the time since the last pass, then draws the chrome
/// pass and the paper.
pub(crate) fn finish(ctx: &egui::Context) {
    let time = now(ctx);
    let (moving, frame) = state(ctx, |state| {
        let dt = state
            .time
            .map_or(0.0, |last| ((time - last) as f32).clamp(0.0, LONGEST_FRAME));
        state.time = Some(time);
        // A panel whose island left the screen goes with it.
        let islands = &state.islands;
        state.entries.retain(|entry| match entry.source {
            Source::Button { island, .. } => islands.iter().any(|(id, _)| *id == island),
            Source::Rect { .. } | Source::Within(_) => true,
        });
        // What rested until now starts moving from this frame on: a long idle time before
        // never jumps it.
        let resting: Vec<Id> = state
            .entries
            .iter()
            .filter(|entry| entry.growth.at_rest())
            .map(|entry| entry.id)
            .collect();
        transitions(state);
        for entry in &mut state.entries {
            let rested = resting.contains(&entry.id);
            entry.growth.step(if rested { 0.0 } else { dt });
        }
        let frame = Frame::of(state);
        state.entries.retain(|entry| !entry.growth.gone());
        let moving = state.entries.iter().any(|entry| !entry.growth.at_rest());
        (moving, frame)
    });
    frame.paint(ctx);
    if moving {
        keep_moving(ctx);
    }
}

/// Opens and closes what the pass declared differently from before. A panel opening while
/// another closes is a switch: the old one retracts fast and the tapped one grows a moment
/// later, so the motion that ends last is always the tapped panel's. One not yet growing just
/// gives way; one still retracting comes back out from where it is.
fn transitions(state: &mut State) {
    let panel = |entry: &Entry| matches!(entry.source, Source::Button { .. });
    let opening = |entry: &Entry| entry.wanted && !entry.growth.open;
    let closing = |entry: &Entry| !entry.wanted && entry.growth.open;
    let switching = state.entries.iter().any(|e| panel(e) && opening(e))
        && state.entries.iter().any(|e| panel(e) && closing(e));
    if !state.entries.iter().any(|e| opening(e) || closing(e)) {
        return;
    }
    state.clock += 1;
    let since = state.clock;
    state
        .entries
        .retain(|entry| !(closing(entry) && entry.growth.pending()));
    for entry in &mut state.entries {
        let switch = switching && panel(entry);
        if closing(entry) {
            entry.growth.close(since, switch);
        } else if opening(entry) && entry.growth.gone() {
            let delay = if switch { SWITCH_DELAY } else { 0.0 };
            entry.growth = Growth::new(since, delay);
        } else if opening(entry) {
            entry.growth.reopen(since);
        }
    }
}

/// The source of a surface out of the ink ring of the button at `button`.
#[must_use]
pub fn ring_of(button: Rect) -> Source {
    Source::Rect {
        rect: Rect::from_center_size(button.center(), egui::Vec2::splat(2.0 * RING)),
        radius: (RING, CORNER),
        veil: false,
    }
}

#[cfg(test)]
#[path = "chrome_tests.rs"]
mod tests;
