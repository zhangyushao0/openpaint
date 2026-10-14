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
     * What the app shows (S52), fields separated by tabs, each "key value": `screen` starting,
     * canvas or gallery; in the gallery `listed` 0 or 1 and `cards` (the artworks' ids, the one
     * changed last first); `fit` (where a canvas fits: left, top, width, height in pixels); a
     * `name` per control in the last frame.
     */
    @JvmStatic external fun state(): String?

    /**
     * The screen's precision check (a soft grey ramp, its upper half at 8 bits, its lower at the
     * window's own) over everything when [on], or off; one frame is drawn at once.
     */
    @JvmStatic external fun gradient(on: Boolean)

    /**
     * The GPU device destroyed as a driver reset loses it (S49); one frame is drawn at once, which
     * makes the renderer again.
     */
    @JvmStatic external fun loseDevice()

    /**
     * The evaluator's micro-benchmark (S55) on this device's GPU: the scene [scene] names, or
     * every one for "all"; its lines, written to the log too, separated by tabs.
     */
    @JvmStatic external fun bench(scene: String): String?
}
