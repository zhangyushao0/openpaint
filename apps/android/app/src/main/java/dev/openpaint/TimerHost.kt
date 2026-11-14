package dev.openpaint

import android.app.Activity
import android.media.AudioManager
import android.media.ToneGenerator
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.WindowManager

/** The chime: a short system tone at this volume (0 to 100), for this long. */
private const val CHIME_VOLUME = 80
private const val CHIME_MS = 180

/**
 * What the Pomodoro asks of the host after each frame (S124): a frame when its timer wants one
 * (the turn of a minute shown, the end of a block or a break; nothing draws at rest otherwise),
 * the window's keep-screen-on flag while it counts, and the break's chime as one short system
 * tone (no asset). Rust decides each; no product rules here.
 */
internal class TimerHost(private val activity: Activity, private val frame: () -> Unit) {
    private val main = Handler(Looper.getMainLooper())
    private val wake = Runnable { frame() }
    private var keptOn = false

    /** After a frame (or leaving): the next wake posted anew, the flag set, the chime played. */
    fun after(handle: Long) {
        if (handle == 0L) {
            stop()
            return
        }
        wakeIn(Native.wakeIn(handle))
        keepOn(Native.screenOn(handle))
        if (Native.chime(handle)) chime()
    }

    /** No wake left and the screen let go: the activity is going. */
    fun stop() {
        main.removeCallbacks(wake)
        keepOn(false)
    }

    /** The frame posted for `ms` from now, any earlier one cancelled; none for -1. */
    private fun wakeIn(ms: Long) {
        main.removeCallbacks(wake)
        if (ms >= 0) main.postDelayed(wake, ms)
    }

    private fun keepOn(on: Boolean) {
        if (on == keptOn || activity.isDestroyed) return
        val window = activity.window ?: return
        if (on) {
            window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        } else {
            window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
        keptOn = on
        Log.i("openpaint", if (on) "screen kept on" else "screen let go")
    }

    /** One short tone on the notification stream (silent mode silences it), released after. */
    private fun chime() {
        val tone = try {
            ToneGenerator(AudioManager.STREAM_NOTIFICATION, CHIME_VOLUME)
        } catch (error: RuntimeException) {
            Log.w("openpaint", "no tone for the chime: $error")
            return
        }
        val sounded = tone.startTone(ToneGenerator.TONE_PROP_ACK, CHIME_MS)
        Log.i("openpaint", if (sounded) "the break's chime sounded" else "the break's chime refused")
        main.postDelayed({ tone.release() }, CHIME_MS + 200L)
    }
}
