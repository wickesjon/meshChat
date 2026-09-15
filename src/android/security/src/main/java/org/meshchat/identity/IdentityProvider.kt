package org.meshchat.identity

import android.content.Context
import uniffi.meshchat_core.IdentityException
import uniffi.meshchat_core.IdentityKeySession
import uniffi.meshchat_core.PublicIdentity

/** Public failures deliberately omit provider messages, paths and key material. */
enum class IdentityFailure { LOCKED, UNAVAILABLE, INVALIDATED, MISSING, INVALID_INPUT, RECOVERY_REQUIRED, STALE_HANDLE, PROVIDER }
class IdentityProviderException(val failure: IdentityFailure) : Exception(failure.name)
enum class WrappingProtection { UNKNOWN, SOFTWARE, HARDWARE }
data class IdentityCapabilities(val wrapping: WrappingProtection, val softwareCurves: Boolean = true)
class IdentityHandle internal constructor(internal val generation: ByteArray)
data class IdentityInfo(val handle: IdentityHandle, val identity: PublicIdentity, val capabilities: IdentityCapabilities)

/** Must durably and idempotently invalidate all prior identity-bound pins/state.
 * MC-018's store implements this barrier; a failure leaves the reset journal and
 * blocks all key use. It must not call back into the provider. */
fun interface IdentityResetStore { fun clearIdentityState() }
internal enum class IdentityFile { ENVELOPE, JOURNAL }
internal interface IdentityStorage {
    fun exists(file: IdentityFile): Boolean
    fun read(file: IdentityFile): ByteArray
    fun write(file: IdentityFile, bytes: ByteArray)
    fun delete(file: IdentityFile)
    fun hasArtifacts(): Boolean
}
internal interface IdentityProtection {
    fun requireUnlocked()
    fun exists(): Boolean
    fun create()
    fun delete()
    fun random(size: Int): ByteArray
    fun seal(plain: ByteArray, aad: ByteArray): ByteArray
    fun open(cipher: ByteArray, aad: ByteArray): ByteArray
    fun capabilities(): IdentityCapabilities
}

/** All operations serialize with reset. No application method exports seeds or
 * retains an unlocked Rust session; only ciphertext is persisted. */
class IdentityProvider internal constructor(
    private val storage: IdentityStorage,
    private val protection: IdentityProtection,
    private val state: IdentityResetStore,
) {
    companion object {
        private val operationLock = Any()
        private var resettingState = false
        internal fun <T> withOperation(work: () -> T): T = synchronized(operationLock) { work() }
        internal fun requireResetInProgress() {
            if (!resettingState) throw IdentityProviderException(IdentityFailure.RECOVERY_REQUIRED)
        }
        fun android(context: Context, state: IdentityResetStore): IdentityProvider = IdentityProvider(
            AndroidIdentityStorage(context.applicationContext), AndroidIdentityProtection(context.applicationContext), state,
        )
    }
    private fun refuse(failure: IdentityFailure): Nothing = throw IdentityProviderException(failure)
    private inline fun <T> guarded(work: () -> T): T = synchronized(operationLock) { try { work() }
        catch (error: IdentityProviderException) { throw error }
        catch (_: IdentityException.InvalidInput) { refuse(IdentityFailure.INVALID_INPUT) }
        catch (_: IdentityException.InvalidPeer) { refuse(IdentityFailure.INVALID_INPUT) }
        catch (_: IdentityException.Unavailable) { refuse(IdentityFailure.UNAVAILABLE) }
        catch (_: Exception) { refuse(IdentityFailure.PROVIDER) } }

    @Synchronized fun create(): IdentityInfo = guarded {
        protection.requireUnlocked()
        if (storage.hasArtifacts() || protection.exists()) refuse(IdentityFailure.RECOVERY_REQUIRED)
        storage.write(IdentityFile.JOURNAL, byteArrayOf(1))
        provision()
        storage.delete(IdentityFile.JOURNAL)
        loadInternal()
    }
    @Synchronized fun load(): IdentityInfo = guarded { loadInternal() }
    private fun checkAvailable() {
        protection.requireUnlocked()
        if (storage.exists(IdentityFile.JOURNAL)) refuse(IdentityFailure.RECOVERY_REQUIRED)
        if (!storage.exists(IdentityFile.ENVELOPE)) {
            refuse(if (protection.exists() || storage.hasArtifacts()) IdentityFailure.RECOVERY_REQUIRED else IdentityFailure.MISSING)
        }
        if (!protection.exists()) refuse(IdentityFailure.INVALIDATED)
    }
    private fun header(encoded: ByteArray): ByteArray {
        if (encoded.size !in 18..1024 || encoded[0] != 1.toByte()) refuse(IdentityFailure.INVALID_INPUT)
        return encoded.copyOfRange(0, 17)
    }
    private fun <T> unlocked(handle: IdentityHandle?, work: (IdentityKeySession) -> T): T {
        checkAvailable()
        val encoded = storage.read(IdentityFile.ENVELOPE)
        val prefix = header(encoded)
        val generation = prefix.copyOfRange(1, 17)
        if (handle != null && !generation.contentEquals(handle.generation)) refuse(IdentityFailure.STALE_HANDLE)
        val material = protection.open(encoded.copyOfRange(17, encoded.size), prefix)
        try {
            if (material.size != 64) refuse(IdentityFailure.INVALID_INPUT)
            val session = IdentityKeySession.importUnlocked(material, generation)
            try {
                val result = work(session)
                try { protection.requireUnlocked() }
                catch (error: Exception) { if (result is ByteArray) result.fill(0); throw error }
                return result
            }
            finally { try { session.invalidate() } finally { session.close() } }
        } finally { material.fill(0) }
    }
    private fun loadInternal(): IdentityInfo = unlocked(null) { session ->
        val public = session.publicIdentity()
        IdentityInfo(IdentityHandle(public.generation.copyOf()), public, protection.capabilities())
    }
    @Synchronized fun publicIdentity(handle: IdentityHandle): PublicIdentity = guarded { unlocked(handle) { it.publicIdentity() } }
    @Synchronized fun sign(handle: IdentityHandle, transcript: ByteArray): ByteArray = guarded {
        if (transcript.size > 2048) refuse(IdentityFailure.INVALID_INPUT)
        unlocked(handle) { it.sign(transcript) }
    }
    @Synchronized fun agree(handle: IdentityHandle, peer: ByteArray): ByteArray = guarded {
        if (peer.size != 32) refuse(IdentityFailure.INVALID_INPUT)
        unlocked(handle) { it.agree(peer) }
    }
    /** Recovery is explicit. Journal becomes durable before touching pins or
     * keys; retrying after any partial failure clears state again and rotates. */
    @Synchronized fun reset(): IdentityInfo = guarded {
        protection.requireUnlocked()
        storage.write(IdentityFile.JOURNAL, byteArrayOf(2))
        if (resettingState) refuse(IdentityFailure.RECOVERY_REQUIRED)
        resettingState = true
        try { state.clearIdentityState() } finally { resettingState = false }
        protection.delete()
        storage.delete(IdentityFile.ENVELOPE)
        provision()
        storage.delete(IdentityFile.JOURNAL)
        loadInternal()
    }
    private fun provision() {
        protection.create()
        val generation = protection.random(16)
        if (generation.size != 16 || generation.all { it == 0.toByte() }) refuse(IdentityFailure.PROVIDER)
        val prefix = byteArrayOf(1) + generation
        val material = protection.random(64)
        try {
            if (material.size != 64) refuse(IdentityFailure.PROVIDER)
            val encoded = prefix + protection.seal(material, prefix)
            if (encoded.size > 1024) refuse(IdentityFailure.PROVIDER)
            storage.write(IdentityFile.ENVELOPE, encoded)
        } finally { material.fill(0) }
    }
}
