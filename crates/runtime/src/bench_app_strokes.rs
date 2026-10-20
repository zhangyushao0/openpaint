//! `bench strokes` (S58, S63; `bench_app.rs`): at fit, a stroke of each brush the author paints
//! with across the artwork's densest place at the size and opacity the toolbox holds for it (its
//! first size without one): Textured Round, Opaque Paint, Soft Brush, Flat, Speckle, Ink Pen and
//! the smudge's brush; then Opaque Paint's first stroke after an opening, after an undo and after a
//! layer switch, beside a later one. Each stroke's line: its frames drawn and skipped, the frame
//! calls' work, each drawn frame's GPU time (timestamps from its first submission to its end, S63),
//! from each sample's event to the return of the present of the frame taking it (at the display's
//! cadence only: `@fast` feeds samples as frames take them), the pen-down's and the lift's frames
//! (and the scene the lift's built for the version holding the stroke; S64: the scenes that took
//! it in place instead), the most the GPU caches held while it was drawn, and how long the
//! snapshots under it took to be taken ahead before it began (it waits for them).

use std::sync::Arc;
use std::time::{Duration, Instant};

use openpaint_app::{
    BrushChoice, BrushLibrary, GalleryIntent, HistoryIntent, Intent, LayerIntent, ViewIntent,
};
use openpaint_core::{StrokeStyle, StrokeTool};

use super::cases::{batch, busiest, to_screen};
use super::{Watch, spread};
use crate::canvas::Canvas;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::{MotionPhase, ToolKind};
use crate::painted::style_of;
use openpaint_render::Counts;

const LIMIT: Duration = Duration::from_secs(60);
/// The brushes a stroke each, then the smudge's.
const BRUSHES: [&str; 6] = [
    "textured-round",
    "opaque-paint",
    "soft-brush",
    "flat",
    "speckle",
    "ink-pen",
];
/// The brush the first strokes are drawn with: the heaviest at the author's size.
const FIRST: &str = "opaque-paint";
/// Strokes `strokes` adds to the artwork: one a brush and the smudge; after the opening; one
/// undone and one after it; after the layer switch; a later one.
pub const STROKES_ADDED: usize = BRUSHES.len() + 1 + 1 + 1 + 1;
/// A bench stroke's samples, 10 screen pixels and 3.3 ms apart, and how many a batch carries
/// `@fast` (the author's pen reports about 300 a second; frames come at 120 Hz).
const SAMPLES: usize = 90;
const PER_BATCH: usize = 3;

/// What one stroke took.
#[derive(Default)]
struct Stroked {
    /// Drawn frames' work and GPU time, from each sample to its frame's present, ms.
    work: Vec<f64>,
    gpu: Vec<f64>,
    to_present: Vec<f64>,
    skipped: usize,
    down: f64,
    /// The lift's frame, and the scene of the version committing the stroke built in it, ms.
    lift: f64,
    lift_scene: f64,
    /// How long the snapshots under it took to be taken ahead before it began, ms.
    ahead: f64,
    /// The most the GPU caches and their snapshots held meanwhile, bytes.
    caches: u64,
    snapshots: u64,
    /// Drawn frames' time drawing the canvas and waiting for the GPU, ms; what render counted.
    drawing: Vec<f64>,
    waiting: Vec<f64>,
    counts: Counts,
}

impl Stroked {
    fn line(&self) -> String {
        let mb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
        format!(
            "{} frames ({} skipped), frame work {}, GPU a frame {}, event to present {}; pen-down \
             {:.1}, lift {:.1} (its scene built in {:.1}); GPU caches {:.1} MB at most (snapshots \
             {:.1}); drawing {}, waiting {}; raster regions {}, snapshots taken {} (ahead {:.0} \
             before it), scenes taking strokes in place {}",
            self.work.len(),
            self.skipped,
            spread(&self.work),
            spread(&self.gpu),
            spread(&self.to_present),
            self.down,
            self.lift,
            self.lift_scene,
            mb(self.caches),
            mb(self.snapshots),
            spread(&self.drawing),
            spread(&self.waiting),
            self.counts.rastered,
            self.counts.snapshots,
            self.ahead,
            self.counts.appended
        )
    }
}

impl Runtime {
    /// `strokes`: see the module.
    pub(crate) fn bench_strokes(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let place = busiest(&artwork);
        let middle = self.bench_fit(place)?;
        lines.push(format!(
            "across the densest place ({:.0}, {:.0}), at the screen's ({:.0}, {:.0}), each brush \
             at the toolbox's size and opacity; ms",
            place[0], place[1], middle[0], middle[1]
        ));
        let library = BrushLibrary::built_in()?;
        let state = self.toolbox.state();
        for (k, key) in BRUSHES.into_iter().enumerate() {
            let style = held_style(&library, &state.paint, key, StrokeTool::Paint)?;
            let stroked = self.bench_brush(style.with_seed(k as u32), middle, k)?;
            lines.push(format!("{key} {stroked}"));
        }
        let smudge = state.smudge.brush.clone();
        let style = held_style(&library, &state.smudge, &smudge, StrokeTool::Smudge)?;
        let stroked = self.bench_brush(style, middle, BRUSHES.len())?;
        lines.push(format!("smudge with {smudge} {stroked}"));
        let first = held_style(&library, &state.paint, FIRST, StrokeTool::Paint)?;
        self.bench_first_strokes(&first, place, lines)?;
        self.set_session_brush(None);
        Ok(())
    }

    /// The view at fit, frames until it settled; where canvas pixel `place` shows then.
    fn bench_fit(&mut self, place: [f32; 2]) -> Result<[f32; 2], RuntimeError> {
        let mut watch = Watch::start(self);
        self.bench_intents(vec![Intent::View(ViewIntent::Fit)], &mut watch)?;
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        Ok(to_screen(&self.bench_view()?, place))
    }

    /// A stroke of `style` across `middle`, the `k`th: its line.
    fn bench_brush(
        &mut self,
        style: StrokeStyle,
        middle: [f32; 2],
        k: usize,
    ) -> Result<String, RuntimeError> {
        let (size, opacity) = (style.size(), style.opacity());
        self.set_session_brush(Some(style));
        let path: Vec<[f32; 2]> = (0..SAMPLES)
            .map(|i| {
                let t = i as f32 / (SAMPLES - 1) as f32;
                let y = 60.0 * (t * std::f32::consts::TAU).sin() + 30.0 * k as f32 - 90.0;
                [middle[0] - 450.0 + 900.0 * t, middle[1] + y]
            })
            .collect();
        let stroked = self.bench_stroke(&path)?;
        Ok(format!(
            "at {size:.0} px, opacity {opacity:.2}: {}",
            stroked.line()
        ))
    }

    /// Opaque Paint's first stroke after an opening, after an undo (of the stroke before) and
    /// after a layer switch (to another layer and back, or to a new one and back), then a later
    /// one, each at fit across `place` once the canvas settled.
    fn bench_first_strokes(
        &mut self,
        style: &StrokeStyle,
        place: [f32; 2],
        lines: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        let (project, _) = self.bench_artwork()?;
        let mut watch = Watch::paced(self);
        self.bench_leave(&mut watch)?;
        self.bench_until(&mut watch, LIMIT, |runtime| !runtime.cache.busy())?;
        let gallery = (self.screen.gallery_mut()).ok_or(RuntimeError::NotNow("the gallery"))?;
        gallery.dispatch(GalleryIntent::Open(project))?;
        let shown = |runtime: &Self| runtime.screen.canvas().is_some();
        self.bench_until(&mut watch, LIMIT, shown)?;
        let mut first = Vec::new();
        let middle = self.bench_fit(place)?;
        first.push(("after an opening", self.bench_line(style, middle)?));
        self.bench_settle_after(vec![Intent::History(HistoryIntent::Undo)])?;
        first.push(("after an undo", self.bench_line(style, middle)?));
        self.bench_switch()?;
        first.push(("after a layer switch", self.bench_line(style, middle)?));
        first.push(("a later one", self.bench_line(style, middle)?));
        lines.push(format!(
            "first strokes of {FIRST} at {:.0} px:",
            style.size()
        ));
        for (when, line) in first {
            lines.push(format!("  {when}: {line}"));
        }
        Ok(())
    }

    /// A stroke of `style` across `middle` once the canvas settled: its line.
    fn bench_line(
        &mut self,
        style: &StrokeStyle,
        middle: [f32; 2],
    ) -> Result<String, RuntimeError> {
        let mut watch = Watch::paced(self);
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        self.set_session_brush(Some(style.clone()));
        let path: Vec<[f32; 2]> = (0..SAMPLES)
            .map(|i| {
                let t = i as f32 / (SAMPLES - 1) as f32;
                [middle[0] - 450.0 + 900.0 * t, middle[1] - 40.0 * t]
            })
            .collect();
        Ok(self.bench_stroke(&path)?.line())
    }

    /// `intents` applied, then frames until the canvas settled.
    fn bench_settle_after(&mut self, intents: Vec<Intent>) -> Result<(), RuntimeError> {
        let mut watch = Watch::paced(self);
        self.bench_intents(intents, &mut watch)?;
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        Ok(())
    }

    /// The active layer left for another (the one below, else above, else a new one) and taken
    /// again.
    fn bench_switch(&mut self) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let active = artwork.active();
        let layers = artwork.layers();
        let at = (layers.iter()).position(|layer| layer.id() == active);
        let other = at.and_then(|at| {
            (at.checked_sub(1).and_then(|below| layers.get(below)))
                .or_else(|| layers.get(at + 1))
                .map(openpaint_core::Layer::id)
        });
        let away = match other {
            Some(other) => Intent::Layer(LayerIntent::Select(other)),
            None => Intent::Layer(LayerIntent::Add),
        };
        self.bench_settle_after(vec![away])?;
        self.bench_settle_after(vec![Intent::Layer(LayerIntent::Select(active))])
    }

    /// One pen stroke through `path` (screen pixels), a sample each 3.3 ms: at the display's
    /// cadence the pen moves in real time and each frame takes what came since; `@fast`,
    /// `PER_BATCH` samples a drawn frame.
    fn bench_stroke(&mut self, path: &[[f32; 2]]) -> Result<Stroked, RuntimeError> {
        let mut stroked = Stroked::default();
        let ahead = Instant::now();
        let mut watch = Watch::paced(self);
        let taken = |runtime: &Self| !(runtime.screen.canvas()).is_some_and(Canvas::taking_ahead);
        self.bench_until(&mut watch, LIMIT, taken)?;
        stroked.ahead = ahead.elapsed().as_secs_f64() * 1e3;
        let start = self.clock.now_ms() + 50;
        let mut watch = Watch::paced(self);
        self.core.wait_idle()?;
        let _ = self.bench_frame_times();
        self.bench_time_frames(true);
        let last = path.len() - 1;
        let time = |at: usize| (at as i64 * 10) / 3;
        let began = Instant::now();
        let counted = self.core.counts();
        let (mut next, mut waiting) = (0, Vec::new());
        while next <= last || !waiting.is_empty() {
            let due = i64::try_from(began.elapsed().as_millis()).unwrap_or(i64::MAX);
            let upto = if !watch.is_paced() {
                (next + PER_BATCH).min(last + 1)
            } else {
                (next..=last)
                    .take_while(|at| time(*at) <= due)
                    .last()
                    .map_or(next, |at| at + 1)
            };
            for (at, point) in path.iter().enumerate().take(upto).skip(next) {
                let phase = match at {
                    0 => MotionPhase::Down,
                    at if at == last => MotionPhase::Up,
                    _ => MotionPhase::Move,
                };
                let point = std::slice::from_ref(point);
                self.motion(&batch(phase, ToolKind::Stylus, point, start + time(at)));
                let event = began + Duration::from_millis(time(at).unsigned_abs());
                waiting.push((at, event.max(began)));
            }
            next = upto;
            self.bench_timed(&mut watch, &mut stroked, &mut waiting)?;
        }
        self.bench_drawn(&mut watch)?;
        self.core.wait_idle()?;
        let gpu = self.bench_frame_times();
        stroked.gpu.extend(gpu);
        self.bench_time_frames(false);
        stroked.skipped = watch.skipped();
        stroked.counts = self.core.counts().since(counted);
        Ok(stroked)
    }

    /// One frame call of a stroke: a drawn frame takes the samples `waiting` (their index and
    /// event), each counted from its event to the frame's return (at the display's cadence), and
    /// its work, the GPU times read back since and the caches held.
    fn bench_timed(
        &mut self,
        watch: &mut Watch,
        stroked: &mut Stroked,
        waiting: &mut Vec<(usize, Instant)>,
    ) -> Result<(), RuntimeError> {
        let drawn = watch.frames.len();
        let (drawn_ns, waited) = (watch.drawn, watch.waited);
        self.bench_call(watch)?;
        let returned = Instant::now();
        let skipped = watch.frames.len() == drawn;
        if !skipped {
            let work = watch.frames[drawn];
            stroked.work.push(work);
            let ms = |ns: u64| ns as f64 / 1e6;
            stroked.drawing.push(ms(watch.drawn - drawn_ns));
            stroked.waiting.push(ms(watch.waited - waited));
            let last = SAMPLES - 1;
            if waiting.iter().any(|(at, _)| *at == 0) {
                stroked.down = work;
            }
            if waiting.iter().any(|(at, _)| *at == last) {
                stroked.lift = work;
                stroked.lift_scene = ms(watch.last.scene_ns);
            }
            if watch.is_paced() {
                let ms = |event: &Instant| (returned - *event).as_secs_f64() * 1e3;
                stroked
                    .to_present
                    .extend(waiting.iter().map(|(_, event)| ms(event)));
            }
            waiting.clear();
            if let Some(held) = self.cache_bytes() {
                stroked.caches = stroked.caches.max(held.total());
                stroked.snapshots = stroked.snapshots.max(held.snapshots);
            }
        }
        let gpu = self.bench_frame_times();
        stroked.gpu.extend(gpu);
        watch.wait(skipped);
        Ok(())
    }
}

/// The style of brush `key` for `tool` at the size and opacity `held` (the toolbox's choice for
/// the tool) holds for it, its first size and opacity without one.
fn held_style(
    library: &BrushLibrary,
    held: &BrushChoice,
    key: &str,
    tool: StrokeTool,
) -> Result<StrokeStyle, RuntimeError> {
    let setting = held.brushes.iter().find(|setting| setting.key == key);
    let mut style = style_of(library, key, setting.map(|setting| setting.size))?;
    if let Some(setting) = setting {
        let recipe = Arc::clone(style.recipe());
        style = StrokeStyle::new(recipe, setting.size, setting.opacity, style.color())?;
    }
    Ok(style.with_tool(tool))
}
