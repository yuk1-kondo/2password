package dev.twopassword

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import uniffi.twopassword_ffi.ItemDetail
import uniffi.twopassword_ffi.NewItem
import uniffi.twopassword_ffi.checkPasswordBreach
import uniffi.twopassword_ffi.totpCode

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ItemDetailScreen(
    store: VaultStore,
    uuid: String,
    clipboard: ClipboardHelper,
    onBack: () -> Unit,
    onEdit: () -> Unit,
) {
    var item by remember { mutableStateOf<ItemDetail?>(null) }
    var showPassword by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    LaunchedEffect(uuid) {
        item = store.detail(uuid)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {},
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "戻る")
                    }
                },
                actions = {
                    IconButton(onClick = {
                        val it = item ?: return@IconButton
                        scope.launch {
                            store.updateItem(
                                uuid,
                                NewItem(
                                    title = it.title,
                                    username = it.username,
                                    password = it.password,
                                    url = it.url,
                                    notes = it.notes,
                                    totpSeed = it.totpSeed,
                                    favorite = !it.favorite,
                                ),
                            )
                            item = store.detail(uuid)
                        }
                    }) {
                        Icon(
                            if (item?.favorite == true) Icons.Filled.Star else Icons.Filled.StarBorder,
                            "お気に入り",
                            tint = if (item?.favorite == true) Color(0xFFFFC107) else LocalContentColor.current,
                        )
                    }
                    IconButton(onClick = onEdit) { Icon(Icons.Default.Edit, "編集") }
                    IconButton(onClick = { confirmDelete = true }) {
                        Icon(Icons.Default.Delete, "削除")
                    }
                },
            )
        },
    ) { padding ->
        val it = item
        if (it == null) {
            Box(Modifier.padding(padding).fillMaxSize(), Alignment.Center) {
                CircularProgressIndicator()
            }
            return@Scaffold
        }
        Column(
            Modifier
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                ItemIcon(it.title, 48)
                Spacer(Modifier.width(14.dp))
                Column {
                    Text(it.title, style = MaterialTheme.typography.titleLarge)
                    if (it.favorite) {
                        Text("お気に入り", color = Color(0xFFFFC107), style = MaterialTheme.typography.labelSmall)
                    }
                }
            }
            Spacer(Modifier.height(20.dp))

            it.username?.let { v -> CopyRow("ユーザー名", v, clipboard) }
            it.password?.let { v ->
                FieldRow("パスワード") {
                    Text(
                        if (showPassword) v else "•".repeat(12),
                        fontFamily = FontFamily.Monospace,
                    )
                }
                Row {
                    IconButton(onClick = { showPassword = !showPassword }) {
                        Icon(
                            if (showPassword) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                            null,
                        )
                    }
                    IconButton(onClick = { clipboard.copy(v) }) {
                        Icon(Icons.Default.ContentCopy, "コピー")
                    }
                }
                BreachRow(v)
            }
            it.url?.let { v -> CopyRow("URL", v, clipboard) }
            it.totpSeed?.let { seed -> TotpRow(seed, clipboard) }
            it.notes?.takeIf { n -> n.isNotEmpty() }?.let { v ->
                FieldRow("メモ") { Text(v) }
                HorizontalDivider(Modifier.padding(vertical = 8.dp))
            }
        }
    }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text("このアイテムを削除しますか？") },
            text = { Text("削除は同期で他の端末にも伝播します。") },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = false
                    scope.launch { store.removeItem(uuid); onBack() }
                }) { Text("削除", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(onClick = { confirmDelete = false }) { Text("キャンセル") }
            },
        )
    }
}

@Composable
private fun FieldRow(label: String, content: @Composable () -> Unit) {
    Column(Modifier.padding(vertical = 8.dp)) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        content()
    }
}

@Composable
private fun CopyRow(label: String, value: String, clipboard: ClipboardHelper) {
    Row(
        Modifier.fillMaxWidth().padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text(value)
        }
        IconButton(onClick = { clipboard.copy(value) }) {
            Icon(Icons.Default.ContentCopy, "コピー")
        }
    }
    HorizontalDivider()
}

@Composable
private fun BreachRow(password: String) {
    var result by remember { mutableStateOf<Result<UInt>?>(null) }
    LaunchedEffect(Unit) {
        result = runCatching { checkPasswordBreach(password) }
    }
    Column(Modifier.padding(vertical = 8.dp)) {
        Text("漏洩チェック", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        when {
            result == null -> Text("確認中…", style = MaterialTheme.typography.bodySmall)
            result!!.isSuccess && result!!.getOrNull() == 0u ->
                Text("✓ 漏洩は見つかりませんでした", color = Color(0xFF2E7D32))
            result!!.isSuccess ->
                Text("⚠ ${result!!.getOrNull()} 件の漏洩で見つかりました — 変更を推奨", color = MaterialTheme.colorScheme.error)
            else -> Text("チェックできませんでした（オフライン?）", style = MaterialTheme.typography.bodySmall)
        }
    }
    HorizontalDivider()
}

@Composable
private fun TotpRow(seed: String, clipboard: ClipboardHelper) {
    var code by remember { mutableStateOf("------") }
    var remaining by remember { mutableIntStateOf(30) }
    LaunchedEffect(seed) {
        while (true) {
            val secs = System.currentTimeMillis() / 1000
            code = runCatching {
                totpCode(seed, secs.toULong())
            }.getOrDefault("------")
            remaining = (30 - secs % 30).toInt()
            delay(1000)
        }
    }
    Row(
        Modifier.fillMaxWidth().padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text("ワンタイムパスワード", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Text("${code.take(3)} ${code.drop(3)}", fontFamily = FontFamily.Monospace)
        }
        LinearProgressIndicator(
            progress = { remaining / 30f },
            modifier = Modifier.width(48.dp),
        )
        Text("$remaining", Modifier.padding(horizontal = 8.dp), style = MaterialTheme.typography.labelSmall)
        IconButton(onClick = { clipboard.copy(code.replace(" ", "")) }) {
            Icon(Icons.Default.ContentCopy, "コピー")
        }
    }
    HorizontalDivider()
}
