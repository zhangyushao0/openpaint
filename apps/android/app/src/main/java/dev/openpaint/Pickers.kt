package dev.openpaint

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Intent
import android.net.Uri
import android.util.Log
import java.io.File

private const val REQUEST_SAVE = 1
private const val REQUEST_OPEN = 2

/**
 * The system's pickers and share sheet for the Rust side's requests (DECISIONS D11): the
 * save dialog (Storage Access Framework, ACTION_CREATE_DOCUMENT), the image and file pickers
 * (ACTION_OPEN_DOCUMENT: PNG and JPEG, or any file) and the share sheet (ACTION_SEND with a
 * content URI from ShareProvider). What the user chose goes back as a detached file
 * descriptor; no product rules here.
 */
internal class Pickers(private val activity: Activity, private val handle: () -> Long) {
    /** Serves every request the last frame left. */
    fun serve() {
        while (true) {
            val line = Native.request(handle()) ?: return
            val fields = line.split('\t')
            when (fields[0]) {
                "save" -> start(REQUEST_SAVE, Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = fields[1]
                    putExtra(Intent.EXTRA_TITLE, fields[2])
                })
                "open" -> start(REQUEST_OPEN, Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = "image/*"
                    putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("image/png", "image/jpeg"))
                })
                // An OpenPaint file: Android has no type for it, so every file is offered and
                // the Rust side checks what was chosen.
                "open-file" -> start(REQUEST_OPEN, Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                    addCategory(Intent.CATEGORY_OPENABLE)
                    type = "*/*"
                })
                "send" -> send(fields[1], File(fields[2]).name)
                else -> Log.e("openpaint", "unknown request: $line")
            }
        }
    }

    private fun start(code: Int, intent: Intent) {
        try {
            activity.startActivityForResult(intent, code)
        } catch (e: ActivityNotFoundException) {
            Native.answer(handle(), -1, e.message ?: "no app for $intent")
        }
    }

    private fun send(mime: String, name: String) {
        val uri = ShareProvider.uri(name)
        val send = Intent(Intent.ACTION_SEND).apply {
            type = mime
            putExtra(Intent.EXTRA_STREAM, uri)
            clipData = ClipData.newRawUri(name, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        try {
            activity.startActivity(Intent.createChooser(send, null))
        } catch (e: ActivityNotFoundException) {
            Log.e("openpaint", "no share sheet: ${e.message}")
        }
    }

    /** A picker's result; returns false for results that are not ours. */
    fun result(code: Int, resultCode: Int, data: Intent?): Boolean {
        val mode = when (code) {
            REQUEST_SAVE -> "wt"
            REQUEST_OPEN -> "r"
            else -> return false
        }
        val uri = data?.data
        if (resultCode != Activity.RESULT_OK || uri == null) {
            Native.answer(handle(), -1, null)
            return true
        }
        try {
            Native.answer(handle(), open(uri, mode), null)
        } catch (e: Exception) {
            Native.answer(handle(), -1, e.message ?: e.javaClass.name)
        }
        return true
    }

    /** A descriptor for `uri`, detached: the Rust side owns and closes it. */
    private fun open(uri: Uri, mode: String): Int {
        val resolver = activity.contentResolver
        val file = try {
            resolver.openFileDescriptor(uri, mode)
        } catch (e: IllegalArgumentException) {
            // Some providers know only "w", which truncates on their side.
            if (mode == "wt") resolver.openFileDescriptor(uri, "w") else throw e
        } ?: throw IllegalStateException("the provider gave no file for $uri")
        return file.detachFd()
    }
}
