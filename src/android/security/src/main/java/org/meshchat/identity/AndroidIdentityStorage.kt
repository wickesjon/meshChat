package org.meshchat.identity

import android.app.KeyguardManager
import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.security.keystore.UserNotAuthenticatedException
import android.system.Os
import android.system.OsConstants
import android.util.AtomicFile
import java.io.File
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec

internal class AndroidIdentityStorage(context: Context) : IdentityStorage {
    private val folder = File(context.noBackupFilesDir, "meshchat-identity-v1")
    private fun path(file: IdentityFile) = File(folder, if (file == IdentityFile.ENVELOPE) "identity.enc" else "operation")
    override fun exists(file: IdentityFile): Boolean = path(file).exists() || File(path(file).path + ".bak").exists()
    override fun hasArtifacts(): Boolean = folder.exists()
    override fun read(file: IdentityFile): ByteArray {
        // File contents are bounded before allocation, including interrupted AtomicFile recovery.
        val atomic = AtomicFile(path(file))
        return atomic.openRead().use { input ->
            if (input.channel.size() > 1024) throw IdentityProviderException(IdentityFailure.INVALID_INPUT)
            val buffer = ByteArray(1025)
            var used = 0
            while (used < buffer.size) {
                val n = input.read(buffer, used, buffer.size - used)
                if (n < 0) break
                used += n
            }
            if (used > 1024) throw IdentityProviderException(IdentityFailure.INVALID_INPUT)
            buffer.copyOf(used)
        }
    }
    private fun syncDirectory(directory: File) {
        val descriptor = Os.open(directory.path, OsConstants.O_RDONLY, 0)
        try { Os.fsync(descriptor) } finally { Os.close(descriptor) }
    }
    override fun write(file: IdentityFile, bytes: ByteArray) {
        check(bytes.size <= 1024)
        if (!folder.exists()) {
            check(folder.mkdir())
            syncDirectory(checkNotNull(folder.parentFile))
        }
        val atomic = AtomicFile(path(file))
        val stream = atomic.startWrite()
        try { stream.write(bytes); stream.fd.sync(); atomic.finishWrite(stream) }
        catch (error: Exception) { atomic.failWrite(stream); throw error }
        syncDirectory(folder)
    }
    override fun delete(file: IdentityFile) {
        if (!folder.exists()) return
        AtomicFile(path(file)).delete()
        check(!exists(file))
        syncDirectory(folder)
    }
}

internal class AndroidIdentityProtection(private val context: Context) : IdentityProtection {
    private val alias = "org.meshchat.identity.wrapping.v1"
    private val keyguard get() = context.getSystemService(KeyguardManager::class.java)
    private fun store() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    override fun requireUnlocked() {
        if (keyguard.isDeviceLocked) throw IdentityProviderException(IdentityFailure.LOCKED)
    }
    override fun exists(): Boolean = store().containsAlias(alias)
    override fun create() {
        requireUnlocked()
        if (exists()) throw IdentityProviderException(IdentityFailure.RECOVERY_REQUIRED)
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setKeySize(256).setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).setUnlockedDeviceRequired(true).build())
        generator.generateKey()
    }
    override fun delete() { requireUnlocked(); store().deleteEntry(alias) }
    override fun random(size: Int): ByteArray = ByteArray(size).also { SecureRandom().nextBytes(it) }
    private fun key(): SecretKey = store().getKey(alias, null) as? SecretKey
        ?: throw IdentityProviderException(IdentityFailure.INVALIDATED)
    private inline fun <T> operation(work: () -> T): T {
        requireUnlocked()
        return try { work() }
        catch (_: UserNotAuthenticatedException) { throw IdentityProviderException(IdentityFailure.LOCKED) }
        catch (_: KeyPermanentlyInvalidatedException) { throw IdentityProviderException(IdentityFailure.INVALIDATED) }
        catch (_: AEADBadTagException) { throw IdentityProviderException(IdentityFailure.INVALID_INPUT) }
    }
    override fun seal(plain: ByteArray, aad: ByteArray): ByteArray = operation {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key())
        cipher.updateAAD(aad)
        check(cipher.iv.size == 12)
        cipher.iv + cipher.doFinal(plain)
    }
    override fun open(cipher: ByteArray, aad: ByteArray): ByteArray = operation {
        if (cipher.size != 12 + 64 + 16) throw IdentityProviderException(IdentityFailure.INVALID_INPUT)
        val engine = Cipher.getInstance("AES/GCM/NoPadding")
        engine.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, cipher.copyOfRange(0, 12)))
        engine.updateAAD(aad)
        engine.doFinal(cipher, 12, cipher.size - 12)
    }
    override fun capabilities(): IdentityCapabilities = operation {
        val info = SecretKeyFactory.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").getKeySpec(key(), KeyInfo::class.java) as KeyInfo
        @Suppress("DEPRECATION") // Reports the actual provider result on API29+, never assumes StrongBox.
        val hardware = info.isInsideSecureHardware
        IdentityCapabilities(if (hardware) WrappingProtection.HARDWARE else WrappingProtection.SOFTWARE)
    }
}
