package dev.twopassword

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val store = VaultStore(applicationContext)
        val clipboard = ClipboardHelper(applicationContext)
        setContent {
            MaterialTheme {
                Surface(modifier = Modifier.fillMaxSize()) {
                    when (store.state) {
                        VaultState.SETUP -> SetupScreen(store)
                        VaultState.LOCKED -> UnlockScreen(store)
                        VaultState.UNLOCKED -> {
                            var selected by remember { mutableStateOf<String?>(null) }
                            var editing by remember { mutableStateOf<String?>(null) }
                            var adding by remember { mutableStateOf(false) }
                            when {
                                adding || editing != null -> ItemEditScreen(
                                    store,
                                    if (adding) null else editing,
                                    onDone = { adding = false; editing = null },
                                )
                                selected != null -> ItemDetailScreen(
                                    store, selected!!, clipboard,
                                    onBack = { selected = null },
                                    onEdit = { editing = selected },
                                )
                                else -> VaultScreen(
                                    store,
                                    onSelect = { selected = it },
                                    onAdd = { adding = true },
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun SetupScreen(store: VaultStore) {
    var password by remember { mutableStateOf("") }
    var confirm by remember { mutableStateOf("") }
    var secretKey by remember { mutableStateOf<String?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        Text("2Password セットアップ", style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(24.dp))
        if (secretKey != null) {
            Text("Secret Key（安全な場所に保存してください）")
            Text(secretKey!!, style = MaterialTheme.typography.bodyMedium)
            Spacer(Modifier.height(16.dp))
            Button(onClick = { store.state; }) { Text("はじめる") }
            return@Column
        }
        OutlinedTextField(
            value = password, onValueChange = { password = it },
            label = { Text("アカウントパスワード（8文字以上）") },
            visualTransformation = PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(12.dp))
        OutlinedTextField(
            value = confirm, onValueChange = { confirm = it },
            label = { Text("確認") },
            visualTransformation = PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
            modifier = Modifier.fillMaxWidth(),
        )
        if (error != null) Text(error!!, color = MaterialTheme.colorScheme.error)
        Spacer(Modifier.height(16.dp))
        Button(
            enabled = password.length >= 8 && password == confirm,
            onClick = {
                scope.launch {
                    try { secretKey = store.setup(password) }
                    catch (e: Exception) { error = e.message }
                }
            },
        ) { Text("Vaultを作成") }
    }
}

@Composable
fun UnlockScreen(store: VaultStore) {
    var password by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    Column(
        modifier = Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        Text("2Password", style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(24.dp))
        OutlinedTextField(
            value = password, onValueChange = { password = it },
            label = { Text("パスワード") },
            visualTransformation = PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
            modifier = Modifier.fillMaxWidth(),
        )
        if (error != null) Text(error!!, color = MaterialTheme.colorScheme.error)
        Spacer(Modifier.height(16.dp))
        Button(onClick = {
            scope.launch {
                try { store.unlock(password) }
                catch (e: Exception) { error = "アンロックに失敗しました" }
            }
        }) { Text("アンロック") }
    }
}
