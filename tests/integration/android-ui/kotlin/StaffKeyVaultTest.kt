package org.meshchat.identity

import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec
import java.security.SecureRandom
import org.junit.Assert.*
import org.junit.Test

class StaffKeyVaultTest {
    private class Files:IdentityStorage {
        val values=mutableMapOf<IdentityFile,ByteArray>();var failWrite=false
        override fun exists(file:IdentityFile)=values.containsKey(file)
        override fun read(file:IdentityFile)=checkNotNull(values[file]).copyOf()
        override fun write(file:IdentityFile,bytes:ByteArray){if(failWrite)throw IllegalStateException("synthetic write failure");values[file]=bytes.copyOf()}
        override fun delete(file:IdentityFile){values.remove(file)}
        override fun hasArtifacts()=values.isNotEmpty()
    }
    private class Protection:IdentityProtection {
        var locked=false;var key:ByteArray?=null;var failSeal=false;var lastPlain:ByteArray?=null
        override fun requireUnlocked(){if(locked)throw IdentityProviderException(IdentityFailure.LOCKED)}
        override fun exists()=key!=null
        override fun create(){key=random(32)}
        override fun delete(){key?.fill(0);key=null}
        override fun random(size:Int)=ByteArray(size).also {SecureRandom().nextBytes(it)}
        override fun seal(plain:ByteArray,aad:ByteArray):ByteArray {
            if(failSeal)throw IllegalStateException("synthetic seal failure")
            val iv=random(12);val c=Cipher.getInstance("AES/GCM/NoPadding")
            c.init(Cipher.ENCRYPT_MODE,SecretKeySpec(checkNotNull(key),"AES"),GCMParameterSpec(128,iv));c.updateAAD(aad)
            return iv+c.doFinal(plain)
        }
        override fun open(cipher:ByteArray,aad:ByteArray):ByteArray {
            val c=Cipher.getInstance("AES/GCM/NoPadding")
            c.init(Cipher.DECRYPT_MODE,SecretKeySpec(checkNotNull(key),"AES"),GCMParameterSpec(128,cipher.copyOfRange(0,12)));c.updateAAD(aad)
            return c.doFinal(cipher,12,cipher.size-12).also {lastPlain=it}
        }
        override fun capabilities()=IdentityCapabilities(WrappingProtection.SOFTWARE)
    }
    @Test fun encrypted_generation_bound_lifecycle_and_failure_preservation() {
        val files=Files();val protection=Protection();val vault=StaffKeyVault(files,protection);val generation=ByteArray(16){4}
        val candidate="synthetic-private-candidate".toByteArray()
        vault.importConfirmed(generation,candidate) {assertTrue(it.startsWith("synthetic"))}
        assertTrue(candidate.all {it==0.toByte()});assertTrue(vault.present())
        assertTrue(!files.read(IdentityFile.ENVELOPE).toString(Charsets.UTF_8).contains("synthetic-private-candidate"))
        assertEquals(27,vault.access(generation){it.length})
        assertTrue(checkNotNull(protection.lastPlain).all {it==0.toByte()})
        val encrypted=files.read(IdentityFile.ENVELOPE)
        val bad="invalid".toByteArray()
        assertThrows(IllegalArgumentException::class.java) {vault.importConfirmed(generation,bad) {throw IllegalArgumentException("rejected")}}
        assertTrue(bad.all {it==0.toByte()});assertArrayEquals(encrypted,files.read(IdentityFile.ENVELOPE))
        protection.failSeal=true
        assertThrows(IllegalStateException::class.java) {vault.importConfirmed(generation,"replacement".toByteArray()) {}}
        assertArrayEquals(encrypted,files.read(IdentityFile.ENVELOPE));protection.failSeal=false
        files.failWrite=true
        assertThrows(IllegalStateException::class.java) {vault.importConfirmed(generation,"replacement".toByteArray()) {}}
        files.failWrite=false;assertArrayEquals(encrypted,files.read(IdentityFile.ENVELOPE))
        assertThrows(IdentityProviderException::class.java) {vault.access(ByteArray(16){5}) {error("wrong generation reached key")}}
        protection.locked=true
        assertThrows(IdentityProviderException::class.java) {vault.access(generation) {error("locked key callback")}}
        protection.locked=false
        assertThrows(IllegalStateException::class.java) {vault.access(generation) {throw IllegalStateException("feature failure")}}
        assertTrue(checkNotNull(protection.lastPlain).all {it==0.toByte()})
        vault.forget();assertFalse(vault.present());assertNull(protection.key)
        assertThrows(IdentityProviderException::class.java) {vault.access(generation) {error("forgotten key callback")}}
    }
    @Test fun interrupted_creation_and_key_loss_require_explicit_recovery() {
        val files=Files();val protection=Protection();val vault=StaffKeyVault(files,protection);val generation=ByteArray(16){4}
        protection.failSeal=true
        assertThrows(IllegalStateException::class.java) {vault.importConfirmed(generation,"candidate".toByteArray()) {}}
        assertTrue(files.exists(IdentityFile.JOURNAL))
        assertThrows(IdentityProviderException::class.java) {vault.access(generation) {error("interrupted callback")}}
        assertThrows(IdentityProviderException::class.java) {vault.importConfirmed(generation,"candidate".toByteArray()) {}}
        vault.forget();protection.failSeal=false;vault.importConfirmed(generation,"candidate".toByteArray()) {}
        protection.key=null
        assertThrows(IdentityProviderException::class.java) {vault.access(generation) {error("invalidated callback")}}
        assertThrows(IdentityProviderException::class.java) {vault.importConfirmed(generation,"candidate".toByteArray()) {}}
        vault.forget();assertFalse(vault.present())
    }
    @Test fun background_gate_closes_before_queued_cleanup_and_serializes_submission() {
        val vault=StaffKeyVault(Files(),Protection())
        val entered=java.util.concurrent.CountDownLatch(1);val release=java.util.concurrent.CountDownLatch(1)
        val closed=java.util.concurrent.CountDownLatch(1)
        val sender=Thread {assertTrue(vault.submitForeground {entered.countDown();check(release.await(5,java.util.concurrent.TimeUnit.SECONDS));true})}
        sender.start();assertTrue(entered.await(5,java.util.concurrent.TimeUnit.SECONDS))
        val background=Thread {vault.background();closed.countDown()};background.start()
        assertFalse(closed.await(50,java.util.concurrent.TimeUnit.MILLISECONDS))
        release.countDown();sender.join(5000);background.join(5000)
        assertEquals(0L,closed.count)
        // Model cleanup can still be blocked: the native-submit gate is already closed.
        assertFalse(vault.submitForeground {error("background submission reached native code")})
        val candidate="candidate".toByteArray()
        assertThrows(uniffi.meshchat_core.OrganizerException.Authority::class.java) {vault.importConfirmed(ByteArray(16),candidate) {error("background import")}}
        assertTrue(candidate.all {it==0.toByte()})
        val stale=vault.requestForeground();vault.background();vault.resumeForeground(stale)
        assertFalse(vault.submitForeground {error("stale resume reopened submission")})
        vault.resumeForeground(vault.requestForeground());assertTrue(vault.submitForeground {true})
    }

}
