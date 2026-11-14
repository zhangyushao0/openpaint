//! What the Kotlin host does after a frame, wherever the frame was drawn: the app draws nothing at
//! rest, so a frame whose duties are skipped (the next frame it wants, the Pomodoro's wake for the
//! state it left, the screen kept on, the panel's rate) can leave a counting block or break with
//! no frame ever again (seen on the tablet, 1.23.0: a break that never counted down). Read from
//! the sources, as `debug_only.rs` reads them.

use crate::debug_only::{source_set, texts};

/// The main source set's `MainActivity.kt`.
fn activity() -> String {
    let main = texts(&source_set("main"));
    let (_, text) = (main.into_iter())
        .find(|(path, _)| path.ends_with("MainActivity.kt"))
        .expect("MainActivity.kt");
    text
}

/// The body of the function declared by `head` in `text`: up to the next line that closes a
/// function at its indent.
fn body<'a>(text: &'a str, head: &str) -> &'a str {
    let start = text.find(head).unwrap_or_else(|| panic!("{head}"));
    let rest = &text[start..];
    let end = rest.find("\n    }\n").map_or(rest.len(), |end| end + 6);
    &rest[..end]
}

#[test]
fn every_frame_the_host_draws_is_followed_by_what_the_host_does_after_a_frame() {
    let text = activity();
    let calls: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("Native.frame("))
        .collect();
    assert!(calls.len() >= 2, "doFrame and surfaceRedrawNeeded draw");
    for line in &calls {
        assert!(
            line.contains("drawn(Native.frame("),
            "drawn without the host's after: {line}"
        );
    }
    let drawn = body(&text, "private fun drawn(again: Boolean)");
    for duty in ["requestFrame()", "timer.after(handle)", "rate.drawn(again)"] {
        assert!(drawn.contains(duty), "after a frame: {duty}");
    }
    // Back in front with the surface kept: leaving held the timer, a frame shows it now.
    let resumed = body(&text, "override fun onResume()");
    assert!(resumed.contains("requestFrame()"), "{resumed}");
}

#[test]
fn the_debug_chores_that_draw_frames_of_their_own_ask_the_activity_for_one_after() {
    let debug = texts(&source_set("debug"));
    let (_, text) = (debug.iter())
        .find(|(path, _)| path.ends_with("GestureInstrumentation.kt"))
        .expect("GestureInstrumentation.kt");
    for chore in [
        "DebugControls.gradient(",
        "DebugControls.loseDevice(",
        "DebugControls.bench(",
        "DebugControls.practiceTest(",
        "DebugControls.openFile(",
    ] {
        let at = text.find(chore).unwrap_or_else(|| panic!("{chore}"));
        let after = &text[at..];
        let next = after.find("framed()").unwrap_or(usize::MAX);
        let answered = after.find("putString(").unwrap_or(usize::MAX);
        assert!(
            next < answered,
            "{chore} answers before a frame is asked for"
        );
    }
}
