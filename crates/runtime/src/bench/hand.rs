//! The benchmarks' hand and what it looks at: input batches, two fingers moving as a hand moves
//! them, where the canvas shows, the artwork's densest place.

use openpaint_core::Artwork;
use openpaint_render::ViewFrame;

use super::Watch;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};

/// The fingers' batches: one each 8 ms of the input clock.
const STEP_MS: i64 = 8;

impl Runtime {
    /// The open artwork's project and a copy of it; `NotNow` without one from the library.
    pub(crate) fn bench_artwork(
        &self,
    ) -> Result<(openpaint_app::ProjectId, Artwork), RuntimeError> {
        let none = RuntimeError::NotNow("an artwork of the library open");
        let canvas = self.screen.canvas().ok_or(none.clone())?;
        let project = canvas.project().ok_or(none)?;
        Ok((project, canvas.session().artwork().clone()))
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

    /// Two fingers down at the first places of `path`, moved straight from each to the next over
    /// `ms` of the input clock in all, a batch each drawn frame, then lifted.
    pub(crate) fn bench_fingers(
        &mut self,
        watch: &mut Watch,
        path: &[[[f32; 2]; 2]],
        ms: i64,
    ) -> Result<(), RuntimeError> {
        let legs = path.len().saturating_sub(1).max(1);
        let steps = (ms / STEP_MS).max(legs as i64);
        let start = self.clock.now_ms() + 50;
        let at = |k: i64| -> Vec<[f32; 2]> {
            let along = k as f32 / steps as f32 * legs as f32;
            let leg = (along as usize).min(legs - 1);
            let t = along - leg as f32;
            let (from, to) = (path[leg], path[(leg + 1).min(path.len() - 1)]);
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

/// The layer holding the most strokes of `artwork`.
pub(crate) fn busiest_layer(artwork: &Artwork) -> Result<&openpaint_core::Layer, RuntimeError> {
    (artwork.layers().iter())
        .max_by_key(|layer| layer.strokes().len())
        .ok_or(RuntimeError::NotNow("a layer"))
}

/// The middle of the 64-px cell the most strokes' boxes (their samples widened by their size)
/// cover: the artwork's densest place.
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
