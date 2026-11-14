//! `bench session`: an hour's painting driven as a hand does, timed; and the session's brush,
//! which `bench strokes` sets too.

use openpaint_app::{
    CanvasPreset, Contact, ContactPhase, Device, GalleryIntent, HistoryIntent, InputSample, Intent,
    LayerIntent, StreamLine, StrokeIntent,
};
use openpaint_core::{Stroke, StrokeStyle};

use super::hand::strokes;
use super::{Watch, mb};
use crate::error::RuntimeError;
use crate::frame::Runtime;

impl Runtime {
    /// The session's next strokes drawn with `style`, without StreamLine; `None`: the toolbox's
    /// again.
    pub(super) fn set_session_brush(&mut self, style: Option<StrokeStyle>) {
        let (style, streamline) = match style {
            Some(style) => (Ok(style), StreamLine::OFF),
            None => (self.toolbox.style(), self.toolbox.streamline()),
        };
        if let (Some(canvas), Ok(style)) = (self.screen.canvas_mut(), style) {
            canvas.set_brush(style);
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
        self.bench_until(&mut watch, Self::bench_settled)?;
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
        self.bench_until(&mut watch, Self::bench_settled)?;
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
        self.bench_leave(watch)?;
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
        if self.bench_until(watch, shown)? {
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

    /// What holds memory now: the GPU caches, the device allocator, the history, the scene.
    fn bench_memory(&self) -> String {
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
