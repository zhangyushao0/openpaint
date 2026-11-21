package dev.openpaint

import android.content.ContentValues
import android.content.Context
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.media.MediaMuxer
import android.net.Uri
import android.opengl.EGL14
import android.opengl.EGLConfig
import android.opengl.EGLContext
import android.opengl.EGLDisplay
import android.opengl.EGLExt
import android.opengl.EGLSurface
import android.opengl.GLES20
import android.os.Environment
import android.os.ParcelFileDescriptor
import android.provider.MediaStore
import android.util.Log
import android.view.Surface
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer

private const val MIME = MediaFormat.MIMETYPE_VIDEO_AVC

/** `EGL_RECORDABLE_ANDROID`: a config whose surfaces an encoder takes. */
private const val EGL_RECORDABLE = 0x3142

/** How long the encoder may take to end the video once its input ended. */
private const val END_WAIT_NS = 5_000_000_000L

/** Bits a second per pixel a second: a sixth of a bit, plenty for line art on paper. */
private const val BITS_DIVISOR = 6L

/** The quad over the whole frame; the picture's first row (top first) at the frame's top. */
private const val VERTEX = """
attribute vec2 position;
varying vec2 texel;
void main() {
    texel = vec2(position.x + 1.0, 1.0 - position.y) * 0.5;
    gl_Position = vec4(position, 0.0, 1.0);
}
"""

private const val FRAGMENT = """
precision mediump float;
uniform sampler2D picture;
varying vec2 texel;
void main() {
    gl_FragColor = texture2D(picture, texel);
}
"""

/** A failure of the video's file, not of its encoder. */
private class WriteFailure(message: String, cause: Throwable? = null) : Exception(message, cause)

/**
 * The replay's video (S163; the Rust side's `apps/android/src/video.rs`): H.264 through
 * MediaCodec's input surface, each RGBA frame drawn on it as a textured quad by an EGL context of
 * its own and stamped with its time (`eglPresentationTimeANDROID`), muxed by MediaMuxer into an
 * MP4: into the album's Movies/OpenPaint, pending until finished, or into the file the Rust side
 * made for the share sheet. The encoder must take the size (`isSizeSupported`); there is no
 * smaller fallback. Every call comes from the Rust side's video worker, on its thread, in order:
 * [begin], [frame] for each frame, then [finish], or [cancel] at any point. A call answers null,
 * or "encoder\t<why>" or "write\t<why>" after letting everything of the video go; nothing is
 * thrown into Rust. No product rules here.
 */
class VideoEncoder {
    private var codec: MediaCodec? = null
    private var input: Surface? = null
    private var muxer: MediaMuxer? = null
    private var track = -1
    private var muxing = false
    private val info = MediaCodec.BufferInfo()
    private var display: EGLDisplay = EGL14.EGL_NO_DISPLAY
    private var context: EGLContext = EGL14.EGL_NO_CONTEXT
    private var window: EGLSurface = EGL14.EGL_NO_SURFACE
    /** The quad's corners, read by GL at each draw: kept while the video is made. */
    private var quad: FloatBuffer? = null
    private var width = 0
    private var height = 0
    /** Where the video goes: the album's row and its file, or the file for the share sheet. */
    private var album: Uri? = null
    private var descriptor: ParcelFileDescriptor? = null
    private var shared: File? = null

    /**
     * Opens the encoder for frames of [width] × [height] at [fps] and the video [name]: in the
     * album, unless [sharePath] names the file made for the share sheet.
     */
    fun begin(name: String, width: Int, height: Int, fps: Int, sharePath: String?): String? =
        attempt {
            val codec = MediaCodec.createEncoderByType(MIME)
            this.codec = codec
            val video = codec.codecInfo.getCapabilitiesForType(MIME).videoCapabilities
            check(video.isSizeSupported(width, height)) {
                "${codec.name} takes no $width × $height"
            }
            val bits = (width.toLong() * height * fps / BITS_DIVISOR)
                .coerceAtMost(Int.MAX_VALUE.toLong()).toInt()
            val format = MediaFormat.createVideoFormat(MIME, width, height).apply {
                setInteger(
                    MediaFormat.KEY_COLOR_FORMAT,
                    MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface,
                )
                setInteger(MediaFormat.KEY_BIT_RATE, video.bitrateRange.clamp(bits))
                setInteger(MediaFormat.KEY_FRAME_RATE, fps)
                setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1)
            }
            codec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
            val surface = codec.createInputSurface()
            input = surface
            this.width = width
            this.height = height
            openGl(surface)
            codec.start()
            muxer = open(name, sharePath)
        }

    /**
     * One frame: opaque RGBA rows of the video's size, top first; [atUs] its time. The video's own
     * context and the encoder's surface are made current first: whatever else ran on the thread
     * since (a GPU device opened or let go) cannot have taken them. Its upload and drawing are
     * finished before it goes to the encoder, so the next frame's upload never finds the texture
     * in use: a driver would keep a copy of it for each frame the GPU is behind (the GPU is busy
     * with the frames being drawn), memory the app counts nowhere.
     */
    fun frame(rgba: ByteBuffer, atUs: Long): String? = attempt {
        check(rgba.capacity() >= width * height * 4) { "a frame of ${rgba.capacity()} bytes" }
        check(EGL14.eglMakeCurrent(display, window, window, context)) {
            "the video's EGL context could not be made current: EGL error ${EGL14.eglGetError()}"
        }
        drain(end = false)
        GLES20.glTexSubImage2D(
            GLES20.GL_TEXTURE_2D, 0, 0, 0, width, height,
            GLES20.GL_RGBA, GLES20.GL_UNSIGNED_BYTE, rgba,
        )
        GLES20.glDrawArrays(GLES20.GL_TRIANGLE_STRIP, 0, 4)
        GLES20.glFinish()
        glChecked("drawing a frame")
        EGLExt.eglPresentationTimeANDROID(display, window, atUs * 1_000)
        check(EGL14.eglSwapBuffers(display, window)) {
            "the frame did not reach the encoder: EGL error ${EGL14.eglGetError()}"
        }
    }

    /** Ends the video: the file complete and, in the album, shown there. */
    fun finish(): String? = attempt {
        checkNotNull(codec) { "no video begun" }.signalEndOfInputStream()
        drain(end = true)
        written { muxer?.stop() }
        muxing = false
        release()
        album?.let { uri ->
            val shown = ContentValues().apply { put(MediaStore.Video.Media.IS_PENDING, 0) }
            written { appContext().contentResolver.update(uri, shown, null, null) }
            last = uri
        }
        shared?.let { last = ShareProvider.uri(it.name) }
        album = null
        shared = null
    }

    /** Drops the video: nothing of it stays, in the album or the shares. */
    fun cancel() {
        release()
        album?.let { uri -> runCatching { appContext().contentResolver.delete(uri, null, null) } }
        shared?.let { runCatching { it.delete() } }
        album = null
        shared = null
    }

    /** The file the muxer writes: a new pending row of the album's, or the file to share. */
    private fun open(name: String, sharePath: String?): MediaMuxer = written {
        if (sharePath != null) {
            shared = File(sharePath)
            return@written MediaMuxer(sharePath, MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4)
        }
        val resolver = appContext().contentResolver
        val values = ContentValues().apply {
            put(MediaStore.Video.Media.DISPLAY_NAME, name)
            put(MediaStore.Video.Media.MIME_TYPE, "video/mp4")
            put(MediaStore.Video.Media.RELATIVE_PATH, "${Environment.DIRECTORY_MOVIES}/OpenPaint")
            put(MediaStore.Video.Media.IS_PENDING, 1)
        }
        val videos = MediaStore.Video.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
        val uri = resolver.insert(videos, values) ?: throw WriteFailure("the album took no video")
        album = uri
        val file = resolver.openFileDescriptor(uri, "rw")
            ?: throw WriteFailure("the album gave no file for $uri")
        descriptor = file
        MediaMuxer(file.fileDescriptor, MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4)
    }

    /** An EGL context of this thread's drawing on the encoder's [surface], and what draws. */
    private fun openGl(surface: Surface) {
        display = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY)
        val version = IntArray(2)
        check(EGL14.eglInitialize(display, version, 0, version, 1)) { "no EGL display" }
        val wanted = intArrayOf(
            EGL14.EGL_RED_SIZE, 8, EGL14.EGL_GREEN_SIZE, 8, EGL14.EGL_BLUE_SIZE, 8,
            EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT, EGL_RECORDABLE, 1,
            EGL14.EGL_NONE,
        )
        val configs = arrayOfNulls<EGLConfig>(1)
        val found = IntArray(1)
        EGL14.eglChooseConfig(display, wanted, 0, configs, 0, 1, found, 0)
        val config = configs[0].takeIf { found[0] > 0 } ?: error("no recordable EGL config")
        val client = intArrayOf(EGL14.EGL_CONTEXT_CLIENT_VERSION, 2, EGL14.EGL_NONE)
        context = EGL14.eglCreateContext(display, config, EGL14.EGL_NO_CONTEXT, client, 0)
        check(context != EGL14.EGL_NO_CONTEXT) { "no EGL context" }
        val none = intArrayOf(EGL14.EGL_NONE)
        window = EGL14.eglCreateWindowSurface(display, config, surface, none, 0)
        check(window != EGL14.EGL_NO_SURFACE) { "no EGL surface on the encoder" }
        check(EGL14.eglMakeCurrent(display, window, window, context)) { "no current EGL context" }
        program()
    }

    /** The quad's program, its corners and the picture's texture, the viewport the frame's. */
    private fun program() {
        val program = GLES20.glCreateProgram()
        GLES20.glAttachShader(program, shader(GLES20.GL_VERTEX_SHADER, VERTEX))
        GLES20.glAttachShader(program, shader(GLES20.GL_FRAGMENT_SHADER, FRAGMENT))
        GLES20.glBindAttribLocation(program, 0, "position")
        GLES20.glLinkProgram(program)
        val linked = IntArray(1)
        GLES20.glGetProgramiv(program, GLES20.GL_LINK_STATUS, linked, 0)
        check(linked[0] != 0) { "the frame's program: ${GLES20.glGetProgramInfoLog(program)}" }
        GLES20.glUseProgram(program)
        val corners = floatArrayOf(-1f, -1f, 1f, -1f, -1f, 1f, 1f, 1f)
        val quad = ByteBuffer.allocateDirect(corners.size * 4).order(ByteOrder.nativeOrder())
            .asFloatBuffer().put(corners)
        quad.position(0)
        this.quad = quad
        GLES20.glVertexAttribPointer(0, 2, GLES20.GL_FLOAT, false, 0, quad)
        GLES20.glEnableVertexAttribArray(0)
        val texture = IntArray(1)
        GLES20.glGenTextures(1, texture, 0)
        GLES20.glActiveTexture(GLES20.GL_TEXTURE0)
        GLES20.glBindTexture(GLES20.GL_TEXTURE_2D, texture[0])
        for (filter in intArrayOf(GLES20.GL_TEXTURE_MIN_FILTER, GLES20.GL_TEXTURE_MAG_FILTER)) {
            GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, filter, GLES20.GL_NEAREST)
        }
        for (wrap in intArrayOf(GLES20.GL_TEXTURE_WRAP_S, GLES20.GL_TEXTURE_WRAP_T)) {
            GLES20.glTexParameteri(GLES20.GL_TEXTURE_2D, wrap, GLES20.GL_CLAMP_TO_EDGE)
        }
        GLES20.glPixelStorei(GLES20.GL_UNPACK_ALIGNMENT, 1)
        GLES20.glTexImage2D(
            GLES20.GL_TEXTURE_2D, 0, GLES20.GL_RGBA, width, height, 0,
            GLES20.GL_RGBA, GLES20.GL_UNSIGNED_BYTE, null,
        )
        GLES20.glUniform1i(GLES20.glGetUniformLocation(program, "picture"), 0)
        GLES20.glViewport(0, 0, width, height)
        glChecked("setting up the frame's drawing")
    }

    private fun shader(kind: Int, source: String): Int {
        val shader = GLES20.glCreateShader(kind)
        GLES20.glShaderSource(shader, source)
        GLES20.glCompileShader(shader)
        val compiled = IntArray(1)
        GLES20.glGetShaderiv(shader, GLES20.GL_COMPILE_STATUS, compiled, 0)
        check(compiled[0] != 0) { "a shader: ${GLES20.glGetShaderInfoLog(shader)}" }
        return shader
    }

    private fun glChecked(doing: String) {
        val error = GLES20.glGetError()
        check(error == GLES20.GL_NO_ERROR) { "GL error $error $doing" }
    }

    /**
     * Hands what the encoder made to the muxer: what is ready now, or with [end] everything up to
     * the end of the stream, waiting for it at most [END_WAIT_NS].
     */
    private fun drain(end: Boolean) {
        val codec = checkNotNull(codec) { "no video begun" }
        val deadline = System.nanoTime() + END_WAIT_NS
        while (true) {
            val index = codec.dequeueOutputBuffer(info, if (end) 10_000L else 0L)
            if (index == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                val muxer = checkNotNull(muxer) { "no file for the video" }
                written {
                    track = muxer.addTrack(codec.outputFormat)
                    muxer.start()
                }
                muxing = true
            } else if (index < 0) {
                if (!end) return
                check(System.nanoTime() < deadline) { "the encoder did not end the video" }
            } else {
                val ended = info.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0
                write(codec, index)
                if (ended) return
            }
        }
    }

    /** The encoder's output buffer [index] into the file, unless it holds only its setup. */
    private fun write(codec: MediaCodec, index: Int) {
        val data = codec.getOutputBuffer(index)
        val setup = info.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG != 0
        if (data != null && !setup && info.size > 0) {
            check(muxing) { "a frame before the video's format" }
            data.position(info.offset)
            data.limit(info.offset + info.size)
            written { muxer?.writeSampleData(track, data, info) }
        }
        codec.releaseOutputBuffer(index, false)
    }

    /** Lets everything of the video go but its file, quietly: what fails to stop goes anyway. */
    private fun release() {
        codec?.let {
            runCatching { it.stop() }
            runCatching { it.release() }
        }
        codec = null
        muxer?.let {
            if (muxing) runCatching { it.stop() }
            runCatching { it.release() }
        }
        muxer = null
        muxing = false
        track = -1
        if (display != EGL14.EGL_NO_DISPLAY) {
            val nothing = EGL14.EGL_NO_SURFACE
            EGL14.eglMakeCurrent(display, nothing, nothing, EGL14.EGL_NO_CONTEXT)
            if (window != EGL14.EGL_NO_SURFACE) EGL14.eglDestroySurface(display, window)
            if (context != EGL14.EGL_NO_CONTEXT) EGL14.eglDestroyContext(display, context)
            EGL14.eglReleaseThread()
        }
        display = EGL14.EGL_NO_DISPLAY
        context = EGL14.EGL_NO_CONTEXT
        window = EGL14.EGL_NO_SURFACE
        quad = null
        input?.release()
        input = null
        descriptor?.let { runCatching { it.close() } }
        descriptor = null
    }

    /** [work], its failures the file's. */
    private inline fun <T> written(work: () -> T): T = try {
        work()
    } catch (e: WriteFailure) {
        throw e
    } catch (e: Exception) {
        throw WriteFailure(e.message ?: e.javaClass.name, e)
    }

    /** [work]: null, or after letting the video go "write\t<why>" or "encoder\t<why>". */
    private inline fun attempt(work: () -> Unit): String? = try {
        work()
        null
    } catch (e: Exception) {
        val kind = if (e is WriteFailure) "write" else "encoder"
        Log.e("openpaint", "video: $kind failed", e)
        cancel()
        "$kind\t${(e.message ?: e.javaClass.name).replace('\t', ' ')}"
    }

    companion object {
        /** The app's context (MainActivity sets it): the album is reached through its resolver. */
        @Volatile
        var app: Context? = null

        /** The video last made, for the share sheet (`Pickers`' "send-video"). */
        @Volatile
        var last: Uri? = null

        private fun appContext(): Context = checkNotNull(app) { "no context for the album" }
    }
}
