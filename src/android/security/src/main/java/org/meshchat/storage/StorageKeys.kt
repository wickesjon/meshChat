package org.meshchat.storage

import uniffi.meshchat_core.legacyStorageKey
import org.meshchat.identity.IdentityFailure
import org.meshchat.identity.IdentityProviderException

/** Version 2 wraps key(32), SQLCipher salt(16), reserved zero bytes(16).
 * Never convert secrets to immutable Strings or retain them between operations. */
internal object StorageKeys {
    private fun invalid(): Nothing = throw IdentityProviderException(IdentityFailure.INVALID_INPUT)

    fun rawInput(material: ByteArray): ByteArray {
        if (material.size != 64 || (48..63).any { material[it] != 0.toByte() }) invalid()
        val hex = "0123456789abcdef".toByteArray(Charsets.US_ASCII)
        return ByteArray(99).also { out ->
            out[0] = 'x'.code.toByte(); out[1] = 39; out[98] = 39
            repeat(48) { i ->
                val n = material[i].toInt() and 255
                out[2 + i * 2] = hex[n ushr 4]; out[3 + i * 2] = hex[n and 15]
            }
        }
    }

    /** Library PBKDF2-SHA512, truncated to the AES-256 key size.
     * PBEKeySpec is unsuitable: the legacy secret is arbitrary binary, not text.
     * Iterations are the pinned, verified SQLCipher v4 setting, never reduced. */
    fun deriveLegacy(passphrase: ByteArray, salt: ByteArray): ByteArray {
        if (passphrase.size != 64 || salt.size != 16) invalid()
        val derived = legacyStorageKey(passphrase, salt)
        try {
            if (derived.size != 32) invalid()
            return ByteArray(64).also { result ->
                derived.copyInto(result)
                salt.copyInto(result, 32)
            }
        } finally {
            derived.fill(0)
        }
    }
}
