//! Decodes the arrays the Kotlin host packs from one `MotionEvent` into a typed `MotionBatch`.
//! Pure Rust, so it is tested on the development machine; the JNI layer only copies arrays.

use std::fmt;

use openpaint_runtime::{Buttons, MotionBatch, MotionPhase, Pointer, PointerSample, ToolKind};

/// Floats per pointer sample, in this order: x, y, pressure, tilt, orientation.
/// Must match `SAMPLE_FIELDS` in `MainActivity.kt`.
pub const SAMPLE_FIELDS: usize = 5;

/// One `MotionEvent` as the host packs it. Steps are the batched history followed by the
/// current sample; each step holds every pointer.
pub struct RawMotion<'a> {
    /// `MotionEvent.getActionMasked()`.
    pub action: i32,
    /// `MotionEvent.getActionIndex()`.
    pub action_index: i32,
    /// `MotionEvent.getButtonState()`.
    pub buttons: i32,
    /// `getPointerId(p)` per pointer.
    pub ids: &'a [i32],
    /// `getToolType(p)` per pointer.
    pub tools: &'a [i32],
    /// `(step * pointers + pointer) * SAMPLE_FIELDS` is where a sample starts.
    pub samples: &'a [f32],
    /// Event time in milliseconds per step.
    pub times_ms: &'a [i64],
}

/// The host packed arrays whose shapes do not fit together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(&'static str);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "malformed motion event from the host: {}", self.0)
    }
}

impl std::error::Error for DecodeError {}

/// The typed batch, or `None` for actions the product never uses (outside touches, scroll).
pub fn decode(raw: &RawMotion<'_>) -> Result<Option<MotionBatch>, DecodeError> {
    let pointers = raw.ids.len();
    let steps = raw.times_ms.len();
    if pointers == 0 || steps == 0 {
        return Err(DecodeError("no pointers or no samples"));
    }
    if raw.tools.len() != pointers {
        return Err(DecodeError("tool types do not match the pointers"));
    }
    if raw.samples.len() != steps * pointers * SAMPLE_FIELDS {
        return Err(DecodeError("sample count does not match steps x pointers"));
    }
    let Some(phase) = phase(raw.action, raw.action_index)? else {
        return Ok(None);
    };
    if let MotionPhase::PointerDown { index } | MotionPhase::PointerUp { index } = phase
        && index >= pointers
    {
        return Err(DecodeError("action index is not a pointer of the event"));
    }
    let pointers = (0..pointers)
        .map(|pointer| Pointer {
            id: raw.ids[pointer],
            tool: tool(raw.tools[pointer]),
            samples: (0..steps).map(|step| sample(raw, step, pointer)).collect(),
        })
        .collect();
    Ok(Some(MotionBatch {
        phase,
        buttons: buttons(raw.buttons),
        pointers,
    }))
}

fn sample(raw: &RawMotion<'_>, step: usize, pointer: usize) -> PointerSample {
    let start = (step * raw.ids.len() + pointer) * SAMPLE_FIELDS;
    let [x, y, pressure, tilt, orientation] =
        [0, 1, 2, 3, 4].map(|field| raw.samples[start + field]);
    PointerSample {
        position: [x, y],
        pressure,
        tilt,
        orientation,
        time_ms: raw.times_ms[step],
    }
}

/// `MotionEvent.ACTION_*` values.
fn phase(action: i32, action_index: i32) -> Result<Option<MotionPhase>, DecodeError> {
    let index = || usize::try_from(action_index).map_err(|_| DecodeError("negative action index"));
    Ok(Some(match action {
        0 => MotionPhase::Down,
        1 => MotionPhase::Up,
        2 => MotionPhase::Move,
        3 => MotionPhase::Cancel,
        5 => MotionPhase::PointerDown { index: index()? },
        6 => MotionPhase::PointerUp { index: index()? },
        7 => MotionPhase::HoverMove,
        9 => MotionPhase::HoverEnter,
        10 => MotionPhase::HoverExit,
        11 => MotionPhase::ButtonPress,
        12 => MotionPhase::ButtonRelease,
        // ACTION_OUTSIDE (4), ACTION_SCROLL (8) and anything newer.
        _ => return Ok(None),
    }))
}

/// `MotionEvent.TOOL_TYPE_*` values.
fn tool(tool_type: i32) -> ToolKind {
    match tool_type {
        1 => ToolKind::Finger,
        2 => ToolKind::Stylus,
        3 => ToolKind::Mouse,
        4 => ToolKind::Eraser,
        _ => ToolKind::Unknown,
    }
}

/// `MotionEvent.BUTTON_*` bits.
fn buttons(state: i32) -> Buttons {
    let held = |bit: i32| state & bit != 0;
    Buttons {
        primary: held(0x01),
        secondary: held(0x02),
        tertiary: held(0x04),
        stylus_primary: held(0x20),
        stylus_secondary: held(0x40),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two pointers (stylus id 0, finger id 7), two steps: one history sample and the current.
    fn raw<'a>(action: i32, action_index: i32, samples: &'a [f32]) -> RawMotion<'a> {
        RawMotion {
            action,
            action_index,
            buttons: 0x20,
            ids: &[0, 7],
            tools: &[2, 1],
            samples,
            times_ms: &[100, 104],
        }
    }

    const SAMPLES: [f32; 20] = [
        1.0, 2.0, 0.1, 0.2, 0.3, // step 0, stylus
        5.0, 6.0, 1.0, 0.0, 0.0, // step 0, finger
        3.0, 4.0, 0.5, 0.6, 0.7, // step 1, stylus
        5.0, 6.5, 1.0, 0.0, 0.0, // step 1, finger
    ];

    #[test]
    fn decodes_every_pointer_with_its_history_in_order() {
        let batch = decode(&raw(2, 0, &SAMPLES)).unwrap().unwrap();
        assert_eq!(batch.phase, MotionPhase::Move);
        assert!(batch.buttons.stylus_primary && !batch.buttons.primary);
        let stylus = &batch.pointers[0];
        assert_eq!((stylus.id, stylus.tool), (0, ToolKind::Stylus));
        assert_eq!(
            stylus.samples[1],
            PointerSample {
                position: [3.0, 4.0],
                pressure: 0.5,
                tilt: 0.6,
                orientation: 0.7,
                time_ms: 104,
            }
        );
        assert_eq!(stylus.samples[0].position, [1.0, 2.0]);
        let finger = &batch.pointers[1];
        assert_eq!((finger.id, finger.tool), (7, ToolKind::Finger));
        assert_eq!(finger.samples[1].position, [5.0, 6.5]);
    }

    #[test]
    fn maps_hover_and_secondary_pointer_actions() {
        let hover = decode(&raw(7, 0, &SAMPLES)).unwrap().unwrap();
        assert_eq!(hover.phase, MotionPhase::HoverMove);
        let down = decode(&raw(5, 1, &SAMPLES)).unwrap().unwrap();
        assert_eq!(down.phase, MotionPhase::PointerDown { index: 1 });
    }

    #[test]
    fn skips_actions_the_product_never_uses() {
        assert_eq!(decode(&raw(8, 0, &SAMPLES)), Ok(None));
        assert_eq!(decode(&raw(4, 0, &SAMPLES)), Ok(None));
    }

    #[test]
    fn rejects_shapes_that_do_not_fit() {
        assert!(decode(&raw(2, 0, &SAMPLES[..15])).is_err());
        assert!(decode(&raw(5, 2, &SAMPLES)).is_err());
        assert!(decode(&raw(6, -1, &SAMPLES)).is_err());
        let mut empty = raw(2, 0, &[]);
        empty.times_ms = &[];
        assert!(decode(&empty).is_err());
    }
}
