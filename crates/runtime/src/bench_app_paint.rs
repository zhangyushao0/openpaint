//! `bench session` (S58, `bench_app.rs`): painting driven as a hand does, timed; the session's
//! brush and intents `bench strokes` (`bench_app_strokes.rs`) shares.

use std::time::Duration;

use openpaint_app::{
    CanvasPreset, Contact, ContactPhase, Device, GalleryIntent, HistoryIntent, InputSample, Intent,
    LayerIntent, StrokeIntent,
};
use openpaint_core::{Stroke, StrokeStyle};

use super::Watch;
use super::cases::strokes;
use crate::error::RuntimeError;
use crate::frame::Runtime;
use crate::screen::Screen;

const LIMIT: Duration = Duration::from_secs(60);

impl Runtime {
    /// The session's next strokes drawn with `style`, without StreamLine; `None`: the toolbox's
    /// again.
    pub(super) fn set_session_brush(&mut self, style: Option<StrokeStyle>) {
        let (style, streamline) = match style {
            Some(style) => (Ok(style), openpaint_app::StreamLine::OFF),
            None => (self.toolbox.style(), self.toolbox.streamline()),
        };
        if let (Some(canvas), Ok(style)) = (self.screen.canvas_mut(), style) {
            canvas.session_mut().set_brush(style);
            canvas.session_mut().set_streamline(streamline);
        }
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
    pub(super) fn bench_intents(
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
        let scene = self.scene_bytes().unwrap_or_default();
        format!(
            "GPU caches {:.1} MB (tiers {:.1}, snapshots {:.1}, arena {:.1}), the allocator {:.1} \
             MB in blocks of {:.1}; the heap's scene {:.1} MB, history {:.1}",
            mb(caches.total()),
            mb(caches.tiers),
            mb(caches.snapshots),
            mb(caches.arena),
            mb(gpu.allocated),
            mb(gpu.reserved),
            mb(scene),
            mb(self.history_bytes().unwrap_or(0) as u64)
        )
    }
}
