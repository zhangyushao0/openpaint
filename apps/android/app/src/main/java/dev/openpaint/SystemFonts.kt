package dev.openpaint

import android.util.Log
import android.util.Xml
import java.io.File
import java.io.FileInputStream
import org.xmlpull.v1.XmlPullParser

/**
 * Finds the system's regular text faces in `/system/etc/fonts.xml` for the UI: the default
 * sans (Roboto) and the Simplified Chinese face (Noto Sans CJK). Each comes back as
 * "<face index>\n<path>" for Rust to read, or null when the system has none. The font files
 * stay the system's; nothing is copied.
 */
internal object SystemFonts {
    fun latin(): String? = find { family, lang -> family == "sans-serif" && lang == null }

    fun chinese(): String? = find { _, lang -> lang?.split(' ')?.contains("zh-Hans") == true }

    /** The first weight-400 upright face of the first family `wanted` accepts. */
    private fun find(wanted: (name: String?, lang: String?) -> Boolean): String? = try {
        FileInputStream("/system/etc/fonts.xml").use { input ->
            val xml = Xml.newPullParser()
            xml.setInput(input, "UTF-8")
            var inFamily = false
            var event = xml.next()
            var found: String? = null
            while (event != XmlPullParser.END_DOCUMENT && found == null) {
                if (event == XmlPullParser.START_TAG && xml.name == "family") {
                    inFamily = wanted(xml.getAttributeValue(null, "name"), xml.getAttributeValue(null, "lang"))
                } else if (event == XmlPullParser.END_TAG && xml.name == "family") {
                    inFamily = false
                } else if (inFamily && event == XmlPullParser.START_TAG && xml.name == "font" &&
                    xml.getAttributeValue(null, "weight") == "400" &&
                    xml.getAttributeValue(null, "style") != "italic"
                ) {
                    val index = xml.getAttributeValue(null, "index")?.toIntOrNull() ?: 0
                    found = face(index, fileName(xml))
                }
                event = xml.next()
            }
            found
        }
    } catch (error: Exception) {
        Log.e("openpaint", "cannot read the system's font list", error)
        null
    }

    /** The text of the `font` element the parser stands on: its file's name. */
    private fun fileName(xml: XmlPullParser): String {
        val depth = xml.depth
        val name = StringBuilder()
        var event = xml.next()
        while (!(event == XmlPullParser.END_TAG && xml.depth == depth)) {
            if (event == XmlPullParser.TEXT && xml.depth == depth) name.append(xml.text)
            event = xml.next()
        }
        return name.toString().trim()
    }

    /** "<index>\n<path>" for a file in /system/fonts, or null when it is not there. */
    private fun face(index: Int, name: String): String? {
        val root = File("/system/fonts").canonicalFile
        val file = File(root, name).canonicalFile
        if (!file.path.startsWith(root.path + "/") || !file.isFile) return null
        return "$index\n${file.path}"
    }
}
