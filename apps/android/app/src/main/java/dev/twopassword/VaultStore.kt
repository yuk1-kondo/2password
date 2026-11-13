package dev.twopassword

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import javax.crypto.Cipher
import uniffi.twopassword_ffi.*

enum class VaultState { SETUP, LOCKED, UNLOCKED }

/**
 * App state holding the unlocked Vault. Android counterpart of the Swift
 * VaultStore — Secret Key is encrypted with an Android Keystore AES-GCM key
 * (spec §8.2); the biometric path wraps the AUK behind a user-auth key.
 */
class VaultStore(ctx: Context) {
    private val vaultDir = File(ctx.filesDir, "vault").absolutePath
    private val keyFile = File(ctx.filesDir, "device.key")
    private val aukFile = File(ctx.filesDir, "auk.bin")

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

    /** Keystore-encrypted Secret Key; migrates legacy plaintext on first read. */
    private fun loadSk(): String {
        val raw = keyFile.readBytes()
        return runCatching {
            String(KeystoreHelper.decrypt(KeystoreHelper.skKey(), raw))
        }.getOrElse {
            // Legacy plaintext device.key — re-encrypt under Keystore now.
            val sk = String(raw)
            runCatching {
                val blob = KeystoreHelper.encrypt(
                    KeystoreHelper.encryptCipher(KeystoreHelper.skKey()),
                    sk.toByteArray(),
                )
                keyFile.writeBytes(blob)
            }
            sk
        }
    }

    private var pendingVault: Vault? = null

    /** Generate + persist the Secret Key first, then create the vault (atomic order).
     *  State stays SETUP so the SK can be shown until begin() is called. */
    suspend fun setup(password: String): String = withContext(Dispatchers.IO) {
        val sk = newSecretKey()
        val blob = KeystoreHelper.encrypt(KeystoreHelper.encryptCipher(KeystoreHelper.skKey()), sk.toByteArray())
        keyFile.writeBytes(blob)
        createVault(vaultDir, "user", password, sk)
        pendingVault = unlockVault(vaultDir, password, sk)
        sk
    }

    /** Finish setup after the user has seen the Secret Key. */
    fun begin() {
        vault = pendingVault
        pendingVault = null
        state = VaultState.UNLOCKED
        reload()
    }

    suspend fun unlock(password: String) = withContext(Dispatchers.IO) {
        vault = unlockVault(vaultDir, password, loadSk())
        state = VaultState.UNLOCKED
        reload()
    }

    /** Emergency-kit restore: validate before persisting the key (spec §7). */
    suspend fun unlockWithSecretKey(password: String, secretKey: String) = withContext(Dispatchers.IO) {
        val v = unlockVault(vaultDir, password, secretKey) // throws on bad key
        val blob = KeystoreHelper.encrypt(
            KeystoreHelper.encryptCipher(KeystoreHelper.skKey()),
            secretKey.toByteArray(),
        )
        keyFile.writeBytes(blob)
        vault = v
        state = VaultState.UNLOCKED
        reload()
    }

    /** Encrypt cipher for the biometric-bound AUK key — needs a BiometricPrompt gate. */
    fun aukEncryptCipher(): Cipher =
        KeystoreHelper.encryptCipher(KeystoreHelper.aukKey())

    /** Whether a biometric-wrapped AUK exists. */
    fun hasBiometricEnrollment(): Boolean = aukFile.exists()

    /** Decrypt cipher for the stored AUK blob (iv is prepended). */
    fun aukDecryptCipher(): Cipher =
        KeystoreHelper.decryptCipher(KeystoreHelper.aukKey(), aukFile.readBytes())

    /** Wrap + store the freshly-derived AUK inside the biometric cipher. */
    suspend fun enrollAuk(password: String, cipher: Cipher) = withContext(Dispatchers.IO) {
        val auk = deriveAukB64(vaultDir, password, loadSk())
        aukFile.writeBytes(KeystoreHelper.encrypt(cipher, auk.toByteArray()))
    }

    /** Biometric unlock: cipher already auth'd by BiometricPrompt. */
    suspend fun unlockWithBiometric(cipher: Cipher) = withContext(Dispatchers.IO) {
        val blob = aukFile.readBytes()
        val aukB64 = String(cipher.doFinal(blob.copyOfRange(12, blob.size)))
        vault = unlockVaultWithAuk(vaultDir, aukB64)
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
