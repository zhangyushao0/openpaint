//! `InputBatch`: platform input after normalisation, the same on every host. The runtime's
//! normaliser builds it from platform events; the UI routes its contacts; the stroke intent
//! turns pen contacts into strokes.

/// What produced a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    Pen,
    /// The eraser end of a pen.
    Eraser,
    Finger,
    Mouse,
    Unknown,
}

/// What a contact did in its batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactPhase {
    /// Above the screen, not touching (a pen in range).
    Hover,
    /// Left hover range.
    Leave,
    /// Touched down; the samples start the contact.
    Down,
    Move,
    /// Lifted; the samples end the contact.
    Up,
    /// The platform took the contact away; everything since `Down` is void.
    Cancel,
}

/// One normalised sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InputSample {
    /// Screen pixels from the normaliser; canvas pixels once the canvas owns the contact.
    pub position: [f32; 2],
    /// `0..=1`; 0 while hovering.
    pub pressure: f32,
    /// Radians from the screen normal, `0..=pi/2`.
    pub tilt: f32,
    /// Radians, `-pi..=pi`: the direction the pen points.
    pub azimuth: f32,
    /// Platform clock in milliseconds; never decreases within a contact.
    pub time_ms: i64,
}

/// One pointer's part of a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    /// Stable from `Down` to `Up` or `Cancel`.
    pub id: i32,
    pub device: Device,
    pub phase: ContactPhase,
    /// Oldest first; never empty.
    pub samples: Vec<InputSample>,
}

/// Everything one platform input event carried, normalised.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InputBatch {
    pub contacts: Vec<Contact>,
}
