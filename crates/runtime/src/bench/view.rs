//! `bench zoom|pan`: pinching and panning over the artwork's densest place as a hand does, each
//! until the view is sharp again, timed.

use super::hand::{busiest, scale, to_screen};
use super::{Watch, reached};
use crate::error::RuntimeError;
use crate::frame::Runtime;

/// A pinch's length in the input clock (longer than a quick pinch's 0.4 s, which fits the
/// canvas).
const PINCH_MS: i64 = 480;
/// The scales the zoom goes through, screen pixels per canvas pixel: in, then back.
const ZOOMS: [f32; 5] = [2.0, 4.0, 8.0, 4.0, 2.0];
/// Screen pixels the fingers of a centring pan or a pinch go up before they go where it takes
/// them: far past the 8 points a finger may drift and still tap.
const DETOUR: f32 = 200.0;

impl Runtime {
    /// `zoom`: pinches about the densest place from fit to 2x, 4x and 8x and back to 4x and 2x,
    /// each until sharp.
    pub(crate) fn bench_zoom(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let place = busiest(&artwork);
        lines.push(format!(
            "over the densest place ({:.0}, {:.0})",
            place[0], place[1]
        ));
        self.bench_settle()?;
        self.bench_centre(place)?;
        for (step, zoom) in ZOOMS.into_iter().enumerate() {
            let way = if step < 3 { "to" } else { "back to" };
            let (pinch, sharp) = self.bench_zoom_to(zoom)?;
            lines.push(format!("{way} {zoom}x: pinch {pinch}; {sharp}"));
        }
        Ok(())
    }

    /// `pan`: at 4x over the densest place, two-finger pans for 3 s, then until sharp.
    pub(crate) fn bench_pan(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let place = busiest(&artwork);
        self.bench_settle()?;
        self.bench_centre(place)?;
        self.bench_zoom_to(4.0)?;
        let mut watch = Watch::paced(self);
        let moves = [[-600.0, 0.0], [600.0, 0.0], [0.0, -400.0], [0.0, 400.0]];
        let mut k = 0;
        while watch.ms() < 3000.0 {
            let by = moves[k % moves.len()];
            let at = self.bench_middle()?;
            let from = [[at[0] - 150.0, at[1]], [at[0] + 150.0, at[1]]];
            let to = from.map(|p| [p[0] + by[0], p[1] + by[1]]);
            self.bench_fingers(&mut watch, &[from, to], 500)?;
            k += 1;
        }
        lines.push(format!("{k} pans at 4x: {}", watch.line(self)));
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, Self::bench_settled)?;
        lines.push(format!("sharp after the pans: {}", watch.line(self)));
        Ok(())
    }

    /// One pinch about the screen's middle to `zoom` screen pixels per canvas pixel, then frames
    /// until sharp; the pinch's and the settling's lines.
    fn bench_zoom_to(&mut self, zoom: f32) -> Result<(String, String), RuntimeError> {
        let pinch = self.bench_pinch(zoom)?;
        let mut watch = Watch::paced(self);
        let sharp = self.bench_until(&mut watch, Self::bench_settled)?;
        let shown = scale(&self.bench_view()?);
        let state = reached(sharp, "sharp");
        Ok((
            pinch,
            format!("at {shown:.2}x {state} {}", watch.line(self)),
        ))
    }

    /// One pinch about the screen's middle to `zoom` screen pixels per canvas pixel; its line.
    pub(super) fn bench_pinch(&mut self, zoom: f32) -> Result<String, RuntimeError> {
        let ratio = zoom / scale(&self.bench_view()?);
        let at = self.bench_middle()?;
        let half = if ratio >= 1.0 { 150.0 } else { 150.0 / ratio };
        let around = |half: f32| [[at[0] - half, at[1]], [at[0] + half, at[1]]];
        let mut watch = Watch::paced(self);
        let path = untapped(around(half), around(half * ratio));
        self.bench_fingers(&mut watch, &path, PINCH_MS)?;
        Ok(watch.line(self))
    }

    /// A two-finger pan bringing canvas pixel `place` to the screen's middle.
    pub(super) fn bench_centre(&mut self, place: [f32; 2]) -> Result<(), RuntimeError> {
        let frame = self.bench_view()?;
        let [x, y] = to_screen(&frame, place);
        let at = self.bench_middle()?;
        let by = [at[0] - x, at[1] - y];
        let from = [[at[0] - 150.0, at[1]], [at[0] + 150.0, at[1]]];
        let to = from.map(|p| [p[0] + by[0], p[1] + by[1]]);
        let mut watch = Watch::paced(self);
        self.bench_fingers(&mut watch, &untapped(from, to), 300)?;
        self.bench_until(&mut watch, Self::bench_settled)?;
        Ok(())
    }
}

/// Two fingers from `from` to `to` by way of `DETOUR` up: a pan or pinch whose fingers end near
/// where they began (a place already in the middle, a zoom already reached) would be a
/// two-finger tap, which undoes.
fn untapped(from: [[f32; 2]; 2], to: [[f32; 2]; 2]) -> [[[f32; 2]; 2]; 3] {
    [from, from.map(|p| [p[0], p[1] - DETOUR]), to]
}
