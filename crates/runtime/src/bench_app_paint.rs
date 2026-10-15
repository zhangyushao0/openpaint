//! `bench strokes|session` (S58, `bench_app.rs`): painting driven as a hand does, timed.

use std::sync::Arc;
use std::time::Duration;

use openpaint_app::{
    BrushLibrary, CanvasPreset, Contact, ContactPhase, Device, GalleryIntent, HistoryIntent,
    InputSample, Intent, LayerIntent, StrokeIntent, ViewIntent,
};
use openpaint_core::{Stroke, StrokeStyle};

use super::Watch;
use super::cases::{batch, busiest, strokes, to_screen};
use crate::bench_scenes::style_of;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::input::{MotionPhase, ToolKind};
use crate::screen::Screen;

const LIMIT: Duration = Duration::from_secs(60);
/// The texture brushes, a stroke each.
const TEXTURES: [&str; 4] = ["textured-round", "flat", "opaque-paint", "speckle"];
/// A bench stroke's samples, 10 screen pixels and 3.3 ms apart, and how many a batch carries
/// (the author's pen reports about 300 a second; frames come at 120 Hz).
const SAMPLES: usize = 90;
const PER_BATCH: usize = 3;

impl Runtime {
    /// `strokes`: at fit, a stroke of each texture brush across the densest place at the size and
    /// opacity the toolbox holds for it (what the hand paints with; its first size without one).
    pub(crate) fn bench_strokes(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let place = busiest(&artwork);
        let mut watch = Watch::start(self);
        self.bench_intents(vec![Intent::View(ViewIntent::Fit)], &mut watch)?;
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        let middle = to_screen(&self.bench_view()?, place);
        lines.push(format!(
            "across the densest place ({:.0}, {:.0}), at the screen's ({:.0}, {:.0}), each brush \
             at the toolbox's size and opacity; ms",
            place[0], place[1], middle[0], middle[1]
        ));
        let library = BrushLibrary::built_in()?;
        let painting = self.toolbox.state().paint;
        for (k, key) in TEXTURES.into_iter().enumerate() {
            // The size and opacity the toolbox holds for the brush: what the hand paints with.
            let held = painting.brushes.iter().find(|held| held.key == key);
            let mut style = style_of(&library, key, held.map(|held| held.size))?;
            if let Some(held) = held {
                let recipe = Arc::clone(style.recipe());
                style = StrokeStyle::new(recipe, held.size, held.opacity, style.color())?;
            }
            let (size, opacity) = (style.size(), style.opacity());
            self.set_session_brush(Some(style.with_seed(k as u32)));
            let path: Vec<[f32; 2]> = (0..SAMPLES)
                .map(|i| {
                    let t = i as f32 / (SAMPLES - 1) as f32;
                    let y = 60.0 * (t * std::f32::consts::TAU).sin() + 40.0 * k as f32 - 60.0;
                    [middle[0] - 450.0 + 900.0 * t, middle[1] + y]
                })
                .collect();
            let line = self.bench_stroke(&path)?;
            lines.push(format!(
                "{key} at {size:.0} px, opacity {opacity:.2}: {line}"
            ));
        }
        self.set_session_brush(None);
        Ok(())
    }

    /// The session's next strokes drawn with `style`, without StreamLine; `None`: the toolbox's
    /// again.
    fn set_session_brush(&mut self, style: Option<StrokeStyle>) {
        let (style, streamline) = match style {
            Some(style) => (Ok(style), openpaint_app::StreamLine::OFF),
            None => (self.toolbox.style(), self.toolbox.streamline()),
        };
        if let (Some(canvas), Ok(style)) = (self.screen.canvas_mut(), style) {
            canvas.session_mut().set_brush(style);
            canvas.session_mut().set_streamline(streamline);
        }
    }

    /// One pen stroke through `path` (screen pixels), a sample each 3.3 ms: at the display's
    /// cadence the pen moves in real time and each frame takes what came since; `@fast`,
    /// `PER_BATCH` samples a drawn frame. Its line: frames, frame work, the pen-down's and the
    /// lift's frame, the time from the pen-down until the GPU is idle after the lift.
    fn bench_stroke(&mut self, path: &[[f32; 2]]) -> Result<String, RuntimeError> {
        let start = self.clock.now_ms() + 50;
        let mut watch = Watch::paced(self);
        let last = path.len() - 1;
        let time = |at: usize| (at as i64 * 10) / 3;
        let began = std::time::Instant::now();
        let (mut next, mut down, mut lift) = (0, None, None);
        while next <= last {
            let due = i64::try_from(began.elapsed().as_millis()).unwrap_or(i64::MAX);
            let upto = if watch.is_paced() {
                (next..=last)
                    .take_while(|at| time(*at) <= due)
                    .last()
                    .map_or(next, |at| at + 1)
            } else {
                (next + PER_BATCH).min(last + 1)
            };
            for (at, point) in path.iter().enumerate().take(upto).skip(next) {
                let phase = match at {
                    0 => MotionPhase::Down,
                    at if at == last => MotionPhase::Up,
                    _ => MotionPhase::Move,
                };
                let point = std::slice::from_ref(point);
                self.motion(&batch(phase, ToolKind::Stylus, point, start + time(at)));
            }
            let drawn = watch.frames.len();
            down = down.or((upto > 0).then_some(drawn));
            lift = lift.or((upto > last).then_some(drawn));
            next = upto;
            if watch.is_paced() {
                self.bench_frame(&mut watch)?;
            } else {
                self.bench_drawn(&mut watch)?;
            }
        }
        self.bench_drawn(&mut watch)?;
        self.core.wait_idle()?;
        let frame = |at: Option<usize>| at.and_then(|at| watch.frames.get(at).copied());
        Ok(format!(
            "{} frames ({} skipped), frame work {}; pen-down {:.1}, lift {:.1}; the GPU idle {:.0} \
             ms after the pen-down",
            watch.frames.len(),
            watch.skipped(),
            super::spread(&watch.frames),
            frame(down).unwrap_or(0.0),
            frame(lift).unwrap_or(0.0),
            watch.ms()
        ))
    }

    /// `session`: the open artwork's strokes (layer by layer, masks left out) appended as records
    /// to a new screen-size artwork, two frames a stroke (drawn, lifted), every twentieth undone
    /// and every fortieth redone, a layer added where the next stroke's layer is higher.
    pub(crate) fn bench_session(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let records: Vec<(usize, Stroke)> = (artwork.layers().iter().enumerate())
            .flat_map(|(at, layer)| layer.strokes().iter().map(move |s| (at, s.clone())))
            .collect();
        let mut watch = Watch::start(self);
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        self.bench_new_artwork(&mut watch)?;
        lines.push(format!(
            "{} strokes of {} layers onto a new screen-size artwork, project {:?}",
            strokes(&artwork),
            artwork.layers().len(),
            self.screen
                .canvas()
                .and_then(|c| c.project())
                .map(|p| p.get())
        ));
        lines.push(format!("before: {}", self.bench_memory()));
        let mut watch = Watch::start(self);
        let mut layer = 0;
        for (k, (at, stroke)) in records.iter().enumerate() {
            if *at > layer {
                self.bench_intents(vec![Intent::Layer(LayerIntent::Add)], &mut watch)?;
                layer = *at;
            }
            self.bench_record(stroke, &mut watch)?;
            if k % 20 == 19 {
                let undo = Intent::History(HistoryIntent::Undo);
                self.bench_intents(vec![undo], &mut watch)?;
            }
            if k % 40 == 39 {
                let redo = Intent::History(HistoryIntent::Redo);
                self.bench_intents(vec![redo], &mut watch)?;
            }
            if (k + 1) % 500 == 0 {
                lines.push(format!("{} strokes: {}", k + 1, watch.line(self)));
            }
        }
        self.bench_until(&mut watch, LIMIT, Self::bench_settled)?;
        self.set_session_brush(None);
        lines.push(format!(
            "all {} strokes: {}",
            records.len(),
            watch.line(self)
        ));
        lines.push(format!("after: {}", self.bench_memory()));
        Ok(())
    }

    /// The canvas left for the gallery, then New artwork, Screen size: frames until it shows.
    fn bench_new_artwork(&mut self, watch: &mut Watch) -> Result<(), RuntimeError> {
        if let Screen::Canvas(canvas) = &mut self.screen {
            let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
            canvas.dispatch(&self.core, vec![Intent::Gallery(GalleryIntent::Show)], app);
        }
        let gallery = |runtime: &Self| matches!(runtime.screen, Screen::Gallery(_));
        self.bench_until(watch, LIMIT, gallery)?;
        let create = GalleryIntent::Create(CanvasPreset::Screen);
        (self.screen.gallery_mut())
            .ok_or(RuntimeError::NotNow("the gallery"))?
            .dispatch(create)?;
        let shown = |runtime: &Self| {
            runtime
                .screen
                .canvas()
                .is_some_and(|c| c.project().is_some())
        };
        if self.bench_until(watch, LIMIT, shown)? {
            Ok(())
        } else {
            Err(RuntimeError::NotNow("a new artwork"))
        }
    }

    /// `stroke` drawn again with its own style: its samples but the last down in one frame, the
    /// last and the lift in the next.
    fn bench_record(&mut self, stroke: &Stroke, watch: &mut Watch) -> Result<(), RuntimeError> {
        let start = self.clock.now_ms() + 20;
        let samples: Vec<InputSample> = (stroke.samples().iter())
            .map(|s| InputSample {
                position: [s.position().x(), s.position().y()],
                pressure: s.pressure(),
                tilt: s.tilt(),
                azimuth: s.azimuth(),
                time_ms: start + i64::from(s.time_ms()),
            })
            .collect();
        self.set_session_brush(Some(stroke.style().clone()));
        let contact = |phase, samples: &[InputSample]| {
            Intent::Stroke(StrokeIntent::Contact(Contact {
                id: 0,
                device: Device::Pen,
                phase,
                samples: samples.to_vec(),
            }))
        };
        let (last, drawn) = samples
            .split_last()
            .ok_or(RuntimeError::NotNow("samples"))?;
        let mut first = vec![contact(ContactPhase::Down, &drawn[..1.min(drawn.len())])];
        if drawn.len() > 1 {
            first.push(contact(ContactPhase::Move, &drawn[1..]));
        }
        self.bench_intents(first, watch)?;
        self.bench_intents(
            vec![contact(ContactPhase::Up, std::slice::from_ref(last))],
            watch,
        )
    }

    /// `intents` applied to the canvas as the frame applies a hand's, then a frame drawn.
    fn bench_intents(
        &mut self,
        intents: Vec<Intent>,
        watch: &mut Watch,
    ) -> Result<(), RuntimeError> {
        if let Screen::Canvas(canvas) = &mut self.screen {
            let app = (&mut self.toolbox, &mut self.clipboard, &mut self.seeds);
            canvas.dispatch(&self.core, intents, app);
        }
        self.bench_drawn(watch)
    }

    /// What holds memory now: the GPU caches, the device allocator, the history, the scene.
    fn bench_memory(&self) -> String {
        let mb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
        let gpu = self.core.gpu_memory().unwrap_or_default();
        let caches = self.cache_bytes().unwrap_or_default();
        let [scene, plan] = self.scene_bytes().unwrap_or_default();
        format!(
            "GPU caches {:.1} MB (tiers {:.1}, snapshots {:.1}, arena {:.1}), the allocator {:.1} \
             MB in blocks of {:.1}; the heap's scene {:.1} MB, plan {:.1}, history {:.1}",
            mb(caches.total()),
            mb(caches.tiers),
            mb(caches.snapshots),
            mb(caches.arena),
            mb(gpu.allocated),
            mb(gpu.reserved),
            mb(scene),
            mb(plan),
            mb(self.history_bytes().unwrap_or(0) as u64)
        )
    }
}
