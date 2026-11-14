package dev.openpaint

import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Surface
import android.view.SurfaceView

/** How long after the last frame of a stretch the panel's rate is let go. */
private const val LET_GO_MS = 1_000L

/**
 * The panel's refresh rate while the app draws frames: its highest at the current resolution
 * asked for on the drawing surface as a stretch of frames starts (`Surface.setFrameRate`, API 30,
 * compatibility default, a change only where seamless), no preference again [LET_GO_MS] after a
 * frame that wanted no other. The app draws nothing at rest, so a still picture never holds the
 * panel up. Without this the panel falls to 60 Hz seconds after the last touch, also while
 * something moves with no finger on the glass. No product rules here.
 */
internal class FrameRate(private val view: SurfaceView) {
    private val main = Handler(Looper.getMainLooper())
    private val letGo = Runnable { ask(false) }
    private var raised = false

    /** A frame is asked for: the stretch goes on, at the panel's highest rate. */
    fun drawing() {
        main.removeCallbacks(letGo)
        ask(true)
    }

    /** A frame drawn; [again] when the next is wanted at once, else the rate let go shortly. */
    fun drawn(again: Boolean) {
        main.removeCallbacks(letGo)
        if (again) ask(true) else if (raised) main.postDelayed(letGo, LET_GO_MS)
    }

    /** The surface went: its vote with it. */
    fun gone() {
        main.removeCallbacks(letGo)
        raised = false
    }

    private fun ask(high: Boolean) {
        if (high == raised || Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return
        val surface = view.holder.surface
        if (surface == null || !surface.isValid) return
        val rate = if (high) highest() else 0f
        surface.setFrameRate(rate, Surface.FRAME_RATE_COMPATIBILITY_DEFAULT)
        raised = high
        Log.i("openpaint", if (high) "frame rate asked: $rate Hz" else "frame rate let go")
    }

    /** The highest refresh rate of the display's modes at its current resolution; 0 unknown. */
    private fun highest(): Float {
        val display = view.display ?: return 0f
        val current = display.mode
        return display.supportedModes
            .filter { it.physicalWidth == current.physicalWidth && it.physicalHeight == current.physicalHeight }
            .maxOfOrNull { it.refreshRate } ?: 0f
    }
}
