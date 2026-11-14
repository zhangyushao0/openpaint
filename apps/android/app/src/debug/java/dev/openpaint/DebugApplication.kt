package dev.openpaint

import android.app.Application

/**
 * Debug builds only: the app's process as the debug build starts it, the recorder of frames in
 * real use switched on before the activity makes its runtime ([DebugControls.recordFrames]): each
 * stretch of motion a line in `files/frames.txt`, which `cargo device-frames` reads. A release
 * has no such application, so its runtime records nothing.
 */
class DebugApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        DebugControls.recordFrames()
    }
}
