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
     * What the app shows, fields separated by tabs, each "key value": `screen` starting,
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
     * The GPU device destroyed as a driver reset loses it; one frame is drawn at once, which
     * makes the renderer again.
     */
    @JvmStatic external fun loseDevice()

    /**
     * One of the app's scenario benchmarks (open-cold, open, zoom, pan, strokes, session,
     * export, touch, undo, adjust, ui) on the artwork open, as [scene] names it. Its lines,
     * written to the log too, separated by tabs.
     */
    @JvmStatic external fun bench(scene: String): String?

    /**
     * An OpenPaint file's [bytes] opened as the gallery's Open OpenPaint file… opens a chosen
     * one, no picker shown: "opening", or why not.
     */
    @JvmStatic external fun openFile(bytes: ByteArray): String?

    /**
     * A test practice session beside the author's: [command] `start` starts one from the gallery
     * ("started <name>" once its sheet shows), `end:<name>` removes that one, its record and
     * sheet ("removed <name>", or "no such session"). Otherwise why not.
     */
    @JvmStatic external fun practiceTest(command: String): String?

    /**
     * Every practice record and practice sheet removed from the library in [filesDir] by the
     * store's own code, each removed sheet's display cache in [cacheDir] with it, while no
     * activity holds the library: "cleared" and what went and stayed by count, tab-separated, or
     * why not.
     */
    @JvmStatic external fun practiceClear(filesDir: String, cacheDir: String): String?

    /**
     * Every runtime made from now on records its frames in real use, a line a stretch of motion
     * in `files/frames.txt` ([DebugApplication], before any activity).
     */
    @JvmStatic external fun recordFrames()
}
