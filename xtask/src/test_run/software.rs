//! The gate on a software GPU (S162): GitHub's runners have no GPU, so CI's gate runs the GPU
//! tests on Mesa's lavapipe, named by `OPENPAINT_SOFTWARE_GPU`. What holds only on a hardware GPU
//! is left out there, each named with why as the gate runs: a drawing held to the bit across two
//! paths (a tier tile and the export, bands and one piece, incremental and cold) or to a hash
//! taken on the dev machine's GPU, which lavapipe's rasterisation does not keep, and frames and
//! walks timed for a hardware GPU. Every local gate runs them all (the dev machine's GPU), the
//! tablet's checks beside it.

use super::Binary;

/// The test binaries left out whole on a software GPU, and why.
const LEFT_OUT: &[(&str, &str)] = &[
    (
        "crates/render/tests/stroke/main.rs",
        "drawings to the bit across paths and golden hashes",
    ),
    (
        "crates/runtime/tests/budget.rs",
        "the caches' bytes as a hardware GPU's drawing leaves them",
    ),
    (
        "crates/render/tests/frame_budget.rs",
        "a frame's background work budgeted in a hardware GPU's time: on lavapipe a tier may get \
         no room in the frames counted",
    ),
];

/// The tests left out of binaries that otherwise run on a software GPU (libtest's `--skip`
/// filters), and why.
const SKIPPED: &[(&str, &str, &str)] = &[
    (
        "crates/render/src/lib.rs",
        "canvas::display_tests::the_ten_bit_canvas",
        "ten-bit reads as a hardware GPU rounds them",
    ),
    (
        "crates/runtime/tests/runtime/main.rs",
        "brushes::",
        "golden waves and incremental equals cold to the bit",
    ),
    (
        "crates/runtime/tests/runtime/main.rs",
        "export::",
        "the export to the bit of what the screen shows",
    ),
    (
        "crates/runtime/tests/runtime/main.rs",
        "pomodoro",
        "a day's sheet's walks within a hang guard timed for a hardware GPU",
    ),
    (
        "crates/render/tests/video.rs",
        "frames_drawn_one_after_another_equal_each_drawn_cold",
        "the video's frames drawn incrementally equal to the bit to each drawn cold",
    ),
    (
        "crates/runtime/tests/runtime/main.rs",
        "replay::the_video_hands_every_frame_in_order_at_its_time_and_finishes",
        "the video's frames to the bit of the replay's moments drawn cold",
    ),
    (
        "crates/runtime/tests/runtime/main.rs",
        "replay::a_segment_ends_on_the_finished_picture",
        "a segment's held end to the bit of the finished picture drawn cold",
    ),
    (
        "crates/runtime/tests/runtime/main.rs",
        "replay::the_replay_shows_instead_and_closing_leaves_the_artwork_as_it_was",
        "the replay's end drawn by appends to the bit of the artwork drawn live",
    ),
];

/// Whether the gate runs on a software GPU (CI's).
pub fn on() -> bool {
    std::env::var_os("OPENPAINT_SOFTWARE_GPU").is_some()
}

/// `binaries` but those left out on a software GPU, each named.
pub(super) fn kept(binaries: Vec<Binary>) -> Vec<Binary> {
    for (source, why) in LEFT_OUT {
        println!("  left out on a software GPU: {source}: {why}");
    }
    for (source, filter, why) in SKIPPED {
        println!("  left out on a software GPU: {source} {filter}: {why}");
    }
    let out = |binary: &Binary| LEFT_OUT.iter().any(|(source, _)| *source == binary.source);
    binaries.into_iter().filter(|binary| !out(binary)).collect()
}

/// The arguments that leave `source`'s skipped tests out on a software GPU; none elsewhere.
pub(super) fn skips(source: &str) -> Vec<&'static str> {
    if !on() {
        return Vec::new();
    }
    (SKIPPED.iter())
        .filter(|(binary, _, _)| *binary == source)
        .flat_map(|(_, filter, _)| ["--skip", *filter])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{LEFT_OUT, SKIPPED};

    #[test]
    fn every_binary_named_is_in_the_workspace() {
        let root = crate::process::root();
        let named = (LEFT_OUT.iter().map(|(source, _)| source))
            .chain(SKIPPED.iter().map(|(source, _, _)| source));
        for source in named {
            assert!(root.join(source).is_file(), "{source} moved or gone");
        }
    }
}
