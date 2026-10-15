//! `bench open-cold|open|zoom|pan` (S58, `bench_app.rs`): opening, zooming and panning driven
//! as a hand does, timed.

use std::time::Duration;

use openpaint_app::{GalleryIntent, Intent, ProjectId};
use openpaint_core::Artwork;
use openpaint_render::ViewFrame;

use super::Watch;
use crate::display::CacheJob;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};
use crate::screen::Screen;

/// How long a stage may take before the benchmark gives up on it.
const LIMIT: Duration = Duration::from_secs(60);
/// A pinch's length in the input clock (longer than a quick pinch's 0.4 s, which fits the
/// canvas), and its batches' step.
const PINCH_MS: i64 = 480;
const STEP_MS: i64 = 8;
/// The scales the zoom goes through, screen pixels per canvas pixel: in, then back.
const ZOOMS: [f32; 5] = [2.0, 4.0, 8.0, 4.0, 2.0];

impl Runtime {
    /// The open artwork's project and a copy of it; `NotNow` without one from the library.
    pub(crate) fn bench_artwork(&self) -> Result<(ProjectId, Artwork), RuntimeError> {
        let none = RuntimeError::NotNow("an artwork of the library open");
        let canvas = self.screen.canvas().ok_or(none.clone())?;
        let project = canvas.project().ok_or(none)?;
        Ok((project, canvas.session().artwork().clone()))
    }

    /// `open-cold` (the display cache deleted first) and `open`.
    pub(crate) fn bench_open(
        &mut self,
        cold: bool,
        lines: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        let (project, artwork) = self.bench_artwork()?;
        let mut watch = Watch::paced(self);
        // A cache being written lands first, so `open` finds it.
        self.bench_until(&mut watch, LIMIT, |runtime| {
            runtime.bench_settled() && !runtime.cache.busy()
        })?;
        if cold {
            self.cache.submit(CacheJob::Delete(project));
        }
        self.bench_leave(&mut watch)?;
        lines.push(format!(
            "left for the gallery: {}; the display cache {}",
            watch.line(self),
            if cold { "deleted" } else { "kept" }
        ));
        let gallery = (self.screen.gallery_mut()).ok_or(RuntimeError::NotNow("the gallery"))?;
        gallery.dispatch(GalleryIntent::Open(project))?;
        let mut watch = Watch::paced(self);
        let shown = |runtime: &Self| runtime.screen.canvas().is_some();
        self.bench_until(&mut watch, LIMIT, shown)?;
        let first = watch.ms();
        let filled = |runtime: &Self| runtime.screen.canvas().is_some_and(|c| !c.filling());
        let complete = self.bench_until(&mut watch, LIMIT, filled)?;
        let full = watch.ms();
        let warm = |runtime: &Self| runtime.screen.canvas().is_some_and(|c| !c.warming());
        let warmed = self.bench_until(&mut watch, LIMIT, warm)?;
        let canvas = artwork.canvas();
        lines.push(format!(
            "project {} ({} x {}, {} strokes): the canvas shown after {first:.0} ms, {} {full:.0} \
             ms, its residuals {} {}",
            project.get(),
            canvas.width(),
            canvas.height(),
            strokes(&artwork),
            if complete {
                "complete after"
            } else {
                "NOT complete within the limit"
            },
            if warmed {
                "ready after"
            } else {
                "NOT ready within the limit:"
            },
            watch.line(self)
        ));
        Ok(())
    }

    /// The canvas left for the gallery as Gallery does, frames until the gallery shows.
    fn bench_leave(&mut self, watch: &mut Watch) -> Result<(), RuntimeError> {
        if let Screen::Canvas(canvas) = &mut self.screen {
            let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
            canvas.dispatch(&self.core, vec![Intent::Gallery(GalleryIntent::Show)], app);
        }
        let gallery = |runtime: &Self| matches!(runtime.screen, Screen::Gallery(_));
        if self.bench_until(watch, LIMIT, gallery)? {
            Ok(())
        } else {
            Err(RuntimeError::NotNow("the gallery after leaving"))
        }
    }

    /// `zoom`: pinches about the densest place from fit to 2x, 4x and 8x and back to 4x and 2x,
    /// each until sharp.
    pub(crate) fn bench_zoom(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let place = busiest(&artwork);
        lines.push(format!(
            "over the densest place ({:.0}, {:.0})",
            place[0], place[1]
        ));
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
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
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
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
            self.bench_fingers(&mut watch, from, to, 500)?;
            k += 1;
        }
        lines.push(format!("{k} pans at 4x: {}", watch.line(self)));
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        lines.push(format!("sharp after the pans: {}", watch.line(self)));
        Ok(())
    }

    /// One pinch about the screen's middle to `zoom` screen pixels per canvas pixel, then frames
    /// until sharp; the pinch's and the settling's lines.
    fn bench_zoom_to(&mut self, zoom: f32) -> Result<(String, String), RuntimeError> {
        let ratio = zoom / scale(&self.bench_view()?);
        let at = self.bench_middle()?;
        let half = if ratio >= 1.0 { 150.0 } else { 150.0 / ratio };
        let around = |half: f32| [[at[0] - half, at[1]], [at[0] + half, at[1]]];
        let mut watch = Watch::paced(self);
        self.bench_fingers(&mut watch, around(half), around(half * ratio), PINCH_MS)?;
        let pinch = watch.line(self);
        let mut watch = Watch::paced(self);
        let sharp = self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        let shown = scale(&self.bench_view()?);
        let state = if sharp {
            "sharp after"
        } else {
            "NOT sharp within the limit:"
        };
        Ok((
            pinch,
            format!("at {shown:.2}x {state} {}", watch.line(self)),
        ))
    }

    /// A two-finger pan bringing canvas pixel `place` to the screen's middle.
    fn bench_centre(&mut self, place: [f32; 2]) -> Result<(), RuntimeError> {
        let frame = self.bench_view()?;
        let [x, y] = to_screen(&frame, place);
        let at = self.bench_middle()?;
        let by = [at[0] - x, at[1] - y];
        let from = [[at[0] - 150.0, at[1]], [at[0] + 150.0, at[1]]];
        let to = from.map(|p| [p[0] + by[0], p[1] + by[1]]);
        let mut watch = Watch::paced(self);
        self.bench_fingers(&mut watch, from, to, 300)?;
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        Ok(())
    }

    /// Where the open canvas shows on the screen now.
    pub(crate) fn bench_view(&self) -> Result<ViewFrame, RuntimeError> {
        let (Some(canvas), Some(area)) = (self.screen.canvas(), self.shell.canvas_area()) else {
            return Err(RuntimeError::NotNow("a canvas shown"));
        };
        Ok(canvas.view_frame((&area, self.pixels_per_point), |mapping| mapping))
    }

    /// The middle of the canvas area under the top band, in screen pixels.
    pub(crate) fn bench_middle(&self) -> Result<[f32; 2], RuntimeError> {
        let area = (self.shell.canvas_area()).ok_or(RuntimeError::NotNow("a canvas area"))?;
        let ([x, y], [w, h]) = (area.fit.origin, area.fit.size);
        Ok([x as f32 + w as f32 / 2.0, y as f32 + h as f32 / 2.0])
    }

    /// Two fingers down at `from`, moved straight to `to` over `ms` of the input clock, a batch
    /// each drawn frame, then lifted.
    pub(crate) fn bench_fingers(
        &mut self,
        watch: &mut Watch,
        from: [[f32; 2]; 2],
        to: [[f32; 2]; 2],
        ms: i64,
    ) -> Result<(), RuntimeError> {
        let steps = (ms / STEP_MS).max(1);
        let start = self.clock.now_ms() + 50;
        let at = |k: i64| -> Vec<[f32; 2]> {
            let t = k as f32 / steps as f32;
            (0..2)
                .map(|f| [0, 1].map(|axis| from[f][axis] + (to[f][axis] - from[f][axis]) * t))
                .collect()
        };
        let mut batches = vec![
            (MotionPhase::Down, at(0)[..1].to_vec(), 0),
            (MotionPhase::PointerDown { index: 1 }, at(0), 0),
        ];
        batches.extend((1..=steps).map(|k| (MotionPhase::Move, at(k), k)));
        batches.push((MotionPhase::PointerUp { index: 1 }, at(steps), steps));
        batches.push((MotionPhase::Up, at(steps)[..1].to_vec(), steps));
        // At the display's cadence the hand moves in real time and each frame takes what came
        // since; else a batch a drawn frame.
        let began = std::time::Instant::now();
        let mut batches = batches.into_iter().peekable();
        while batches.peek().is_some() {
            let due = i64::try_from(began.elapsed().as_millis()).unwrap_or(i64::MAX);
            while let Some((phase, points, k)) =
                batches.next_if(|(_, _, k)| !watch.is_paced() || k * STEP_MS <= due)
            {
                let time_ms = start + k * STEP_MS;
                self.motion(&batch(phase, ToolKind::Finger, &points, time_ms));
                if !watch.is_paced() {
                    break;
                }
            }
            self.bench_drawn(watch)?;
        }
        Ok(())
    }

    /// Frames until one is drawn (the GPU may be behind, S56).
    pub(crate) fn bench_drawn(&mut self, watch: &mut Watch) -> Result<(), RuntimeError> {
        let drawn = watch.frames.len();
        while watch.frames.len() == drawn {
            self.bench_frame(watch)?;
        }
        Ok(())
    }
}

/// One batch of one sample per pointer at `time_ms`, pressed 0.8 by a pen.
pub(crate) fn batch(
    phase: MotionPhase,
    tool: ToolKind,
    points: &[[f32; 2]],
    time_ms: i64,
) -> MotionBatch {
    let pointers = (0i32..)
        .zip(points)
        .map(|(id, position)| Pointer {
            id,
            tool,
            samples: vec![PointerSample {
                position: *position,
                pressure: if tool == ToolKind::Stylus { 0.8 } else { 1.0 },
                tilt: 0.7,
                orientation: 0.0,
                time_ms,
            }],
        })
        .collect();
    MotionBatch {
        phase,
        buttons: Buttons::default(),
        pointers,
    }
}

/// Screen pixels per canvas pixel of the view.
pub(crate) fn scale(frame: &ViewFrame) -> f32 {
    let [a, b, _, d, e, _] = frame.to_canvas;
    let area = (a * e - b * d).abs();
    if area > 0.0 { area.sqrt().recip() } else { 0.0 }
}

/// The screen pixel showing canvas pixel `place`.
pub(crate) fn to_screen(frame: &ViewFrame, place: [f32; 2]) -> [f32; 2] {
    let [a, b, c, d, e, f] = frame.to_canvas;
    let det = a * e - b * d;
    let [x, y] = [place[0] - c, place[1] - f];
    [(e * x - b * y) / det, (a * y - d * x) / det]
}

/// Strokes on every layer and mask of `artwork`.
pub(crate) fn strokes(artwork: &Artwork) -> usize {
    (artwork.layers().iter())
        .map(|l| l.strokes().len() + l.mask().map_or(0, |m| m.strokes().len()))
        .sum()
}

/// The middle of the 64-px cell the most strokes' boxes (their samples widened by their size)
/// cover: the artwork's densest place (S53's `nav-timing`).
#[must_use]
pub fn busiest(artwork: &Artwork) -> [f32; 2] {
    const CELL: f32 = 64.0;
    let canvas = artwork.canvas();
    let columns = (canvas.width() as f32 / CELL).ceil() as usize;
    let rows = (canvas.height() as f32 / CELL).ceil() as usize;
    let mut cover = vec![0u32; columns * rows];
    for stroke in (artwork.layers().iter()).flat_map(|layer| layer.strokes().iter()) {
        let size = stroke.style().size();
        let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
        for sample in stroke.samples() {
            let p = sample.position();
            low = [low[0].min(p.x() - size), low[1].min(p.y() - size)];
            high = [high[0].max(p.x() + size), high[1].max(p.y() + size)];
        }
        let span = |axis: usize, cells: usize| {
            let first = (low[axis] / CELL).floor().max(0.0) as usize;
            let last = ((high[axis] / CELL).floor().max(0.0) as usize).min(cells - 1);
            first..=last
        };
        for row in span(1, rows) {
            for column in span(0, columns) {
                cover[row * columns + column] += 1;
            }
        }
    }
    let best = (0..cover.len()).max_by_key(|at| cover[*at]).unwrap_or(0);
    [
        (best % columns) as f32 * CELL + CELL / 2.0,
        (best / columns) as f32 * CELL + CELL / 2.0,
    ]
}
