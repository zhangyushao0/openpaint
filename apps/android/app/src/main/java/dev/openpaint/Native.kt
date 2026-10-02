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

    /** Opens the GPU device. Returns 0 when that failed; the reason is in the log. */
    @JvmStatic external fun create(density: Float): Long

    @JvmStatic external fun destroy(handle: Long)

    @JvmStatic external fun attach(handle: Long, surface: Surface, width: Int, height: Int, density: Float)

    @JvmStatic external fun resize(handle: Long, width: Int, height: Int, density: Float)

    /** Must return before the surface is destroyed. */
    @JvmStatic external fun detach(handle: Long)

    /** Draws one frame. Returns true when another frame is wanted right away. */
    @JvmStatic external fun frame(handle: Long): Boolean

    /**
     * One MotionEvent. `samples` holds `timesMs.size` steps (history, then current), each with
     * one block of [SAMPLE_FIELDS] floats per pointer.
     */
    @JvmStatic external fun motion(
        handle: Long,
        action: Int,
        actionIndex: Int,
        buttons: Int,
        ids: IntArray,
        tools: IntArray,
        samples: FloatArray,
        timesMs: LongArray,
    )
}
