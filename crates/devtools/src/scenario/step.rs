//! A scenario's steps: what the user does, and what the runner looks at.

use serde::Deserialize;

use super::{hex_hash, optional_hash};
use crate::host::HostExpect;
use crate::lesson::Lesson;
use crate::pen::Pen;
use crate::system::SystemAnswer;
use crate::target::{Key, Spot};
use crate::video::VideoCheck;

/// One thing the user does, or one thing the runner looks at.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    /// Touch down and lift at one spot.
    Tap {
        at: Spot,
        #[serde(default = "Tool::finger")]
        tool: Tool,
    },
    /// A straight drag at full pressure.
    Drag {
        from: Spot,
        to: Spot,
        #[serde(default = "Tool::finger")]
        tool: Tool,
    },
    /// A pen stroke through `[x, y, pressure]` points, pressure interpolated between them.
    Stroke {
        points: Vec<[f32; 3]>,
        /// Where the pen leaves the glass, reported as Android does: one more sample with
        /// pressure 0. Without it the last point is the lift.
        #[serde(default)]
        lift: Option<[f32; 2]>,
        /// A hand resting on the glass around the stroke.
        #[serde(default)]
        palm: Option<Palm>,
        /// A sample at each point and none between: a fast stroke's sparse samples.
        #[serde(default)]
        sparse: bool,
        /// Milliseconds the pen rests at the last point before it lifts (QuickShape).
        #[serde(default)]
        hold_ms: u32,
        /// The tip with a side button held, or the eraser end; the tip by default.
        #[serde(default)]
        pen: Pen,
        /// Degrees the pen leans from upright all along the stroke, 0 to 90; 0 upright.
        #[serde(default)]
        tilt: f32,
        /// Degrees of the direction it leans in, as Android's orientation, -180 to 180.
        #[serde(default)]
        azimuth: f32,
    },
    /// One contact through keys (a place and `ms` from touching down) at `pressure` (default
    /// full): it moves straight between keys in their time and rests where two keys share a
    /// place (holding still), sampled every 4 ms.
    Press {
        path: Vec<Key>,
        #[serde(default = "Tool::finger")]
        tool: Tool,
        #[serde(default = "full_pressure")]
        pressure: f32,
        /// A screenshot of this name just before the contact lifts.
        #[serde(default)]
        before_lift: Option<String>,
        /// The canvas just before the contact lifts must hash to this (a checkpoint named as
        /// `before_lift`, which it needs).
        #[serde(default, deserialize_with = "optional_hash")]
        canvas_before_lift: Option<u64>,
        /// Instead of lifting, the app is interrupted while the contact is down: its window
        /// goes and comes back, or the user leaves the app and comes back; then the platform
        /// cancels the contact, as Android does.
        #[serde(default)]
        interrupted: Option<Interruption>,
    },
    /// The pen hovers just above the glass at `at`, touching nothing; it stays there until the
    /// next contact.
    /// With `pen`, a side button held or the eraser end.
    Hover {
        at: Spot,
        #[serde(default)]
        pen: Pen,
    },
    /// Fingers touch down at `from`, move together straight to `to` (default: stay) over `ms`
    /// (default 64), and lift: two fingers apart or together pinch, turned they rotate, moved
    /// alike they pan; fingers that stay tap.
    Touch {
        from: Vec<Spot>,
        #[serde(default)]
        to: Option<Vec<Spot>>,
        #[serde(default = "default_touch_ms")]
        ms: u32,
    },
    /// The view must be at `zoom` (relative to fit, within 1 %) and `rotation` (degrees,
    /// within half a degree), and with `pan` (points, within one) when given.
    View {
        name: String,
        zoom: f32,
        rotation: f32,
        #[serde(default)]
        pan: Option<[f32; 2]>,
    },
    /// Text committed by the keyboard into the field that has the focus; a last newline then
    /// presses Enter, the keyboard's Done.
    Type(String),
    /// The system's back button.
    Back,
    /// Frames with no input.
    Wait { frames: u32 },
    /// The user puts the tablet down for `minutes`: the app stays in front untouched, or with
    /// `hidden` it is left (Android's `onPause`) and come back to; then one frame. The clock
    /// (the wall clock's too) moves on by that much, no frames between: a Pomodoro's block
    /// counts it down with its sheet in front, and holds while hidden.
    Later {
        minutes: u32,
        #[serde(default)]
        hidden: bool,
    },
    /// The app process ends and starts again with the same screen and the same library, as
    /// after leaving the app and coming back once the system has closed it.
    Restart,
    /// From now on every storage call fails, as on a full or removed disk.
    StorageFails,
    /// Storage works again.
    StorageWorks,
    /// The GPU device is lost as a driver reset loses it: the next frame makes the
    /// renderer again.
    LoseDevice,
    /// The canvas pixels (the artwork without the UI) must hash to `canvas`. The canvas, not
    /// the screen, because only the artwork's pixels are the same on every GPU backend.
    Checkpoint {
        name: String,
        #[serde(deserialize_with = "hex_hash")]
        canvas: u64,
    },
    /// The controls on screen once motion settled: every name of `shown` in reading order
    /// (top to bottom, then left to right; not necessarily next to each other), none of
    /// `absent`. Otherwise the run fails with every name on screen.
    Expect {
        #[serde(default)]
        shown: Vec<String>,
        #[serde(default)]
        absent: Vec<String>,
    },
    /// What the app asked its host so far (`host.rs`): the chimes, the screen kept on.
    Host(HostExpect),
    /// The brush editor's preview strip (its picture's pixels) must hash to `strip`.
    Preview {
        name: String,
        #[serde(deserialize_with = "hex_hash")]
        strip: u64,
    },
    /// The screen, kept for looking at; never compared. Taken once every motion has settled,
    /// or, with `after_ms`, that long after the last input step ended, while it may still move
    /// (the runner's clock is fixed, so it moves alike every run).
    Screenshot {
        name: String,
        #[serde(default)]
        after_ms: Option<u32>,
    },
    /// The user answers the system picker, dialog or share sheet the app asked for.
    System(SystemAnswer),
    /// A lesson started from the home, its sheet open after it (`lesson.rs` for its shapes).
    Lesson(Lesson),
    /// The file the last export wrote, decoded: `scale` times the canvas's size (default 1); at
    /// 1x a PNG must equal the canvas pixel for pixel, above it and as a JPEG come close to it
    /// (reduced by the scale); with `pixels`, its decoded pixels must hash to that.
    Exported {
        name: String,
        #[serde(default = "one")]
        scale: u32,
        #[serde(default, deserialize_with = "optional_hash")]
        pixels: Option<u64>,
    },
    /// The replay's last video as the encoder the runner lent kept it (`video.rs`): frames until
    /// it ended, then the order of its encoder's calls and what the step gives checked, the
    /// frames it kept written as screenshots.
    Video(VideoCheck),
}

fn one() -> u32 {
    1
}

/// What touches the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    Finger,
    Stylus,
}

impl Tool {
    fn finger() -> Self {
        Self::Finger
    }
}

fn default_touch_ms() -> u32 {
    64
}

fn full_pressure() -> f32 {
    1.0
}

/// What interrupts a press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Interruption {
    /// The window goes and comes back (the screen turned off and on): `Runtime::detach`.
    Detach,
    /// The user leaves the app and comes back (home): `Runtime::leave`, Android's `onPause`.
    Leave,
}

/// A hand on the glass at `at` (window pixels), a touch of its own beside the pen.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palm {
    pub at: [f32; 2],
    pub lands: Lands,
}

/// When the hand touches the glass, relative to the pen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lands {
    /// Before the pen touches down; it lifts after the pen.
    Before,
    /// After the pen touched down; it lifts before the pen.
    During,
}
