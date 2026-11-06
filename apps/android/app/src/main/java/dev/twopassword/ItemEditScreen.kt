package dev.twopassword

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import kotlin.math.log2
import uniffi.twopassword_ffi.NewItem
import uniffi.twopassword_ffi.generatePassword
import uniffi.twopassword_ffi.normalizeTotpSeed

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ItemEditScreen(
    store: VaultStore,
    uuid: String?,
    onDone: () -> Unit,
) {
    var title by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var url by remember { mutableStateOf("") }
    var notes by remember { mutableStateOf("") }
    var totp by remember { mutableStateOf("") }
    var favorite by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(uuid) {
        if (uuid != null) {
            store.detail(uuid)?.let {
                title = it.title
                username = it.username ?: ""
                password = it.password ?: ""
                url = it.url ?: ""
                notes = it.notes ?: ""
                totp = it.totpSeed ?: ""
                favorite = it.favorite
            }
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(if (uuid == null) "アイテム追加" else "アイテム編集") },
                navigationIcon = {
                    IconButton(onClick = onDone) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "戻る")
                    }
                },
                actions = {
                    TextButton(
                        enabled = title.isNotEmpty(),
                        onClick = {
                            var seed: String? = null
                            if (totp.isNotBlank()) {
                                seed = try {
                                    normalizeTotpSeed(totp)
                                } catch (e: Exception) {
                                    error = "TOTPシードの形式が正しくありません"
                                    return@TextButton
                                }
                            }
                            val item = NewItem(
                                title = title,
                                username = username.ifEmpty { null },
                                password = password.ifEmpty { null },
                                url = url.ifEmpty { null },
                                notes = notes.ifEmpty { null },
                                totpSeed = seed,
                                favorite = favorite,
                            )
                            scope.launch {
                                try {
                                    if (uuid == null) store.addItem(item)
                                    else store.updateItem(uuid, item)
                                    onDone()
                                } catch (e: Exception) {
                                    error = e.message
                                }
                            }
                        },
                    ) { Text("保存") }
                },
            )
        },
    ) { padding ->
        Column(
            Modifier
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            OutlinedTextField(title, { title = it }, label = { Text("タイトル") }, modifier = Modifier.fillMaxWidth())
            OutlinedTextField(username, { username = it }, label = { Text("ユーザー名") }, modifier = Modifier.fillMaxWidth())
            OutlinedTextField(
                password, { password = it },
                label = { Text("パスワード") },
                trailingIcon = {
                    TextButton(onClick = {
                        password = runCatching {
                            generatePassword(20u, true, true, false)
                        }.getOrDefault(password)
                    }) { Text("生成") }
                },
                modifier = Modifier.fillMaxWidth(),
            )
            if (password.isNotEmpty()) {
                val bits = entropyBits(password)
                Text(
                    "強度: ${strengthLabel(bits)}（${bits.toInt()}bit）",
                    color = strengthColor(bits),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
            OutlinedTextField(url, { url = it }, label = { Text("URL") }, modifier = Modifier.fillMaxWidth())
            OutlinedTextField(
                totp, { totp = it },
                label = { Text("TOTPシード（otpauth:// または base32）") },
                modifier = Modifier.fillMaxWidth(),
            )
            Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                Checkbox(checked = favorite, onCheckedChange = { favorite = it })
                Text("お気に入り")
            }
            OutlinedTextField(
                notes, { notes = it },
                label = { Text("メモ") },
                minLines = 3,
                modifier = Modifier.fillMaxWidth(),
            )
            error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
        }
    }
}

private fun entropyBits(pw: String): Double {
    var pool = 0
    if (pw.any { it.isLowerCase() }) pool += 26
    if (pw.any { it.isUpperCase() }) pool += 26
    if (pw.any { it.isDigit() }) pool += 10
    if (pw.any { !it.isLetterOrDigit() }) pool += 32
    return if (pool == 0) 0.0 else pw.length * log2(pool.toDouble())
}

private fun strengthLabel(bits: Double) = when {
    bits < 40 -> "弱い"
    bits < 70 -> "普通"
    else -> "強い"
}

private fun strengthColor(bits: Double) = when {
    bits < 40 -> Color(0xFFC62828)
    bits < 70 -> Color(0xFFEF6C00)
    else -> Color(0xFF2E7D32)
}
