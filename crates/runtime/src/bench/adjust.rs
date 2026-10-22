//! `bench adjust`: a slider dragged in each adjustment and filter panel on the layer holding the
//! most strokes, at fit, as a hand drags it: Hue, Saturation, Brightness's hue, a point of the
//! composite curve, Gaussian Blur's radius to 30 % and Sharpen's amount, 60 values each, one a
//! frame at the display's cadence (`@fast`: one a drawn frame), then Cancel. Each panel's line:
//! the drawn frames' work while the slider moves, how many values the canvas showed complete in
//! the frame that took them, each value's time to show (from the frame taking it to the end of
//! the first frame after which the canvas is complete), and the last value's.

use std::time::Instant;

use openpaint_app::{AdjustIntent, Intent, LayerIntent, ViewIntent};
use openpaint_core::{Adjustment, AdjustmentKind, Blur, Curves, Hsb, Sharpen, ToneCurve};

use super::hand::{busiest_layer, strokes};
use super::{Watch, spread};
use crate::error::RuntimeError;
use crate::frame::Runtime;

/// Values a drag gives, one a frame.
const VALUES: usize = 60;
/// The panels, in the wand's order.
const PANELS: [AdjustmentKind; 4] = [
    AdjustmentKind::HueSaturationBrightness,
    AdjustmentKind::Curves,
    AdjustmentKind::GaussianBlur,
    AdjustmentKind::Sharpen,
];

impl Runtime {
    /// `adjust`: see the module.
    pub(crate) fn bench_adjust(&mut self, lines: &mut Vec<String>) -> Result<(), RuntimeError> {
        let (_, artwork) = self.bench_artwork()?;
        let active = artwork.active();
        let layer = busiest_layer(&artwork)?;
        lines.push(format!(
            "on the layer of {} strokes (of {}), {VALUES} values a panel; ms",
            layer.strokes().len(),
            strokes(&artwork)
        ));
        let mut watch = Watch::paced(self);
        let chosen = vec![
            Intent::View(ViewIntent::Fit),
            Intent::Layer(LayerIntent::Select(layer.id())),
        ];
        self.bench_intents(chosen, &mut watch)?;
        self.bench_until(&mut watch, Self::bench_settled)?;
        for kind in PANELS {
            let line = self.bench_panel(kind)?;
            lines.push(format!("{kind:?}: {line}"));
        }
        let mut watch = Watch::paced(self);
        self.bench_intents(vec![Intent::Layer(LayerIntent::Select(active))], &mut watch)?;
        self.bench_until(&mut watch, Self::bench_settled)?;
        Ok(())
    }

    /// One panel opened, its slider dragged over `VALUES` values, then cancelled: its line.
    fn bench_panel(&mut self, kind: AdjustmentKind) -> Result<String, RuntimeError> {
        let mut watch = Watch::paced(self);
        self.bench_intents(vec![Intent::Adjust(AdjustIntent::Begin(kind))], &mut watch)?;
        self.bench_until(&mut watch, Self::bench_settled)?;
        let mut watch = Watch::paced(self);
        let (mut set, mut complete) = (Vec::with_capacity(VALUES), Vec::new());
        let mut in_frame = 0;
        for at in 0..VALUES {
            let value = slid(kind, at as f32 / (VALUES - 1) as f32)?;
            set.push(Instant::now());
            self.bench_intents(vec![Intent::Adjust(AdjustIntent::Set(value))], &mut watch)?;
            if self.bench_shown() {
                in_frame += 1;
                complete.push(Instant::now());
            }
        }
        let work = spread(&watch.frames);
        self.bench_until(&mut watch, Self::bench_shown)?;
        complete.push(Instant::now());
        let shown: Vec<f64> = (set.iter())
            .map(|value| {
                let first = complete.iter().find(|done| *done >= value);
                first.map_or(0.0, |done| (*done - *value).as_secs_f64() * 1e3)
            })
            .collect();
        let last = shown.last().copied().unwrap_or(0.0);
        let line = format!(
            "frame work while dragged {work}, complete in their frame {in_frame} of {VALUES}, to \
             show {} (the last {last:.1}); {}",
            spread(&shown),
            watch.line(self)
        );
        let mut watch = Watch::paced(self);
        self.bench_intents(vec![Intent::Adjust(AdjustIntent::Cancel)], &mut watch)?;
        self.bench_until(&mut watch, Self::bench_settled)?;
        Ok(line)
    }

    /// The canvas shows what it draws complete (nothing of it waits).
    fn bench_shown(&self) -> bool {
        (self.screen.canvas()).is_some_and(|canvas| !canvas.filling() && !canvas.view_pending())
    }
}

/// The values of `kind`'s panel with its slider `t` of the way along its drag.
fn slid(kind: AdjustmentKind, t: f32) -> Result<Adjustment, RuntimeError> {
    Ok(match kind {
        AdjustmentKind::HueSaturationBrightness => {
            Adjustment::Hsb(Hsb::new(0.5 + 0.3 * t, 0.5, 0.5))
        }
        AdjustmentKind::Curves => {
            let point = [[0.0, 0.0], [0.5, 0.5 + 0.3 * t], [1.0, 1.0]];
            let composite = ToneCurve::new(&point).ok_or(RuntimeError::NotNow("a curve"))?;
            Adjustment::Curves(Curves {
                composite,
                red: ToneCurve::identity(),
                green: ToneCurve::identity(),
                blue: ToneCurve::identity(),
            })
        }
        AdjustmentKind::GaussianBlur => Adjustment::Blur(Blur::new(0.3 * Blur::MAX_RADIUS * t)),
        AdjustmentKind::Sharpen => Adjustment::Sharpen(Sharpen::new(t)),
    })
}
