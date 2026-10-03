package dev.openpaint

import android.content.Context
import android.text.InputType
import android.view.SurfaceView
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection

/** `kind` of [Native.text]; must match `keys.rs`. */
private const val COMPOSE = 0
private const val COMMIT = 1

/** Editing key codes of [Native.key]; must match `keys.rs`. */
internal const val KEY_BACKSPACE = 0
internal const val KEY_DELETE = 1
internal const val KEY_ENTER = 2
internal const val KEY_LEFT = 3
internal const val KEY_RIGHT = 4

/**
 * The full-screen surface the app draws into. While a text field in the UI has the focus
 * ([editing]) it is also the soft keyboard's text editor: the keyboard's text goes to Rust
 * through [Native]; the field itself lives in the UI. No product rules here.
 */
internal class CanvasView(context: Context) : SurfaceView(context) {
    var handle = 0L
    /** Asks the activity for a frame after input. */
    var onInput: () -> Unit = {}
    var editing = false

    init {
        isFocusable = true
        isFocusableInTouchMode = true
    }

    override fun onCheckIsTextEditor(): Boolean = editing

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? {
        if (!editing) return null
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
        // In landscape a keyboard otherwise takes the whole screen with its own text box.
        outAttrs.imeOptions = EditorInfo.IME_ACTION_DONE or
            EditorInfo.IME_FLAG_NO_EXTRACT_UI or
            EditorInfo.IME_FLAG_NO_FULLSCREEN
        return Connection()
    }

    /**
     * Forwards composing and committed text, deletions and the Done action. Its own text buffer
     * stays empty: the UI's field holds the text, so a keyboard deleting with nothing before the
     * cursor sends delete key events, which the activity forwards.
     */
    private inner class Connection : BaseInputConnection(this@CanvasView, true) {
        private var composing = ""

        override fun setComposingText(text: CharSequence, newCursorPosition: Int): Boolean {
            composing = text.toString()
            Native.text(handle, COMPOSE, composing)
            onInput()
            return true
        }

        override fun commitText(text: CharSequence, newCursorPosition: Int): Boolean {
            composing = ""
            Native.text(handle, COMMIT, text.toString())
            onInput()
            return true
        }

        override fun finishComposingText(): Boolean {
            if (composing.isNotEmpty()) {
                Native.text(handle, COMMIT, composing)
                composing = ""
                onInput()
            }
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            Native.deleteSurrounding(handle, beforeLength, afterLength)
            onInput()
            return true
        }

        override fun performEditorAction(editorAction: Int): Boolean {
            Native.key(handle, KEY_ENTER)
            onInput()
            return true
        }
    }
}
