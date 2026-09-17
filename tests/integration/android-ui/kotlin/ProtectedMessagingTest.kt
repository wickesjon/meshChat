package org.meshchat.identity

import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec
import java.security.SecureRandom
import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_core.*

/** Provider-wrapper fault tests. The fake wrapping provider does not claim
 * AndroidKeyStore, encrypted filesystem or real-device protection evidence. */
class ProtectedMessagingTest {
    private class Files:IdentityStorage {
        val values=mutableMapOf<IdentityFile,ByteArray>()
        override fun exists(file:IdentityFile)=values.containsKey(file)
        override fun read(file:IdentityFile)=checkNotNull(values[file]).copyOf()
        override fun write(file:IdentityFile,bytes:ByteArray){values[file]=bytes.copyOf()}
        override fun delete(file:IdentityFile){values.remove(file)}
        override fun hasArtifacts()=values.isNotEmpty()
    }
    private class Protection:IdentityProtection {
        var locked=false;var key:ByteArray?=null
        override fun requireUnlocked(){if(locked)throw IdentityProviderException(IdentityFailure.LOCKED)}
        override fun exists()=key!=null
        override fun create(){key=random(32)}
        override fun delete(){key?.fill(0);key=null}
        override fun random(size:Int)=ByteArray(size).also {SecureRandom().nextBytes(it)}
        override fun seal(plain:ByteArray,aad:ByteArray):ByteArray {
            val iv=random(12);val cipher=Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE,SecretKeySpec(checkNotNull(key),"AES"),GCMParameterSpec(128,iv));cipher.updateAAD(aad)
            return iv+cipher.doFinal(plain)
        }
        override fun open(cipher:ByteArray,aad:ByteArray):ByteArray {
            val c=Cipher.getInstance("AES/GCM/NoPadding")
            c.init(Cipher.DECRYPT_MODE,SecretKeySpec(checkNotNull(key),"AES"),GCMParameterSpec(128,cipher.copyOfRange(0,12)));c.updateAAD(aad)
            return c.doFinal(cipher,12,cipher.size-12)
        }
        override fun capabilities()=IdentityCapabilities(WrappingProtection.SOFTWARE)
    }
    @Test fun lock_during_operation_and_feature_failure_close_session_and_reset_retires_generation() {
        val protection=Protection();var cleared=false
        val provider=IdentityProvider(Files(),protection,IdentityResetStore {cleared=true})
        val original=provider.create();var held:IdentityKeySession?=null
        assertThrows(MessagingException.Busy::class.java) {provider.messaging {session->held=session;throw MessagingException.Busy()}}
        assertThrows(IllegalStateException::class.java) {checkNotNull(held).publicIdentity()}
        val failure=assertThrows(IdentityProviderException::class.java) {provider.messaging {session->held=session;protection.locked=true;session.publicIdentity()}}
        assertEquals(IdentityFailure.LOCKED,failure.failure)
        assertThrows(IllegalStateException::class.java) {checkNotNull(held).publicIdentity()}
        assertThrows(IdentityProviderException::class.java) {provider.messaging {error("locked callback ran")}}
        protection.locked=false
        val next=provider.reset();assertTrue(cleared);assertFalse(original.identity.generation.contentEquals(next.identity.generation))
        assertThrows(IdentityProviderException::class.java) {provider.sign(original.handle,byteArrayOf(1))}
    }
}
