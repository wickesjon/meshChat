package org.meshchat.identity

import android.content.Context

/** One staff credential encrypted by its own OS wrapping key, bound to this
 * device identity generation. No unwrapped key/session survives an operation. */
class StaffKeyVault internal constructor(private val files: IdentityStorage, private val protection: IdentityProtection) : IdentityResetStore {
    private val foregroundLock=Any()
    @Volatile private var foreground=true
    private var foregroundRevision=0L
    fun background() {synchronized(foregroundLock) {foregroundRevision++;foreground=false}}
    fun requestForeground():Long=synchronized(foregroundLock) {++foregroundRevision}
    fun resumeForeground(revision:Long) {synchronized(foregroundLock) {if(revision==foregroundRevision)foreground=true}}
    fun signingAllowed():Boolean=foreground
    /** Serialize the short native submission with background closure. A send
     * already submitted cannot be recalled; no new submission starts after it. */
    fun submitForeground(submit:()->Boolean):Boolean=synchronized(foregroundLock) {if(foreground)submit() else false}
    private fun requireForeground() {if(!foreground)throw uniffi.meshchat_core.OrganizerException.Authority()}
    companion object {
        fun android(context: Context): StaffKeyVault = StaffKeyVault(
            AndroidIdentityStorage(context.applicationContext,"meshchat-staff-v1"),
            AndroidIdentityProtection(context.applicationContext,"org.meshchat.staff.wrapping.v1",1..512))
    }
    private fun refuse(failure: IdentityFailure): Nothing = throw IdentityProviderException(failure)
    fun present(): Boolean = IdentityProvider.withOperation {
        files.exists(IdentityFile.ENVELOPE)||files.exists(IdentityFile.JOURNAL)||protection.exists()
    }
    private fun header(generation: ByteArray): ByteArray {
        if(generation.size!=16)refuse(IdentityFailure.INVALID_INPUT)
        return byteArrayOf(1)+generation
    }
    /** Validation runs before any mutation. Atomic replacement preserves the old
     * encrypted credential if validation, encryption or the file write fails. */
    fun importConfirmed(generation: ByteArray, candidate: ByteArray, validate: (String)->Unit) = IdentityProvider.withOperation {
        try {
            protection.requireUnlocked();requireForeground()
            if(candidate.size !in 1..512)refuse(IdentityFailure.INVALID_INPUT)
            val aad=header(generation)
            if(files.exists(IdentityFile.JOURNAL))refuse(IdentityFailure.RECOVERY_REQUIRED)
            validate(candidate.toString(Charsets.UTF_8));requireForeground()
            val existing=files.exists(IdentityFile.ENVELOPE)
            if(existing && !protection.exists())refuse(IdentityFailure.INVALIDATED)
            if(!existing) {
                if(protection.exists())refuse(IdentityFailure.RECOVERY_REQUIRED)
                files.write(IdentityFile.JOURNAL,byteArrayOf(1));protection.create()
            }
            val encrypted=protection.seal(candidate,aad)
            files.write(IdentityFile.ENVELOPE,aad+encrypted)
            if(!existing)files.delete(IdentityFile.JOURNAL)
        } finally { candidate.fill(0) }
    }
    fun <T> access(generation: ByteArray, work: (String)->T): T = IdentityProvider.withOperation {
        protection.requireUnlocked();requireForeground()
        if(files.exists(IdentityFile.JOURNAL))refuse(IdentityFailure.RECOVERY_REQUIRED)
        if(!files.exists(IdentityFile.ENVELOPE))refuse(if(protection.exists())IdentityFailure.RECOVERY_REQUIRED else IdentityFailure.MISSING)
        if(!protection.exists())refuse(IdentityFailure.INVALIDATED)
        val aad=header(generation);val encoded=files.read(IdentityFile.ENVELOPE)
        if(encoded.size !in 46..557 || !encoded.copyOfRange(0,17).contentEquals(aad))refuse(IdentityFailure.INVALID_INPUT)
        val plain=protection.open(encoded.copyOfRange(17,encoded.size),aad)
        try { if(plain.size !in 1..512)refuse(IdentityFailure.INVALID_INPUT);val result=work(plain.toString(Charsets.UTF_8));protection.requireUnlocked();requireForeground();result }
        finally {plain.fill(0)}
    }
    fun forget() = IdentityProvider.withOperation {
        protection.requireUnlocked()
        files.write(IdentityFile.JOURNAL,byteArrayOf(2))
        protection.delete();files.delete(IdentityFile.ENVELOPE);files.delete(IdentityFile.JOURNAL)
    }
    override fun clearIdentityState() { IdentityProvider.requireResetInProgress();forget() }
}
