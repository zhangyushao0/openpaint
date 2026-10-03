package dev.openpaint

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import java.io.File
import java.io.FileNotFoundException

/**
 * Serves the file made for the share sheet (`cacheDir/share/<name>`) read-only to the app the
 * user shares it with, which gets a per-URI read grant; nothing else is reachable. A file
 * provider of our own, so the host needs no AndroidX (DECISIONS D11, D14).
 */
class ShareProvider : ContentProvider() {
    override fun onCreate(): Boolean = true

    /** The shared file a URI names, if it is a plain name in the share directory. */
    private fun file(uri: Uri): File? {
        val name = uri.lastPathSegment ?: return null
        if (name.contains('/') || name == "." || name == "..") return null
        val dir = File(context?.cacheDir ?: return null, "share")
        return File(dir, name).takeIf { it.isFile }
    }

    override fun getType(uri: Uri): String? = when (file(uri)?.extension?.lowercase()) {
        "png" -> "image/png"
        "jpg", "jpeg" -> "image/jpeg"
        else -> null
    }

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (mode != "r") throw SecurityException("shared files are read-only")
        val shared = file(uri) ?: throw FileNotFoundException(uri.toString())
        return ParcelFileDescriptor.open(shared, ParcelFileDescriptor.MODE_READ_ONLY)
    }

    override fun query(
        uri: Uri,
        projection: Array<out String>?,
        selection: String?,
        selectionArgs: Array<out String>?,
        sortOrder: String?,
    ): Cursor? {
        val shared = file(uri) ?: return null
        val columns = projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)
        val cursor = MatrixCursor(columns)
        cursor.addRow(columns.map {
            when (it) {
                OpenableColumns.DISPLAY_NAME -> shared.name
                OpenableColumns.SIZE -> shared.length()
                else -> null
            }
        })
        return cursor
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?): Int = 0

    override fun update(
        uri: Uri,
        values: ContentValues?,
        selection: String?,
        selectionArgs: Array<out String>?,
    ): Int = 0

    companion object {
        const val AUTHORITY = "dev.openpaint.share"

        /** The URI other apps read `name` from. */
        fun uri(name: String): Uri =
            Uri.Builder().scheme("content").authority(AUTHORITY).appendPath(name).build()
    }
}
