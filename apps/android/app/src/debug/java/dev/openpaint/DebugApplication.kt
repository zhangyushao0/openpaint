package dev.openpaint

import android.app.Activity
import android.app.Application
import android.os.Bundle

/**
 * Debug builds only: the app's process as the debug build starts it, the recorder of frames in
 * real use switched on before the activity makes its runtime ([DebugControls.recordFrames]): each
 * stretch of motion a line in `files/frames.txt`, which `cargo device-frames` reads. It also knows
 * the activity in front ([front]), so [GestureInstrumentation] can ask it for a frame after a chore
 * that drew frames of its own. A release has no such application, so its runtime records nothing.
 */
class DebugApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        DebugControls.recordFrames()
        registerActivityLifecycleCallbacks(Front)
    }

    /** The resumed activity, while one is; read and written on the main thread. */
    internal object Front : ActivityLifecycleCallbacks {
        var activity: MainActivity? = null
            private set

        override fun onActivityResumed(activity: Activity) {
            this.activity = activity as? MainActivity
        }

        override fun onActivityPaused(activity: Activity) {
            if (this.activity === activity) this.activity = null
        }

        override fun onActivityCreated(activity: Activity, state: Bundle?) {}
        override fun onActivityStarted(activity: Activity) {}
        override fun onActivityStopped(activity: Activity) {}
        override fun onActivitySaveInstanceState(activity: Activity, state: Bundle) {}
        override fun onActivityDestroyed(activity: Activity) {}
    }

    companion object {
        /** The activity in front, if any. */
        val front: MainActivity?
            get() = Front.activity
    }
}
