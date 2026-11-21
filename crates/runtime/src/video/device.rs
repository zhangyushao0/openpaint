//! The video's own GPU device (S163): a `Core` the video worker opens beside the screen's, on
//! the same adapter and its backend alone, when a video starts (`Core::reopened`: no other
//! backend started on the worker's thread, whose EGL context the platform's encoder draws with,
//! and which that encoder makes current again for every frame), with the `VideoTarget`
//! drawing on it, and lets go when the video ends. What a driver keeps for a device's drawing
//! (on the tablet's, memory the process never got back while the device lived, which added up
//! export after export) goes with it. Within one video the device is made anew once it drew
//! `VideoBounds::frames` frames or the process's resident memory gained `VideoBounds::drawing`
//! bytes while it drew (where the system says, `resident.rs`): the new target draws the next
//! frame whole, which equals drawing it on from the frames before (the raster core's rule), so
//! the video is the same. The cost: a device and its pipelines made, and one frame drawn whole,
//! each time.

use std::time::Instant;

use openpaint_app::VideoError;
use openpaint_render::{Core, VideoTarget};

use super::rendering;

/// The frames one device draws at most: a 30 s highlight's 900 on two devices.
const DEVICE_FRAMES: u32 = 450;
/// The resident memory the process may gain while one device draws.
const DEVICE_GROWTH: u64 = 400 << 20;

/// How far one video's device goes before it is made anew, and its target's arena room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoBounds {
    /// The bytes the target's arena keeps within (the budget's share of the residuals).
    pub arena: u64,
    /// The frames one device draws at most.
    pub frames: u32,
    /// The resident bytes the process may gain while one device draws.
    pub drawing: u64,
}

impl VideoBounds {
    /// The app's bounds, the target's arena within `arena` bytes.
    #[must_use]
    pub const fn of(arena: u64) -> Self {
        Self {
            arena,
            frames: DEVICE_FRAMES,
            drawing: DEVICE_GROWTH,
        }
    }
}

/// The device a video draws on now, its target, and what it drew so far.
pub(super) struct VideoDevice {
    // Before the device it was made on: dropped first.
    pub(super) target: VideoTarget,
    pub(super) core: Core,
    frames: u32,
    /// The resident bytes the process gained while it drew.
    grown: i64,
}

impl VideoDevice {
    /// A new device beside `core` and a target on it for a canvas of `canvas` pixels whose
    /// frames are `video` pixels, its arena within `arena` bytes.
    ///
    /// # Errors
    /// `Render` when no device or target can be made.
    pub(super) fn open(
        core: &Core,
        (canvas, video): ([u32; 2], [u32; 2]),
        arena: u64,
    ) -> Result<Self, VideoError> {
        let started = Instant::now();
        let own = core.reopened().map_err(rendering)?;
        own.catch_errors();
        let target = VideoTarget::new(&own, canvas, (video, arena)).map_err(rendering)?;
        log::info!(
            "a device of the video's own opened in {} ms on {}",
            started.elapsed().as_millis(),
            own.adapter()
        );
        Ok(Self {
            target,
            core: own,
            frames: 0,
            grown: 0,
        })
    }

    /// One frame drawn, the resident memory having gained `grown` bytes meanwhile.
    pub(super) fn drew(&mut self, grown: i64) {
        self.frames += 1;
        self.grown += grown;
    }

    /// It went as far as `bounds` lets one device go.
    pub(super) fn spent(&self, bounds: &VideoBounds) -> bool {
        self.frames >= bounds.frames
            || self.grown >= i64::try_from(bounds.drawing).unwrap_or(i64::MAX)
    }

    /// This device and its target let go, then a new one opened as `open` opens it: what the
    /// driver kept for this one is given back first.
    ///
    /// # Errors
    /// As `open`.
    pub(super) fn made_anew(
        self,
        core: &Core,
        sizes: ([u32; 2], [u32; 2]),
        arena: u64,
    ) -> Result<Self, VideoError> {
        log::info!(
            "the video's device made anew after {} frames, {:+.1} MB resident while they were drawn",
            self.frames,
            self.grown as f64 / f64::from(1 << 20)
        );
        drop(self);
        Self::open(core, sizes, arena)
    }
}
