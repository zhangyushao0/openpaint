package dev.openpaint

import android.view.Surface

/**
 * The Rust side (`apps/android/src/jni_entry.rs`). Every call comes from the main thread.
 * A handle of 0 means the runtime could not start; every call then does nothing.
 */
internal object Native {
    init {
        System.loadLibrary("openpaint_android")
    }

    /**
     * Opens the GPU device and starts opening the artwork library in `filesDir` on a worker.
     * Returns 0 when that failed; the reason is in the log.
     */
    @JvmStatic external fun create(density: Float, filesDir: String): Long

    /** Stores what is left to store before the process may be killed. Waits at most 2 s. */
    @JvmStatic external fun leave(handle: Long)

    /** Also finishes a save still running before it returns. */
    @JvmStatic external fun destroy(handle: Long)

    @JvmStatic external fun attach(handle: Long, surface: Surface, width: Int, height: Int, density: Float)

    @JvmStatic external fun resize(handle: Long, width: Int, height: Int, density: Float)

    /** Must return before the surface is destroyed. */
    @JvmStatic external fun detach(handle: Long)

    /**
     * Draws one frame; `startNanos` is `System.nanoTime()` at the call, for the pen latency
     * log. Returns true when another frame is wanted right away.
     */
    @JvmStatic external fun frame(handle: Long, startNanos: Long): Boolean

    /**
     * One MotionEvent. `samples` holds `timesMs.size` steps (history, then current), each with
     * one block of [SAMPLE_FIELDS] floats per pointer; `eventNanos` is the event's time in
     * `System.nanoTime()`'s clock. Returns true when the event needs a frame.
     */
    @JvmStatic external fun motion(
        handle: Long,
        action: Int,
        actionIndex: Int,
        flags: Int,
        buttons: Int,
        ids: IntArray,
        tools: IntArray,
        samples: FloatArray,
        timesMs: LongArray,
        eventNanos: Long,
    ): Boolean

    /** Keyboard text for the focused field: `kind` 0 composing, 1 committed. */
    @JvmStatic external fun text(handle: Long, kind: Int, text: String)

    /** The keyboard deletes characters around the cursor. */
    @JvmStatic external fun deleteSurrounding(handle: Long, before: Int, after: Int)

    /** An editing key: 0 backspace, 1 delete, 2 enter, 3 left, 4 right. */
    @JvmStatic external fun key(handle: Long, code: Int)

    /** True while a text field waits for typing: show the soft keyboard. */
    @JvmStatic external fun keyboard(handle: Long): Boolean

    /** The back button; true when the app used it, false for the system's own back. */
    @JvmStatic external fun back(handle: Long): Boolean
}
