package dev.twopassword

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Android Keystore-backed AES-GCM keys (spec §8.2).
 * - SK key: hardware-bound, no user auth — replaces the plaintext device.key.
 * - AUK key: requires biometric/device-credential within the last 30s —
 *   cryptographically gates the biometric unlock path.
 */
object KeystoreHelper {
    private const val PROVIDER = "AndroidKeyStore"
    private const val SK_KEY = "twopassword_sk"
    private const val AUK_KEY = "twopassword_auk"
    private const val IV_LEN = 12
    private const val TAG_BITS = 128

    private fun keystore() = KeyStore.getInstance(PROVIDER).apply { load(null) }

    private fun getOrCreate(name: String, requireAuth: Boolean): SecretKey {
        (keystore().getKey(name, null) as? SecretKey)?.let { return it }
        val builder = KeyGenParameterSpec.Builder(
            name,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
        if (requireAuth) {
            builder
                .setUserAuthenticationRequired(true)
                .setUserAuthenticationParameters(
                    30,
                    KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
                )
        }
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, PROVIDER)
        gen.init(builder.build())
        return gen.generateKey()
    }

    /** Storage key for the device Secret Key (no user-auth requirement). */
    fun skKey(): SecretKey = getOrCreate(SK_KEY, requireAuth = false)

    /** Biometric-bound key for the wrapped AUK (auth within 30s required). */
    fun aukKey(): SecretKey = getOrCreate(AUK_KEY, requireAuth = true)

    /** Returns iv||ciphertext. */
    fun encrypt(cipher: Cipher, plain: ByteArray): ByteArray = cipher.iv + cipher.doFinal(plain)

    fun encryptCipher(key: SecretKey): Cipher =
        Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key) }

    fun decryptCipher(key: SecretKey, blob: ByteArray): Cipher =
        Cipher.getInstance("AES/GCM/NoPadding").apply {
            init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_BITS, blob.copyOfRange(0, IV_LEN)))
        }

    fun decrypt(key: SecretKey, blob: ByteArray): ByteArray =
        decryptCipher(key, blob).doFinal(blob.copyOfRange(IV_LEN, blob.size))
}
