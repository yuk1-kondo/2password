package dev.twopassword

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.twopassword_ffi.*

enum class VaultState { SETUP, LOCKED, UNLOCKED }

/**
 * App state holding the unlocked Vault. Android counterpart of the Swift
 * VaultStore — Secret Key is stored in app-private storage for now
 * (Keystore+BiometricPrompt upgrade is a spec §8.2 follow-up).
 */
class VaultStore(ctx: Context) {
    private val vaultDir = File(ctx.filesDir, "vault").absolutePath
    private val keyFile = File(ctx.filesDir, "device.key")

    var state by mutableStateOf(VaultState.SETUP)
        private set
    var vault: Vault? = null
        private set
    var items by mutableStateOf(listOf<ItemSummary>())
        private set

    init {
        state = if (File(vaultDir, "profile.json").exists()
            && File(vaultDir, "keyset.json").exists()
        ) VaultState.LOCKED else VaultState.SETUP
    }

    /** Generate + persist the Secret Key first, then create the vault (atomic order). */
    suspend fun setup(password: String): String = withContext(Dispatchers.IO) {
        val sk = newSecretKey()
        keyFile.writeText(sk)
        createVault(vaultDir, "user", password, sk)
        vault = unlockVault(vaultDir, password, sk)
        state = VaultState.UNLOCKED
        reload()
        sk
    }

    suspend fun unlock(password: String) = withContext(Dispatchers.IO) {
        val sk = keyFile.readText()
        vault = unlockVault(vaultDir, password, sk)
        state = VaultState.UNLOCKED
        reload()
    }

    fun lock() {
        vault?.close()
        vault = null
        items = emptyList()
        state = VaultState.LOCKED
    }

    fun reload() {
        items = vault?.listItems() ?: emptyList()
    }

    suspend fun addItem(item: NewItem) = withContext(Dispatchers.IO) {
        vault?.addItem(item)
        reload()
    }

    suspend fun updateItem(uuid: String, item: NewItem) = withContext(Dispatchers.IO) {
        vault?.updateItem(uuid, item)
        reload()
    }

    suspend fun removeItem(uuid: String) = withContext(Dispatchers.IO) {
        vault?.removeItem(uuid)
        reload()
    }

    suspend fun detail(uuid: String): ItemDetail? = withContext(Dispatchers.IO) {
        try { vault?.getItem(uuid) } catch (e: Exception) { null }
    }
}
