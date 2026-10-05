package dev.openpaint

/**
 * Debug builds only: the Rust side's control map (`apps/android/src/jni_debug.rs`), for
 * [GestureInstrumentation]'s `-e tap <name>`. Call on the main thread, like [Native].
 */
internal object DebugControls {
    init {
        System.loadLibrary("openpaint_android")
    }

    /**
     * "x,y", the middle of the control called [name] (or "name #n") in the last frame in screen
     * pixels; otherwise why not, a tab and every name on screen, one per line.
     */
    @JvmStatic external fun find(name: String): String?

    /**
     * The screen's precision check (a soft grey ramp, its upper half at 8 bits, its lower at the
     * window's own) over everything when [on], or off; one frame is drawn at once.
     */
    @JvmStatic external fun gradient(on: Boolean)
}
