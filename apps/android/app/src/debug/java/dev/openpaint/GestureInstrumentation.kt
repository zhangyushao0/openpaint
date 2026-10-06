package dev.openpaint

import android.app.Activity
import android.app.Instrumentation
import android.os.Bundle
import android.os.SystemClock
import android.view.InputDevice
import android.view.MotionEvent

/**
 * Debug builds only: injects one touch gesture through the system's input pipeline, as the
 * dev machine's device checks need multi-finger gestures that `adb shell input` cannot make
 * (the approach of the previous project). Run with
 * `am instrument -w --no-restart -e from x,y[,x,y...] [-e to ...] [-e ms N] [-e tool stylus]
 * dev.openpaint/.GestureInstrumentation`: the fingers (or the pen) touch down one after another
 * at `from`, move straight to `to` over `ms` in 16 ms frames, and lift one after another.
 * One pointer can instead follow `-e path x,y,x,y...`, one point per 8 ms event, and the pen
 * presses with `-e pressure P` (default 0.6). `-e hover x,y` holds the pen just above the glass
 * there instead for `-e ms` (default 1500), then takes it away (hover enter, move, exit).
 * `-e tool eraser` is the pen's eraser end and `-e button primary|secondary` holds a side
 * button through the pen's events (S41). `-e tap <name>` is a finger tap on the middle of the
 * control the app last drew under that name ([DebugControls]); the result's `tapped` says
 * where. `-e gradient on|off` injects nothing: it turns the screen's precision check on or off
 * ([DebugControls.gradient]).
 */
class GestureInstrumentation : Instrumentation() {
    private var arguments = Bundle()

    override fun onCreate(arguments: Bundle?) {
        super.onCreate(arguments)
        this.arguments = arguments ?: Bundle()
        start()
    }

    override fun onStart() {
        val result = Bundle()
        try {
            arguments.getString("gradient")?.let { state ->
                runOnMainSync { DebugControls.gradient(state == "on") }
                result.putString("gesture", "injected")
                finish(Activity.RESULT_OK, result)
                return
            }
            arguments.getString("tap")?.let { name ->
                val at = control(name)
                arguments.putString("from", "${at[0]},${at[1]}")
                result.putString("tapped", "$name at ${at[0]},${at[1]}")
            }
            inject()
            result.putString("gesture", "injected")
            finish(Activity.RESULT_OK, result)
        } catch (error: Throwable) {
            result.putString("gesture", "failed: $error")
            finish(Activity.RESULT_CANCELED, result)
        }
    }

    private fun points(name: String): List<FloatArray>? = arguments.getString(name)
        ?.split(',')
        ?.map { it.trim().toFloat() }
        ?.chunked(2)
        ?.map { floatArrayOf(it[0], it[1]) }

    /** The middle of the control called [name], asked on the main thread as every JNI call is. */
    private fun control(name: String): FloatArray {
        var found: String? = null
        runOnMainSync { found = DebugControls.find(name) }
        val answer = found ?: error("no answer for $name")
        val point = answer.split(',').mapNotNull { it.toFloatOrNull() }
        require(point.size == 2) { "$name: $answer" }
        return floatArrayOf(point[0], point[1])
    }

    private val pressure: Float
        get() = arguments.getString("pressure")?.toFloat() ?: 0.6f

    /** The pen (`-e tool stylus`) or its eraser end (`-e tool eraser`) rather than fingers. */
    private val pen: Boolean
        get() = arguments.getString("tool") in setOf("stylus", "eraser")

    /** The pen's tool type: its eraser end with `-e tool eraser`, else its tip. */
    private val penTool: Int
        get() = if (arguments.getString("tool") == "eraser") {
            MotionEvent.TOOL_TYPE_ERASER
        } else {
            MotionEvent.TOOL_TYPE_STYLUS
        }

    /** The pen's side button held through the gesture (`-e button primary|secondary`, S41). */
    private val buttons: Int
        get() = when (arguments.getString("button")) {
            "primary" -> MotionEvent.BUTTON_STYLUS_PRIMARY
            "secondary" -> MotionEvent.BUTTON_STYLUS_SECONDARY
            else -> 0
        }

    private fun inject() {
        points("hover")?.let { hover ->
            val time = SystemClock.uptimeMillis()
            send(time, MotionEvent.ACTION_HOVER_ENTER, hover.take(1), true, 0f)
            SystemClock.sleep(8)
            send(time, MotionEvent.ACTION_HOVER_MOVE, hover.take(1), true, 0f)
            SystemClock.sleep(arguments.getString("ms")?.toLong() ?: 1500L)
            send(time, MotionEvent.ACTION_HOVER_EXIT, hover.take(1), true, 0f)
            return
        }
        points("path")?.let { path ->
            follow(path)
            return
        }
        val from = points("from") ?: error("no from")
        val to = points("to") ?: from
        require(to.size == from.size) { "from and to differ in fingers" }
        val stylus = pen
        val frames = maxOf(1, (arguments.getString("ms")?.toInt() ?: 64) / 16)
        val at = { frame: Int ->
            val t = frame.toFloat() / frames
            from.zip(to) { a, b -> floatArrayOf(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t) }
        }
        val down = SystemClock.uptimeMillis()
        for (count in 1..from.size) {
            val action = if (count == 1) MotionEvent.ACTION_DOWN else pointer(MotionEvent.ACTION_POINTER_DOWN, count - 1)
            send(down, action, at(0).take(count), stylus)
        }
        for (frame in 1 until frames) {
            SystemClock.sleep(16)
            send(down, MotionEvent.ACTION_MOVE, at(frame), stylus)
        }
        SystemClock.sleep(16)
        for (count in from.size downTo 1) {
            val action = if (count == 1) MotionEvent.ACTION_UP else pointer(MotionEvent.ACTION_POINTER_UP, count - 1)
            send(down, action, at(frames).take(count), stylus)
        }
    }

    /** One pointer down at the path's first point, a move per later point, up at its last. */
    private fun follow(path: List<FloatArray>) {
        val stylus = pen
        val down = SystemClock.uptimeMillis()
        send(down, MotionEvent.ACTION_DOWN, path.take(1), stylus)
        for (point in path.drop(1)) {
            SystemClock.sleep(8)
            send(down, MotionEvent.ACTION_MOVE, listOf(point), stylus)
        }
        SystemClock.sleep(8)
        send(down, MotionEvent.ACTION_UP, path.takeLast(1), stylus)
    }

    private fun pointer(action: Int, index: Int) = action or (index shl MotionEvent.ACTION_POINTER_INDEX_SHIFT)

    private fun send(
        downTime: Long,
        action: Int,
        points: List<FloatArray>,
        stylus: Boolean,
        penPressure: Float = pressure,
    ) {
        val source = if (stylus) InputDevice.SOURCE_STYLUS else InputDevice.SOURCE_TOUCHSCREEN
        val device = InputDevice.getDeviceIds().firstOrNull { id ->
            InputDevice.getDevice(id)?.supportsSource(source) == true
        } ?: error("no input device for source $source")
        val properties = points.indices.map { index ->
            MotionEvent.PointerProperties().apply {
                id = index
                toolType = if (stylus) penTool else MotionEvent.TOOL_TYPE_FINGER
            }
        }.toTypedArray()
        val coords = points.map { point ->
            MotionEvent.PointerCoords().apply {
                x = point[0]
                y = point[1]
                pressure = if (stylus) penPressure else 1.0f
                size = 0.08f
            }
        }.toTypedArray()
        val event = MotionEvent.obtain(
            downTime, SystemClock.uptimeMillis(), action, points.size, properties, coords,
            0, if (stylus) buttons else 0, 1f, 1f, device, 0, source, 0,
        )
        try {
            check(uiAutomation.injectInputEvent(event, true)) { "the input dispatcher refused the event" }
        } finally {
            event.recycle()
        }
    }
}
