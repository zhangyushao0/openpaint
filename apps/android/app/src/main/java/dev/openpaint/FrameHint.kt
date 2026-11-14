package dev.openpaint

import android.app.Activity
import android.hardware.display.DisplayManager
import android.os.Build
import android.os.PerformanceHintManager
import android.os.Process
import android.util.Log
import java.util.Locale

/** Used when the display reports no refresh rate. */
private const val FALLBACK_HZ = 60f

/**
 * The frame thread's performance hint session (ADPF, API 31): the system is told that the
 * thread works to the display's frame period and how long each frame's work took on it, so it
 * can place and clock the thread to meet that. Open while the surface is; below API 31, or when
 * the device has no manager or refuses a session, every call does nothing. No product rules.
 */
internal class FrameHint(private val activity: Activity) : DisplayManager.DisplayListener {
    private var session: PerformanceHintManager.Session? = null
    private var targetNanos = 0L

    /** On the thread that runs frames, once its surface exists: opens the session for it. */
    fun open() {
        if (session != null) return
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            Log.i("openpaint", "no performance hint session: API ${Build.VERSION.SDK_INT} < 31")
            return
        }
        val manager = activity.getSystemService(PerformanceHintManager::class.java)
        if (manager == null) {
            Log.i("openpaint", "no performance hint session: no PerformanceHintManager")
            return
        }
        val tid = Process.myTid()
        targetNanos = periodNanos()
        session = try {
            manager.createHintSession(intArrayOf(tid), targetNanos)
        } catch (error: RuntimeException) {
            Log.i("openpaint", "no performance hint session: $error")
            return
        }
        if (session == null) {
            Log.i("openpaint", "no performance hint session: the device refused one (tid $tid)")
            return
        }
        Log.i("openpaint", "performance hint session for tid $tid: target ${millis(targetNanos)} ms")
        activity.getSystemService(DisplayManager::class.java)?.registerDisplayListener(this, null)
    }

    /** The time one frame's work took on the session's thread; not called between frames. */
    fun report(workNanos: Long) {
        if (workNanos > 0 && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            session?.reportActualWorkDuration(workNanos)
        }
    }

    /** The surface or the activity went away. */
    fun close() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) return
        val open = session ?: return
        activity.getSystemService(DisplayManager::class.java)?.unregisterDisplayListener(this)
        open.close()
        session = null
    }

    /** The refresh rate changed: the target follows it. */
    override fun onDisplayChanged(displayId: Int) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) return
        val open = session ?: return
        val period = periodNanos()
        if (period == targetNanos) return
        targetNanos = period
        open.updateTargetWorkDuration(period)
        Log.i("openpaint", "performance hint target ${millis(period)} ms")
    }

    override fun onDisplayAdded(displayId: Int) {}

    override fun onDisplayRemoved(displayId: Int) {}

    /** The display's frame period now, from its current refresh rate. */
    private fun periodNanos(): Long {
        val hz = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            activity.display?.refreshRate ?: FALLBACK_HZ
        } else {
            FALLBACK_HZ
        }
        return (1e9 / (if (hz >= 1f) hz else FALLBACK_HZ)).toLong()
    }

    private fun millis(nanos: Long): String = String.format(Locale.ROOT, "%.2f", nanos / 1e6)
}
