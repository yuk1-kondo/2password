package dev.twopassword

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Handler
import android.os.Looper

/**
 * Copy to clipboard with a 90s auto-clear (spec §8.2). Clears only if the
 * clipboard still holds what we put there — same policy as the macOS
 * changeCount guard.
 */
class ClipboardHelper(ctx: Context) {
    private val cm = ctx.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    private val handler = Handler(Looper.getMainLooper())
    private var lastCopied: String? = null
    private var clearTask: Runnable? = null

    fun copy(text: String) {
        cm.setPrimaryClip(ClipData.newPlainText("2password", text))
        lastCopied = text
        clearTask?.let { handler.removeCallbacks(it) }
        val task = Runnable {
            if (lastCopied == text) {
                val current = cm.primaryClip
                val currentText = current?.getItemAt(0)?.text?.toString()
                if (currentText == text) {
                    cm.setPrimaryClip(ClipData.newPlainText("", ""))
                }
                lastCopied = null
            }
        }
        clearTask = task
        handler.postDelayed(task, 90_000)
    }
}
