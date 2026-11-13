package dev.twopassword

import android.app.PendingIntent
import android.app.assist.AssistStructure
import android.content.Intent
import android.os.CancellationSignal
import android.service.autofill.AutofillService
import android.service.autofill.Dataset
import android.service.autofill.FillCallback
import android.service.autofill.FillRequest
import android.service.autofill.FillResponse
import android.service.autofill.SaveCallback
import android.service.autofill.SaveRequest
import android.view.autofill.AutofillId
import android.view.autofill.AutofillValue
import android.widget.RemoteViews
import uniffi.twopassword_ffi.domainMatches

/**
 * Android Autofill provider (spec §8.3). Two states:
 * - Unlocked vault → fill matching credentials into detected fields.
 * - Locked → authentication intent back to MainActivity; the framework
 *   re-issues a FillRequest once the app returns.
 */
class TwoPasswordAutofillService : AutofillService() {

    private data class FormFields(
        var userId: AutofillId? = null,
        var passId: AutofillId? = null,
        var host: String = "",
    )

    override fun onFillRequest(
        request: FillRequest,
        cancellationSignal: CancellationSignal,
        callback: FillCallback,
    ) {
        val structure = request.fillContexts.last().structure
        val fields = FormFields()
        findFields(structure.getWindowNodeAt(0).rootViewNode, fields)

        if (fields.passId == null) {
            callback.onSuccess(null)
            return
        }
        val store = VaultStore.current
        if (store == null || store.state != VaultState.UNLOCKED) {
            // Locked: present an auth-gated row that opens the app.
            val intent = Intent(this, MainActivity::class.java)
            val sender = PendingIntent.getActivity(
                this, 0, intent,
                PendingIntent.FLAG_CANCEL_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            val label = RemoteViews(packageName, android.R.layout.simple_list_item_1).apply {
                setTextViewText(android.R.id.text1, "🔒 2Password をアンロック")
            }
            val ids = listOfNotNull(fields.userId, fields.passId).toTypedArray()
            callback.onSuccess(
                FillResponse.Builder()
                    .setAuthentication(ids, sender.intentSender, label)
                    .build()
            )
            return
        }

        val matches = store.items.filter {
            domainMatches(it.url, fields.host)
        }
        if (matches.isEmpty()) {
            callback.onSuccess(null)
            return
        }

        val response = FillResponse.Builder()
        for (item in matches) {
            // Passwords live in the detail record, not the list summary.
            val detail = runCatching {
                kotlinx.coroutines.runBlocking { store.detail(item.uuid) }
            }.getOrNull() ?: continue
            val presentation = RemoteViews(packageName, android.R.layout.simple_list_item_1).apply {
                setTextViewText(android.R.id.text1, "🔑 ${item.title}")
            }
            val dataset = Dataset.Builder(presentation)
            fields.userId?.let { id ->
                dataset.setValue(id, AutofillValue.forText(detail.username ?: ""))
            }
            dataset.setValue(
                fields.passId!!,
                AutofillValue.forText(detail.password ?: ""),
            )
            response.addDataset(dataset.build())
        }
        callback.onSuccess(response.build())
    }

    override fun onSaveRequest(request: SaveRequest, callback: SaveCallback) {
        callback.onFailure("save not supported")
    }

    private fun findFields(node: AssistStructure.ViewNode, out: FormFields) {
        val hints = node.autofillHints?.map { it.lowercase() } ?: emptyList()
        when {
            hints.any { it.contains("password") } -> out.passId = node.autofillId
            hints.any { it.contains("username") || it.contains("email") } ->
                out.userId = node.autofillId
        }
        if (!node.webDomain.isNullOrEmpty()) out.host = node.webDomain!!
        for (i in 0 until node.childCount) findFields(node.getChildAt(i), out)
    }
}
