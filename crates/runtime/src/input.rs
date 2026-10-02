//! Platform input as typed batches: one batch per platform motion event, each pointer with its
//! batched history. Hosts translate native events into these; the normaliser (S4) turns them
//! into stroke input and egui events.

use std::fmt;

/// What produced a pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    Unknown,
    Finger,
    Stylus,
    /// The eraser end of a stylus.
    Eraser,
    Mouse,
}

/// What happened in a batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionPhase {
    /// The first pointer touched down.
    Down,
    Move,
    /// The last pointer lifted.
    Up,
    /// The platform took the gesture away; everything since `Down` is void.
    Cancel,
    /// Another pointer touched down; `index` is its position in `MotionBatch::pointers`.
    PointerDown {
        index: usize,
    },
    /// A pointer lifted while others stay down.
    PointerUp {
        index: usize,
    },
    /// The platform took one pointer away while others stay down (Android: `ACTION_POINTER_UP`
    /// with `FLAG_CANCELED`, a touch judged to be a palm); everything it did is void.
    PointerCancel {
        index: usize,
    },
    HoverEnter,
    HoverMove,
    HoverExit,
    ButtonPress,
    ButtonRelease,
}

/// Buttons held during a batch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Buttons {
    pub primary: bool,
    pub secondary: bool,
    pub tertiary: bool,
    pub stylus_primary: bool,
    pub stylus_secondary: bool,
}

/// One sample of one pointer, in window pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerSample {
    pub position: [f32; 2],
    /// Normally 0 to 1; some devices report slightly above 1. Hover samples report 0.
    pub pressure: f32,
    /// Radians from the screen normal: 0 is upright, pi/2 lies flat.
    pub tilt: f32,
    /// Radians: the direction a stylus points, or a touch ellipse's major axis.
    pub orientation: f32,
    /// Platform event time in milliseconds.
    pub time_ms: i64,
}

/// One pointer of a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct Pointer {
    /// Stable while the pointer stays down.
    pub id: i32,
    pub tool: ToolKind,
    /// Oldest first; the last sample is the event's current one. Hosts never send it empty.
    pub samples: Vec<PointerSample>,
}

/// Everything one platform motion event carried.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionBatch {
    pub phase: MotionPhase,
    pub buttons: Buttons,
    pub pointers: Vec<Pointer>,
}

/// One log line: phase, buttons, then each pointer's current sample and its sample count.
impl fmt::Display for MotionBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} buttons={}", self.phase, self.buttons)?;
        for pointer in &self.pointers {
            write!(f, " | {pointer}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{} {:?}", self.id, self.tool)?;
        if let Some(last) = self.samples.last() {
            let [x, y] = last.position;
            write!(
                f,
                " x={x:.1} y={y:.1} p={:.3} tilt={:.2} orient={:.2} t={}",
                last.pressure, last.tilt, last.orientation, last.time_ms
            )?;
        }
        write!(f, " samples={}", self.samples.len())
    }
}

impl fmt::Display for Buttons {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = [
            (self.primary, "primary"),
            (self.secondary, "secondary"),
            (self.tertiary, "tertiary"),
            (self.stylus_primary, "stylus_primary"),
            (self.stylus_secondary, "stylus_secondary"),
        ];
        let held: Vec<&str> = names
            .iter()
            .filter(|(down, _)| *down)
            .map(|(_, name)| *name)
            .collect();
        if held.is_empty() {
            f.write_str("-")
        } else {
            f.write_str(&held.join("+"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(x: f32, pressure: f32, time_ms: i64) -> PointerSample {
        PointerSample {
            position: [x, 20.0],
            pressure,
            tilt: 0.5,
            orientation: -1.25,
            time_ms,
        }
    }

    #[test]
    fn log_line_shows_the_current_sample_and_history_length() {
        let batch = MotionBatch {
            phase: MotionPhase::Move,
            buttons: Buttons {
                stylus_primary: true,
                ..Buttons::default()
            },
            pointers: vec![Pointer {
                id: 0,
                tool: ToolKind::Stylus,
                samples: vec![sample(10.0, 0.25, 100), sample(12.5, 0.5, 104)],
            }],
        };
        assert_eq!(
            batch.to_string(),
            "Move buttons=stylus_primary | #0 Stylus x=12.5 y=20.0 p=0.500 tilt=0.50 orient=-1.25 t=104 samples=2"
        );
    }

    #[test]
    fn log_line_lists_every_pointer_and_marks_no_buttons() {
        let finger = |id| Pointer {
            id,
            tool: ToolKind::Finger,
            samples: vec![sample(1.0, 1.0, 7)],
        };
        let batch = MotionBatch {
            phase: MotionPhase::PointerDown { index: 1 },
            buttons: Buttons::default(),
            pointers: vec![finger(0), finger(3)],
        };
        let line = batch.to_string();
        assert!(line.starts_with("PointerDown { index: 1 } buttons=- | #0 Finger"));
        assert!(line.contains(" | #3 Finger"));
    }
}
