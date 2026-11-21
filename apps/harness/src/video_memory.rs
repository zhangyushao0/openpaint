//! `video-memory`: the replay's video of `kinds-300` (test-art: 2880 x 1716, six layers, 449
//! strokes with smudges, filters and adjustments) made as the tablet made it when it ran out of
//! memory (S163, 2026-11-21): the app at the tablet's size and density with its GPU budget
//! (2,831 MB), the artwork the one its library opens, its canvas settled with the
//! snapshots under the next stroke taken ahead, the replay opened and its 30 s highlight exported
//! at 1080p (900 frames, the end held with its line) through the whole frame loop, the video
//! worker drawing beside the screen's frames. The encoder stops at every `EVERY`th frame until
//! the frame loop has read the memory: a row of the process's private memory, the Rust heap, the
//! screen's device's allocations and the blocks it holds them in (the video draws on a device of
//! its own, let go when it ends), the screen's GPU caches and the video's, the longest a screen
//! frame waited for the GPU since the row before (how far the video's work stood in its way),
//! under it the allocations by label; a last row once the video ended, which the process comes
//! back to after each video. Then the same video again.
//!
//! Checks, once every row is printed: while the second video is made, the allocations and the
//! blocks pass the most the first one took by at most `GROWTH` (what a video makes on first need
//! is made once; nothing the app holds grows per frame or per video), and wgpu's live objects of
//! every kind (`Core::gpu_objects`) never pass the first one's most; the screen holds no view
//! tier and no raster target while a video is made (its canvas, the arena's blocks its scene
//! shows and the snapshot pages a replay standing half way along a stroke holds stay); the
//! screen's caches and the video's within the budget. How much the process grew with each video
//! is printed beside them, not bounded: on the development machine (NVIDIA, Vulkan) memory
//! outside the heap and the allocator grows with the raster core's work, the video's or not
//! (STATUS, known limits). Run it with `--release`.

use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use openpaint_app::{
    Delivery, HostAnswer, HostRequest, ReplayIntent, VideoEncoder, VideoError, VideoOrder,
};
use openpaint_app::{ProjectStore, Title};
use openpaint_render::{CacheBytes, GpuMemory, GpuObjects};
use openpaint_runtime::{Runtime, StartOptions, take_spent};
use openpaint_store::Library;

use crate::error::HarnessError;
use crate::launch::{fresh_library, library_dir};
use crate::long_session::process_memory;
use crate::test_art::{Kind, make};

/// The MovinkPad 14 and its GPU budget (a quarter of its 11.3 GB, as its log says).
const SIZE: [u32; 2] = [2880, 1800];
const PIXELS_PER_POINT: f32 = 1.75;
const BUDGET: u64 = 2831 << 20;
/// A row every this many video frames.
const EVERY: u32 = 30;
/// What the allocator's allocations and blocks may pass the first video's most by while the
/// second is made.
const GROWTH: u64 = 32 << 20;
/// The frame loop's held clock steps a vsync of the tablet's 120 Hz.
const VSYNC_MS: i64 = 8;
/// A guard against a hang, wall time, never a measure.
const HANG: Duration = Duration::from_secs(1800);

/// One row's measures; the screen's GPU caches without
/// the video's, and the longest a screen frame waited for the GPU since the row before.
#[derive(Debug, Clone, Default)]
struct Row {
    process: u64,
    heap: u64,
    gpu: GpuMemory,
    screen: CacheBytes,
    video: u64,
    waited_ms: f64,
    objects: GpuObjects,
    wall: f64,
}

/// Where the encoder stops for the frame loop: the frame it waits at.
#[derive(Debug, Default)]
struct Gate {
    waiting: Option<u32>,
}

type Shared = Arc<(Mutex<Gate>, Condvar)>;

fn locked(shared: &Shared) -> MutexGuard<'_, Gate> {
    shared.0.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The encoder the frame loop lends the replay: it takes every frame, stopping at every
/// `EVERY`th until the frame loop read the memory.
struct Sampled {
    shared: Shared,
    frames: u32,
    size: usize,
}

impl VideoEncoder for Sampled {
    fn begin(&mut self, order: &VideoOrder) -> Result<(), VideoError> {
        self.size = order.size[0] as usize * order.size[1] as usize * 4;
        println!(
            "video {} x {}, {} fps",
            order.size[0], order.size[1], order.fps
        );
        Ok(())
    }

    fn frame(&mut self, rgba: &[u8], _at_us: u64) -> Result<(), VideoError> {
        assert_eq!(rgba.len(), self.size, "a frame of the video's size");
        if self.frames.is_multiple_of(EVERY) {
            let started = Instant::now();
            let mut gate = locked(&self.shared);
            gate.waiting = Some(self.frames);
            while gate.waiting.is_some() && started.elapsed() < HANG {
                gate = (self.shared.1.wait_timeout(gate, HANG))
                    .unwrap_or_else(PoisonError::into_inner)
                    .0;
            }
        }
        self.frames += 1;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), VideoError> {
        Ok(())
    }

    fn cancel(&mut self) {}
}

pub fn run() -> Result<(), HarnessError> {
    let started = Instant::now();
    let mut runtime = opened()?;
    runtime.set_memory_budget(BUDGET);
    let mut clock = 0;
    settle(&mut runtime, &mut clock)?;
    println!("adapter {}", runtime.adapter());
    println!("settled in {:.1} s", started.elapsed().as_secs_f64());
    print_row("before", &row(&runtime, 0, started));
    // The author watched it for a while: it stands somewhere along a stroke.
    runtime.replay_intent(ReplayIntent::Open);
    for _ in 0..120 {
        frame(&mut runtime, &mut clock)?;
    }
    let first = made(&mut runtime, &mut clock, "first")?;
    runtime.replay_intent(ReplayIntent::Dismiss);
    let second = made(&mut runtime, &mut clock, "second")?;
    check(&first, &second)
}

/// One video exported and made through the frame loop, a row every `EVERY` frames; `name`
/// tells it in the output.
fn made(runtime: &mut Runtime, clock: &mut i64, name: &str) -> Result<Vec<Row>, HarnessError> {
    let shared = Shared::default();
    export(runtime, clock, &shared)?;
    let made = Instant::now();
    let mut rows = Vec::new();
    let mut longest = 0;
    while runtime.making_video() {
        let _ = take_spent();
        frame(runtime, clock)?;
        longest = longest.max(take_spent().longest_ns);
        let waiting = locked(&shared).waiting;
        if let Some(index) = waiting {
            let read = row(runtime, std::mem::take(&mut longest), made);
            print_row(&format!("{index}"), &read);
            rows.push(read);
            locked(&shared).waiting = None;
            shared.1.notify_all();
        }
        if made.elapsed() > HANG {
            return Err(HarnessError::Missing("the video was never made"));
        }
    }
    println!("{name} video made in {:.1} s", made.elapsed().as_secs_f64());
    for _ in 0..30 {
        frame(runtime, clock)?;
    }
    print_row("after", &row(runtime, longest, made));
    Ok(rows)
}

/// The app on a fresh library holding `kinds-300` as the artwork open last, which it opens.
fn opened() -> Result<Runtime, HarnessError> {
    let dir = fresh_library(library_dir("video-memory"))?;
    {
        let title = Title::new("Test art kinds 300").map_err(|_| HarnessError::Missing("title"))?;
        let mut library = Library::at(&dir);
        let id = library
            .create(&make(Kind::Kinds(300))?, &title)
            .map_err(HarnessError::Port)?;
        library.load(id).map_err(HarnessError::Port)?;
    }
    let library = Library::at(&dir);
    let mut runtime = Runtime::start(PIXELS_PER_POINT, Box::new(library), StartOptions::default())?;
    let started = Instant::now();
    let mut clock = 0;
    while runtime.artwork().is_none() {
        frame(&mut runtime, &mut clock)?;
        if started.elapsed() > HANG {
            return Err(HarnessError::Missing("the artwork never opened"));
        }
    }
    Ok(runtime)
}

/// One frame of the screen, the held clock a vsync on.
fn frame(runtime: &mut Runtime, clock: &mut i64) -> Result<(), HarnessError> {
    *clock += VSYNC_MS;
    runtime.fix_clock(*clock);
    runtime.frame_offscreen(SIZE)?;
    Ok(())
}

/// Frames until the canvas rests with everything taken ahead, as after the author looked at it.
fn settle(runtime: &mut Runtime, clock: &mut i64) -> Result<(), HarnessError> {
    let started = Instant::now();
    frame(runtime, clock)?;
    while runtime.moving() || runtime.canvas_waiting() {
        frame(runtime, clock)?;
        if started.elapsed() > HANG {
            return Err(HarnessError::Missing("the canvas never settled"));
        }
    }
    Ok(())
}

/// Export video… › Save to album with the panel's first choices (the 30 s highlight, 1080p, the
/// end held with its line), the encoder lent at once as the Android host lends it.
fn export(runtime: &mut Runtime, clock: &mut i64, shared: &Shared) -> Result<(), HarnessError> {
    runtime.replay_intent(ReplayIntent::Export(Delivery::Save));
    frame(runtime, clock)?;
    match runtime.host_request() {
        Some(HostRequest::Video { .. }) => {}
        _ => return Err(HarnessError::Missing("the replay asked for no encoder")),
    }
    let encoder = Sampled {
        shared: Arc::clone(shared),
        frames: 0,
        size: 0,
    };
    runtime.answer(HostAnswer::Video(Box::new(encoder)));
    frame(runtime, clock)
}

/// The row now, the screen's frames having waited `longest_ns` at most since the last.
fn row(runtime: &Runtime, longest_ns: u64, since: Instant) -> Row {
    let caches = runtime.cache_bytes().unwrap_or_default();
    Row {
        process: process_memory().unwrap_or(0),
        heap: crate::heap::bytes()[0],
        gpu: runtime.core().gpu_memory().unwrap_or_default(),
        screen: CacheBytes { video: 0, ..caches },
        video: caches.video,
        waited_ms: longest_ns as f64 / 1e6,
        objects: runtime.core().gpu_objects().unwrap_or_default(),
        wall: since.elapsed().as_secs_f64(),
    }
}

const MB: f64 = 1024.0 * 1024.0;

fn print_row(name: &str, row: &Row) {
    let mb = |bytes: u64| format!("{:>8.1}", bytes as f64 / MB);
    let screen = &row.screen;
    let sizes = [
        row.process,
        row.heap,
        row.gpu.allocated,
        row.gpu.reserved,
        screen.total(),
        screen.tiers,
        screen.raster,
        screen.snapshots,
        screen.arena,
        row.video,
    ];
    let sizes: String = sizes.into_iter().map(mb).collect();
    if name == "before" {
        let names = [
            "frame", "process", "heap", "gpu", "blocks", "screen", "tiers", "raster", "snaps",
            "arena", "video", "wait ms", "wall s",
        ];
        let header: Vec<String> = names.iter().map(|name| format!("{name:>8}")).collect();
        println!("{}", header.join(""));
    }
    println!("{name:>8}{sizes}{:>8.1}{:>8.1}", row.waited_ms, row.wall);
    let labels: Vec<String> = (row.gpu.by_label.iter().take(8))
        .map(|(label, bytes)| format!("{label} {:.1}", *bytes as f64 / MB))
        .collect();
    println!("{:>16}{}", "", labels.join(", "));
    println!("{:>16}wgpu objects {:?}", "", row.objects);
}

/// The checks on the rows read while the two videos were made (see the module).
fn check(first: &[Row], second: &[Row]) -> Result<(), HarnessError> {
    let both = || first.iter().chain(second);
    let most = |rows: &[Row], measure: fn(&Row) -> u64| rows.iter().map(measure).max();
    let again = |measure: fn(&Row) -> u64| {
        let (once, twice) = (most(first, measure), most(second, measure));
        twice.unwrap_or(0).saturating_sub(once.unwrap_or(0))
    };
    let screen = both()
        .map(|row| row.screen.tiers + row.screen.raster)
        .max()
        .unwrap_or(0);
    let caches = both().map(|row| row.screen.total() + row.video).max();
    let grew = |rows: &[Row]| {
        let process = rows.iter().map(|row| row.process);
        process.clone().max().unwrap_or(0) - process.min().unwrap_or(0)
    };
    println!(
        "the process grew {:.1} MB over the first video, {:.1} over the second (not bounded)",
        grew(first) as f64 / MB,
        grew(second) as f64 / MB
    );
    let (once, twice) = (most_objects(first), most_objects(second));
    println!("wgpu objects at most {once:?} in the first video, {twice:?} in the second");
    let more = once.iter().zip(twice).map(|(a, b)| b.saturating_sub(*a));
    let checks = [
        (
            "allocations, the second video",
            again(|row| row.gpu.allocated),
            GROWTH,
        ),
        (
            "blocks, the second video",
            again(|row| row.gpu.reserved),
            GROWTH,
        ),
        ("the screen's tiers and raster targets", screen, 0),
        (
            "the screen's and the video's caches",
            caches.unwrap_or(0),
            BUDGET,
        ),
    ];
    for (what, value, bound) in checks {
        println!(
            "{what}: {:.1} MB (bound {:.1})",
            value as f64 / MB,
            bound as f64 / MB
        );
    }
    let objects = (
        "wgpu objects past the first video's most",
        more.sum::<usize>() as u64,
        0,
    );
    match (checks.into_iter().chain([objects])).find(|(_, value, bound)| value > bound) {
        Some((what, value, bound)) => Err(HarnessError::Bound { what, value, bound }),
        None => Ok(()),
    }
}

/// The most wgpu objects of each kind (buffers, textures, views, bind groups, command
/// encoders, command buffers, query sets, pipelines) the rows saw.
fn most_objects(rows: &[Row]) -> [usize; 8] {
    rows.iter().fold([0; 8], |most, row| {
        let o = &row.objects;
        let now = [
            o.buffers,
            o.textures,
            o.views,
            o.bind_groups,
            o.command_encoders,
            o.command_buffers,
            o.query_sets,
            o.pipelines,
        ];
        std::array::from_fn(|kind| most[kind].max(now[kind]))
    })
}
