package dev.openpaint

import android.app.Activity
import android.app.Instrumentation
import android.os.Bundle
import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent

/**
 * Debug builds only: injects one touch gesture through the system's input pipeline, as the
 * dev machine's device checks need multi-finger gestures that `adb shell input` cannot make
 * (the approach of the previous project). Run with
 * `am instrument -w --no-restart -e from x,y[,x,y...] [-e to ...] [-e ms N] [-e tool stylus]
 * dev.openpaint/.GestureInstrumentation`: the fingers (or the pen) touch down one after another
 * at `from`, move straight to `to` over `ms` in 16 ms frames, and lift one after another.
 * One pointer can instead follow `-e path x,y,x,y...`, one point per 8 ms event, and the pen
 * presses with `-e pressure P` (default 0.6), leaning `-e tilt D` degrees from upright (default
 * 0, S44); `-e repeat N` draws the path N times, each turned and moved ([turned], S48);
 * `-e away sleep|home` turns the screen off or presses home while the pen is still down at the
 * path's end ([followAway], S50).
 * `-e hover x,y` holds the pen just above the glass
 * there instead for `-e ms` (default 1500), then takes it away (hover enter, move, exit).
 * `-e tool eraser` is the pen's eraser end and `-e button primary|secondary|tertiary` holds a
 * side button through the pen's events (S41, S43). `-e tap <name>` is a finger tap on the
 * middle of the control the app last drew under that name ([DebugControls]); the result's
 * `tapped` says where. `-e drag <name> -e to x,y` is a finger from that control's middle to
 * `to` (S52). `-e within N` waits up to N ms for the control to show. `-e find <name>` injects
 * nothing: the result's `found` says where it lies; `-e state 1` answers what the app shows
 * ([DebugControls.state], S52). `-e gradient on|off` injects nothing: it turns the screen's
 * precision check on or off ([DebugControls.gradient]); `-e lose device` loses the GPU device on
 * purpose ([DebugControls.loseDevice], S49). A question is answered with `gesture=answered`.
 */
class GestureInstrumentation : Instrumentation() {
    private var arguments = Bundle()
    /** The input device injecting each source's events. */
    private val devices = mutableMapOf<Int, Int>()

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
            arguments.getString("lose")?.let {
                runOnMainSync { DebugControls.loseDevice() }
                result.putString("gesture", "injected")
                finish(Activity.RESULT_OK, result)
                return
            }
            if (answer(result)) {
                result.putString("gesture", "answered")
                finish(Activity.RESULT_OK, result)
                return
            }
            arguments.getString("tap")?.let { name ->
                val at = control(name)
                arguments.putString("from", "${at[0]},${at[1]}")
                result.putString("tapped", "$name at ${at[0]},${at[1]}")
            }
            arguments.getString("drag")?.let { name ->
                val at = control(name)
                arguments.putString("from", "${at[0]},${at[1]}")
                result.putString("dragged", "$name from ${at[0]},${at[1]}")
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

    /**
     * A question instead of a gesture (S52), its answer put in [result]: `-e find <name>` (where
     * the control lies) or `-e state 1` (what the app shows). False when there is none.
     */
    private fun answer(result: Bundle): Boolean {
        arguments.getString("find")?.let { name ->
            val at = control(name)
            result.putString("found", "$name at ${at[0]},${at[1]}")
            return true
        }
        if (arguments.getString("state") == null) return false
        var state: String? = null
        runOnMainSync { state = DebugControls.state() }
        result.putString("state", state ?: error("no answer for the state"))
        return true
    }

    /**
     * The middle of the control called [name], asked on the main thread as every JNI call is;
     * with `-e within N`, asked again every 50 ms until it shows, at most N ms.
     */
    private fun control(name: String): FloatArray {
        val until = SystemClock.uptimeMillis() + (arguments.getString("within")?.toLong() ?: 0L)
        while (true) {
            var found: String? = null
            runOnMainSync { found = DebugControls.find(name) }
            val answer = found ?: error("no answer for $name")
            val point = answer.split(',').mapNotNull { it.toFloatOrNull() }
            if (point.size == 2) return floatArrayOf(point[0], point[1])
            require(SystemClock.uptimeMillis() < until) { "$name: $answer" }
            SystemClock.sleep(50)
        }
    }

    private val pressure: Float
        get() = arguments.getString("pressure")?.toFloat() ?: 0.6f

    /** The pen's lean from upright in radians, as Android's `AXIS_TILT` (`-e tilt` degrees). */
    private val tilt: Float
        get() = Math.toRadians(arguments.getString("tilt")?.toDouble() ?: 0.0).toFloat()

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

    /**
     * The pen's side button held through the gesture (`-e button primary|secondary|tertiary`,
     * S41, S43): from the tip outwards, as the author's Wacom pen reports its three buttons.
     */
    private val buttons: Int
        get() = when (arguments.getString("button")) {
            "primary" -> MotionEvent.BUTTON_STYLUS_PRIMARY
            "secondary" -> MotionEvent.BUTTON_STYLUS_SECONDARY
            "tertiary" -> MotionEvent.BUTTON_TERTIARY
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
            arguments.getString("away")?.let { away ->
                followAway(path, away)
                return
            }
            val repeat = arguments.getString("repeat")?.toInt() ?: 1
            for (k in 0 until repeat) {
                follow(turned(path, k))
                SystemClock.sleep(24)
            }
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

    /**
     * The pen down along [path] and still on the glass at its end when the app is sent away
     * (`-e away sleep|home`, S50): the screen turned off (`KEYCODE_SLEEP`) or the home key
     * pressed, as a key the system takes; then, 1.5 s later, the pen's lift, which the system
     * may refuse once the window is gone.
     */
    private fun followAway(path: List<FloatArray>, away: String) {
        val stylus = pen
        val down = SystemClock.uptimeMillis()
        send(down, MotionEvent.ACTION_DOWN, path.take(1), stylus)
        for (point in path.drop(1)) {
            SystemClock.sleep(8)
            send(down, MotionEvent.ACTION_MOVE, listOf(point), stylus)
        }
        val code = if (away == "home") KeyEvent.KEYCODE_HOME else KeyEvent.KEYCODE_SLEEP
        val pressed = SystemClock.uptimeMillis()
        for (action in listOf(KeyEvent.ACTION_DOWN, KeyEvent.ACTION_UP)) {
            val key = KeyEvent(pressed, SystemClock.uptimeMillis(), action, code, 0)
            check(uiAutomation.injectInputEvent(key, true)) { "the system refused the key $away" }
        }
        SystemClock.sleep(1500)
        try {
            send(down, MotionEvent.ACTION_UP, path.takeLast(1), stylus)
        } catch (refused: IllegalStateException) {
            // The window went with the screen or home: no lift reaches it.
        }
    }

    /**
     * Stroke [k] of a repeated path (`-e repeat N`, S48): the path turned about the middle of
     * its box by k golden angles and moved up to 140 px from there, so N strokes cross one area
     * at every angle, as many strokes painted over one place do.
     */
    private fun turned(path: List<FloatArray>, k: Int): List<FloatArray> {
        if (k == 0) return path
        val middle = floatArrayOf(
            (path.minOf { it[0] } + path.maxOf { it[0] }) / 2f,
            (path.minOf { it[1] } + path.maxOf { it[1] }) / 2f,
        )
        val angle = k * 2.3999632
        val (cos, sin) = Pair(Math.cos(angle).toFloat(), Math.sin(angle).toFloat())
        val shift = floatArrayOf(((k * 37) % 281 - 140).toFloat(), ((k * 59) % 281 - 140).toFloat())
        return path.map { point ->
            val (x, y) = Pair(point[0] - middle[0], point[1] - middle[1])
            floatArrayOf(
                middle[0] + shift[0] + x * cos - y * sin,
                middle[1] + shift[1] + x * sin + y * cos,
            )
        }
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
        // Looked up once: the lookup's binder calls took about 25 ms an event (S48), so a path
        // came at 33 ms a point instead of 8.
        val device = devices.getOrPut(source) {
            InputDevice.getDeviceIds().firstOrNull { id ->
                InputDevice.getDevice(id)?.supportsSource(source) == true
            } ?: error("no input device for source $source")
        }
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
                if (stylus) setAxisValue(MotionEvent.AXIS_TILT, tilt)
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
