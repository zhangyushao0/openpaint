//! The intents a pass collects from taps on the chrome. egui may run a pass twice: one tap, or
//! one value of a touch, is sent once.

use openpaint_app::{BrushIntent, Intent};

/// Pushes `intent` unless it is the one pushed last.
pub(crate) fn push_once(taps: &mut Vec<Intent>, intent: Intent) {
    if taps.last() != Some(&intent) {
        taps.push(intent);
    }
}

/// As `push_once`, for the toolbox's brush.
pub(crate) fn push_brush(taps: &mut Vec<Intent>, intent: BrushIntent) {
    push_once(taps, Intent::Brush(intent));
}
