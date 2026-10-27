package dev.dortyuzlu.diary

import android.app.Activity
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.view.Gravity
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import java.io.File
import java.io.FileOutputStream
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

// One input bar. Enter stores the line, with its time, in
//   <phone>/Android/data/dev.dortyuzlu.diary/files/outbox.toml
// as TOML. `sync` on the PC pulls that file over adb.
//
// Same prefixes as the browser panel:  todo: ...   timetable: 14:00-15:30 ...   diary: ...
class MainActivity : Activity() {

    private lateinit var input: EditText
    private lateinit var status: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val bg = Color.parseColor("#1e1e2e")
        window.statusBarColor = bg
        window.navigationBarColor = bg

        input = EditText(this).apply {
            hint = "Write diary, todo: [task], or timetable: [time-range event]..."
            setTextColor(Color.parseColor("#cdd6f4"))
            setHintTextColor(Color.parseColor("#585b70"))
            textSize = 16f
            typeface = android.graphics.Typeface.MONOSPACE
            setPadding(36, 36, 36, 36)
            setSingleLine(true)
            imeOptions = EditorInfo.IME_ACTION_SEND
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#181825"))
                setStroke(4, Color.parseColor("#89b4fa"))
                cornerRadius = 24f
            }
            setOnEditorActionListener { _, actionId, _ ->
                if (actionId == EditorInfo.IME_ACTION_SEND) { submit(); true } else false
            }
        }

        status = TextView(this).apply {
            textSize = 12f
            typeface = android.graphics.Typeface.MONOSPACE
            gravity = Gravity.CENTER
            setPadding(0, 24, 0, 0)
        }

        val hint = TextView(this).apply {
            text = "diary (default) | todo: buy milk #errands @quick | timetable: 14:00-15:30 dentist"
            textSize = 11f
            typeface = android.graphics.Typeface.MONOSPACE
            setTextColor(Color.parseColor("#585b70"))
            gravity = Gravity.CENTER
            setPadding(0, 24, 0, 0)
        }

        setContentView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setBackgroundColor(bg)
            setPadding(48, 48, 48, 48)
            addView(input)
            addView(status)
            addView(hint)
        })
    }

    private fun submit() {
        val content = input.text.toString().trim()
        if (content.isEmpty()) return
        try {
            val module = save(content)
            input.setText("")
            show("sent to $module (${File(filesDirPath(), OUTBOX).length()} bytes stored)", false)
        } catch (e: Exception) {
            // the text stays in the box so nothing typed is lost
            show("could not save: ${e.message}", true)
        }
    }

    private fun show(message: String, bad: Boolean) {
        status.text = message
        status.setTextColor(Color.parseColor(if (bad) "#f38ba8" else "#a6e3a1"))
    }

    private fun filesDirPath(): File =
        getExternalFilesDir(null) ?: throw IllegalStateException("no shared storage")

    private fun save(content: String): String {
        var module = "diary"
        var text = content
        for (prefix in listOf("todo", "timetable", "diary")) {
            if (content.startsWith("$prefix:")) {
                module = prefix
                text = content.removePrefix("$prefix:").trim()
                break
            }
        }
        if (text.isEmpty()) throw IllegalArgumentException("nothing after \"$module:\"")

        val now = Date()
        val id = now.time.toString() // unique per entry; the PC skips ids it already has
        val at = SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.US).format(now)

        val entry = "\n[[entry]]\n" +
            "id = \"$id\"\n" +
            "module = \"$module\"\n" +
            "text = \"${tomlEscape(text)}\"\n" +
            "at = \"$at\"\n"

        FileOutputStream(File(filesDirPath(), OUTBOX), true).use { out ->
            out.write(entry.toByteArray(Charsets.UTF_8))
            out.fd.sync() // on the disk before we say it was sent
        }
        return module
    }

    // TOML basic string escaping
    private fun tomlEscape(s: String): String {
        val b = StringBuilder()
        for (c in s) {
            when {
                c == '\\' -> b.append("\\\\")
                c == '"' -> b.append("\\\"")
                c == '\n' -> b.append("\\n")
                c == '\r' -> b.append("\\r")
                c == '\t' -> b.append("\\t")
                c.code < 0x20 || c.code == 0x7f -> b.append(String.format("\\u%04X", c.code))
                else -> b.append(c)
            }
        }
        return b.toString()
    }

    companion object { const val OUTBOX = "outbox.toml" }
}
