//! `long-session`'s strokes, its rows every 5 simulated minutes checked against their bounds, and
//! its end: the canvas against a cold evaluation of the artwork.

use std::f32::consts::{PI, TAU};

use openpaint_core::Color;
use openpaint_render::{CanvasTarget, FrameRequest, GpuMemory, PixelRect, RegionEvaluator};
use openpaint_runtime::PointerSample;

use super::{BOUNDS, Painter, SAMPLES, SPACING};
use crate::error::HarnessError;
use crate::long_session_store::{process_memory, size_on_disk};

/// Where strokes start and stay, in window pixels: right of the side bar, under the islands.
const LEFT: f32 = 300.0;
const RIGHT: f32 = 2600.0;
const TOP: f32 = 250.0;
const BOTTOM: f32 = 1650.0;

/// SplitMix64: the same strokes on every run.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A number from 0 to 1 drawn from `k` and `salt`.
fn unit(k: usize, salt: u64) -> f32 {
    (mix(k as u64 * 4 + salt) >> 40) as f32 / (1u64 << 24) as f32
}

/// Stroke `k`'s samples from `start_ms`: a gentle curve `SPACING` px a sample from a place and
/// direction of its own, kept inside the canvas's middle, pressure rising and falling, about
/// 3.3 ms apart.
pub(super) fn path(k: usize, start_ms: i64) -> Vec<PointerSample> {
    let mut at = [
        LEFT + 100.0 + (RIGHT - LEFT - 200.0) * unit(k, 0),
        TOP + 100.0 + (BOTTOM - TOP - 200.0) * unit(k, 1),
    ];
    let mut angle = TAU * unit(k, 2);
    let turn = (unit(k, 3) - 0.5) * 0.02;
    (0..SAMPLES)
        .map(|i| {
            let t = i as f32 / (SAMPLES - 1) as f32;
            angle += turn;
            at = [
                (at[0] + SPACING * angle.cos()).clamp(LEFT, RIGHT),
                (at[1] + SPACING * angle.sin()).clamp(TOP, BOTTOM),
            ];
            PointerSample {
                position: at,
                pressure: 0.35 + 0.5 * (PI * t).sin(),
                tilt: 0.7,
                orientation: 0.0,
                time_ms: start_ms + i as i64 * 10 / 3,
            }
        })
        .collect()
}

const MB: f64 = 1024.0 * 1024.0;

/// The columns of `report`'s rows, in MB but for the minute, the strokes and the versions:
/// the process's private memory, the Rust heap live and at its most (`heap.rs`), the device
/// allocator's live allocations and the blocks it holds them in (`Core::gpu_memory`), the
/// process beside the heap and those blocks (the driver's own), the GPU caches the app counts
/// (canvas, tiers, snapshots, arena, pictures and their total), the history, the strokes'
/// samples, the library on disk. Under each row the allocations by label, largest first.
pub(super) fn header() {
    let names = [
        "min", "strokes", "process", "heap", "peak", "gpu", "blocks", "other", "canvas", "tiers",
        "snaps", "arena", "pics", "caches", "history", "samples", "library", "vers", "wall s",
    ];
    let row: Vec<String> = names.iter().map(|name| format!("{name:>8}")).collect();
    println!("{}", row.join(""));
}

impl Painter {
    /// One row: the simulated minute, the strokes, then each measure, checked against its bound.
    pub(super) fn report(&mut self) -> Result<(), HarnessError> {
        let memory = process_memory().unwrap_or(0);
        let [heap, peak] = crate::heap::bytes();
        let gpu = self.runtime.core().gpu_memory().unwrap_or_default();
        let caches = self.runtime.cache_bytes().unwrap_or_default();
        let history = self.runtime.history_bytes().unwrap_or(0) as u64;
        let library = size_on_disk(&self.library)?;
        let versions = self.versions.lock().map_or(0, |versions| *versions);
        let (strokes, samples) = self.runtime.artwork().map_or((0, 0), |artwork| {
            let strokes = artwork
                .layers()
                .iter()
                .flat_map(|layer| layer.strokes().iter());
            strokes.fold((0, 0), |(count, bytes), stroke| {
                (count + 1, bytes + size_of_val(stroke.samples()) as u64)
            })
        });
        let mb = |bytes: u64| format!("{:>8.1}", bytes as f64 / MB);
        let sizes = [
            memory,
            heap,
            peak,
            gpu.allocated,
            gpu.reserved,
            memory.saturating_sub(heap + gpu.reserved),
            caches.canvas,
            caches.tiers,
            caches.snapshots,
            caches.arena,
            caches.pictures,
            caches.total(),
            history,
            samples,
            library,
        ];
        let sizes: String = sizes.into_iter().map(mb).collect();
        let minute = self.clock / 60_000;
        let wall = self.started.elapsed().as_secs_f64();
        println!("{minute:>8}{strokes:>8}{sizes}{versions:>8}{wall:>8.0}");
        self.labels(&gpu);
        if minute >= 10 && self.settled.is_none() {
            self.settled = Some(memory);
        }
        let growth = memory.saturating_sub(self.settled.unwrap_or(memory));
        let checks = [
            ("process memory since minute 10", growth, BOUNDS.growth),
            ("live Rust heap", heap, BOUNDS.heap),
            ("arena", caches.arena, BOUNDS.arena),
            ("GPU caches", caches.total(), BOUNDS.caches),
            ("history", history, BOUNDS.history),
            ("library on disk", library, BOUNDS.library),
            ("versions kept", versions as u64, BOUNDS.versions as u64),
        ];
        match checks.into_iter().find(|(_, value, bound)| value > bound) {
            Some((what, value, bound)) => Err(HarnessError::Bound { what, value, bound }),
            None => Ok(()),
        }
    }

    /// Under a row: the scene and plan the canvas keeps, fjall's cache and memtables, and the
    /// device allocator's allocations by label, largest first.
    fn labels(&self, gpu: &GpuMemory) {
        let [cache, memtables] = self.fjall.lock().map_or([0; 2], |fjall| *fjall);
        let [scene, plan] = self.runtime.scene_bytes().unwrap_or_default();
        let named = [
            ("scene", scene),
            ("plan", plan),
            ("fjall cache", cache),
            ("memtables", memtables),
        ];
        let labels: Vec<String> = (named.into_iter())
            .chain(
                gpu.by_label
                    .iter()
                    .take(8)
                    .map(|(label, bytes)| (label.as_str(), *bytes)),
            )
            .map(|(label, bytes)| format!("{label} {:.1}", bytes as f64 / MB))
            .collect();
        println!("{:>16}{}", "", labels.join(", "));
    }

    /// The end: everything settled, the canvas against the artwork evaluated cold on the same
    /// device.
    pub(super) fn finish(&mut self) -> Result<(), HarnessError> {
        self.settle()?;
        let core = self.runtime.core();
        let artwork = self
            .runtime
            .artwork()
            .ok_or(HarnessError::Missing("no canvas at the end"))?
            .clone();
        let canvas = artwork.canvas();
        let size = [canvas.width(), canvas.height()];
        let mut target = CanvasTarget::new(&core, size)?;
        let request = FrameRequest {
            artwork: &artwork,
            active: None,
            background: Color::WHITE,
        };
        RegionEvaluator::new(&core, &mut target).evaluate(&request, &[PixelRect::full(size)])?;
        let cold = target.read(&core)?;
        let shown = self
            .runtime
            .read_canvas()?
            .ok_or(HarnessError::Missing("no canvas at the end"))?;
        let (shown_hash, cold_hash) = (shown.content_hash(), cold.content_hash());
        println!("canvas {shown_hash:016x}, cold evaluation {cold_hash:016x}");
        if shown_hash == cold_hash {
            println!("the canvas equals a cold evaluation of the artwork");
            return Ok(());
        }
        let pairs = shown.pixels().iter().zip(cold.pixels());
        let differing = pairs.clone().filter(|(a, b)| a != b).count();
        let most = pairs.map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0);
        println!("{differing} channels differ, by at most {most}");
        Err(HarnessError::Golden {
            expected: cold_hash,
            actual: shown_hash,
        })
    }
}
