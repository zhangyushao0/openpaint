package dev.openpaint

import android.app.Activity
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.view.Choreographer
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.SurfaceHolder
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.view.inputmethod.InputMethodManager

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
    private lateinit var view: CanvasView
    private val pickers = Pickers(this) { handle }
    /** The app used the back button's press; its release is the app's too. */
    private var backUsed = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        handle = Native.create(density(), filesDir.absolutePath, cacheDir.absolutePath)
        view = CanvasView(this)
        view.handle = handle
        view.onInput = { requestFrame() }
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
        syncKeyboard()
        if (handle != 0L) pickers.serve()
    }

    /** A system picker closed; what it gave goes to Rust, which starts the work at once. */
    @Deprecated("Activity results without AndroidX")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (handle != 0L && pickers.result(requestCode, resultCode, data)) {
            requestFrame()
            return
        }
        super.onActivityResult(requestCode, resultCode, data)
    }

    /** Shows the soft keyboard while a text field waits for typing, hides it after. */
    private fun syncKeyboard() {
        val wanted = Native.keyboard(handle)
        if (wanted == view.editing) return
        view.editing = wanted
        val keyboard = getSystemService(InputMethodManager::class.java) ?: return
        if (wanted) {
            view.requestFocus()
            keyboard.restartInput(view)
            keyboard.showSoftInput(view, 0)
        } else {
            keyboard.hideSoftInputFromWindow(view.windowToken, 0)
            keyboard.restartInput(view)
        }
    }

    /**
     * Back goes to the app first (canvas to gallery, closing a dialog); when it does not use
     * it, the system's back applies. While a text field waits for typing, keys type into it.
     */
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (event.keyCode == KeyEvent.KEYCODE_BACK) {
            if (event.action == KeyEvent.ACTION_DOWN && event.repeatCount == 0) {
                backUsed = Native.back(handle)
                if (backUsed) requestFrame()
            }
            val used = backUsed
            if (event.action == KeyEvent.ACTION_UP) backUsed = false
            return used || super.dispatchKeyEvent(event)
        }
        if (view.editing && type(event)) {
            requestFrame()
            return true
        }
        return super.dispatchKeyEvent(event)
    }

    /** Sends a key event to the focused text field; false when it is not a typing key. */
    private fun type(event: KeyEvent): Boolean {
        if (event.action == KeyEvent.ACTION_MULTIPLE && event.keyCode == KeyEvent.KEYCODE_UNKNOWN) {
            event.characters?.let { Native.text(handle, 1, it) }
            return true
        }
        val code = when (event.keyCode) {
            KeyEvent.KEYCODE_DEL -> KEY_BACKSPACE
            KeyEvent.KEYCODE_FORWARD_DEL -> KEY_DELETE
            KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> KEY_ENTER
            KeyEvent.KEYCODE_DPAD_LEFT -> KEY_LEFT
            KeyEvent.KEYCODE_DPAD_RIGHT -> KEY_RIGHT
            else -> null
        }
        val character = event.unicodeChar
        if (code == null && (character == 0 || event.isCtrlPressed)) return false
        if (event.action == KeyEvent.ACTION_DOWN) {
            if (code != null) {
                Native.key(handle, code)
            } else {
                Native.text(handle, 1, String(Character.toChars(character)))
            }
        }
        return true
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
