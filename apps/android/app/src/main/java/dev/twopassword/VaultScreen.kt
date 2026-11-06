package dev.twopassword

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import uniffi.twopassword_ffi.ItemSummary

enum class ItemCategory(val label: String) {
    ALL("すべて"), FAVORITES("お気に入り"), LOGINS("ログイン"), NOTES("ノート");

    fun matches(item: ItemSummary): Boolean = when (this) {
        ALL -> true
        FAVORITES -> item.favorite
        LOGINS -> item.url != null || item.username != null
        NOTES -> item.url == null && item.username == null
    }
}

/** Deterministic monogram color — no favicon fetching (privacy). */
fun monogramColor(title: String): Color {
    val hue = (title.hashCode() % 360 + 360) % 360
    return Color.hsv(hue.toFloat(), 0.5f, 0.8f)
}

@Composable
fun ItemIcon(title: String, size: Int = 36) {
    Box(
        modifier = Modifier
            .size(size.dp)
            .background(monogramColor(title), CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            title.take(1).uppercase(),
            color = Color.White,
            style = MaterialTheme.typography.bodyLarge,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun VaultScreen(
    store: VaultStore,
    onSelect: (String) -> Unit,
    onAdd: () -> Unit,
) {
    var query by remember { mutableStateOf("") }
    var category by remember { mutableStateOf(ItemCategory.ALL) }
    var menuOpen by remember { mutableStateOf(false) }

    val items = store.items
        .filter { category.matches(it) }
        .filter {
            query.isEmpty() ||
                it.title.contains(query, ignoreCase = true) ||
                (it.username ?: "").contains(query, ignoreCase = true) ||
                (it.url ?: "").contains(query, ignoreCase = true)
        }
        .sortedBy { it.title.lowercase() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("2Password") },
                actions = {
                    Text("${items.size}")
                    IconButton(onClick = { menuOpen = true }) {
                        Icon(Icons.Default.MoreVert, null)
                    }
                    DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                        DropdownMenuItem(
                            text = { Text("ロック") },
                            leadingIcon = { Icon(Icons.Default.Lock, null) },
                            onClick = { menuOpen = false; store.lock() },
                        )
                    }
                },
            )
        },
        floatingActionButton = {
            FloatingActionButton(onClick = onAdd) {
                Icon(Icons.Default.Add, "追加")
            }
        },
    ) { padding ->
        Column(Modifier.padding(padding)) {
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                label = { Text("検索") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
            )
            Row(
                Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                ItemCategory.entries.forEach { cat ->
                    FilterChip(
                        selected = category == cat,
                        onClick = { category = cat },
                        label = { Text(cat.label) },
                    )
                }
            }
            LazyColumn {
                items(items, key = { it.uuid }) { item ->
                    ListItem(
                        headlineContent = { Text(item.title) },
                        supportingContent = { Text(item.username ?: item.url ?: "") },
                        leadingContent = { ItemIcon(item.title) },
                        trailingContent = {
                            if (item.favorite) {
                                Icon(Icons.Default.Star, null, tint = Color(0xFFFFC107))
                            }
                        },
                        modifier = Modifier.clickable { onSelect(item.uuid) },
                    )
                    HorizontalDivider()
                }
            }
        }
    }
}
