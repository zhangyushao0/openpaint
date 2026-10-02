package dev.openpaint

import android.app.Activity
import android.os.Build
import android.os.Bundle
import android.view.Choreographer
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.WindowInsets
import android.view.WindowInsetsController

/** Floats per pointer sample: x, y, pressure, tilt, orientation. Must match motion.rs. */
private const val SAMPLE_FIELDS = 5

private val SAMPLE_AXES = intArrayOf(
    MotionEvent.AXIS_X,
    MotionEvent.AXIS_Y,
    MotionEvent.AXIS_PRESSURE,
    MotionEvent.AXIS_TILT,
    MotionEvent.AXIS_ORIENTATION,
)

/**
 * The only activity (DECISIONS D11): owns the lifecycle and a full-screen SurfaceView, paces
 * frames with Choreographer and forwards every MotionEvent to Rust. No product rules here.
 */
class MainActivity : Activity(), SurfaceHolder.Callback2, Choreographer.FrameCallback {
    private var handle = 0L
    private var attached = false
    private var frameRequested = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        handle = Native.create(density(), filesDir.absolutePath)
        val view = SurfaceView(this)
        view.holder.addCallback(this)
        setContentView(view)
        hideSystemBars()
    }

    /** Leaving (home, another app, the screen off): the artwork is stored before anything else. */
    override fun onPause() {
        Native.leave(handle)
        super.onPause()
    }

    override fun onDestroy() {
        Choreographer.getInstance().removeFrameCallback(this)
        frameRequested = false
        Native.destroy(handle)
        handle = 0L
        super.onDestroy()
    }

    override fun surfaceCreated(holder: SurfaceHolder) {}

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        if (attached) {
            Native.resize(handle, width, height, density())
        } else {
            Native.attach(handle, holder.surface, width, height, density())
            attached = true
        }
        requestFrame()
    }

    /** Rotation and resizing wait for this, so the new size never shows a stale frame. */
    override fun surfaceRedrawNeeded(holder: SurfaceHolder) {
        Native.frame(handle, System.nanoTime())
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        Native.detach(handle)
        attached = false
    }

    override fun doFrame(frameTimeNanos: Long) {
        frameRequested = false
        if (Native.frame(handle, System.nanoTime())) requestFrame()
    }

    private fun requestFrame() {
        if (!frameRequested) {
            frameRequested = true
            Choreographer.getInstance().postFrameCallback(this)
        }
    }

    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        forward(event)
        return true
    }

    /** Stylus hover, buttons and mouse events arrive here. */
    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        forward(event)
        return true
    }

    private fun forward(event: MotionEvent) {
        if (handle == 0L) return
        val pointers = event.pointerCount
        val history = event.historySize
        val steps = history + 1
        val ids = IntArray(pointers) { event.getPointerId(it) }
        val tools = IntArray(pointers) { event.getToolType(it) }
        val samples = FloatArray(steps * pointers * SAMPLE_FIELDS)
        val times = LongArray(steps)
        for (step in 0 until steps) {
            val current = step == history
            times[step] = if (current) event.eventTime else event.getHistoricalEventTime(step)
            for (pointer in 0 until pointers) {
                val start = (step * pointers + pointer) * SAMPLE_FIELDS
                SAMPLE_AXES.forEachIndexed { field, axis ->
                    samples[start + field] = if (current) {
                        event.getAxisValue(axis, pointer)
                    } else {
                        event.getHistoricalAxisValue(axis, pointer, step)
                    }
                }
            }
        }
        val wantsFrame = Native.motion(
            handle, event.actionMasked, event.actionIndex, event.flags, event.buttonState,
            ids, tools, samples, times, eventNanos(event),
        )
        if (wantsFrame) requestFrame()
    }

    /** The event's time in System.nanoTime()'s clock; to the millisecond before API 34. */
    private fun eventNanos(event: MotionEvent): Long =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            event.eventTimeNanos
        } else {
            event.eventTime * 1_000_000
        }

    private fun density(): Float = resources.displayMetrics.density

    /** Full screen for the canvas; a swipe from the edge shows the bars briefly. */
    private fun hideSystemBars() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            window.insetsController?.let {
                it.hide(WindowInsets.Type.systemBars())
                it.systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            }
        }
    }
}
