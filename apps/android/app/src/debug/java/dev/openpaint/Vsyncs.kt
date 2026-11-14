package dev.openpaint

import android.os.Handler
import android.os.HandlerThread
import android.view.Choreographer
import java.util.Collections
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Debug builds only: the display's refresh rate while something runs on the main thread (the
 * bench draws its frames there, outside the activity's Choreographer), as Choreographer delivers
 * its vsyncs to a thread of its own meanwhile.
 */
internal object Vsyncs {
    /**
     * [block] run while the vsyncs are counted: its result and the line "display <hz> Hz as
     * Choreographer delivered it (<low>..<high> Hz between vsyncs)", or "display -" for none.
     */
    fun <T> during(block: () -> T): Pair<T, String> {
        val thread = HandlerThread("openpaint-vsyncs").apply { start() }
        val times = Collections.synchronizedList(mutableListOf<Long>())
        val counting = AtomicBoolean(true)
        Handler(thread.looper).post {
            val choreographer = Choreographer.getInstance()
            choreographer.postFrameCallback(object : Choreographer.FrameCallback {
                override fun doFrame(frameTimeNanos: Long) {
                    times.add(frameTimeNanos)
                    if (counting.get()) choreographer.postFrameCallback(this)
                }
            })
        }
        val result = try {
            block()
        } finally {
            counting.set(false)
            thread.quitSafely()
        }
        return result to line(synchronized(times) { times.toList() })
    }

    private fun line(times: List<Long>): String {
        val rates = times.zipWithNext { a, b -> b - a }.filter { it > 0 }.map { 1e9 / it }.sorted()
        if (rates.isEmpty()) return "display -"
        val median = rates[rates.size / 2]
        return "display %.0f Hz as Choreographer delivered it (%.0f..%.0f Hz between vsyncs)".format(
            median, rates.first(), rates.last(),
        )
    }
}
