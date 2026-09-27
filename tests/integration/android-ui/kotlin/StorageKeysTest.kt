package org.meshchat.storage

import org.junit.Assert.*
import org.junit.Test
import org.meshchat.identity.IdentityProviderException

class StorageKeysTest {
    @Test fun binaryLegacyKnownAnswers() {
        // Independently generated with Python hashlib.pbkdf2_hmac('sha512', ..., 256000, 32).
        val salt=ByteArray(16) {it.toByte()}
        for((offset,expected) in listOf(
            0 to "a15a4766e9517ab2b44a8ba32eb59863231373b86775ffa586453ac691bd9442",
            192 to "367cbca36d8ece53f9db7214cadae05331ad9973b60ef35be9ac3ecac14e6c05")) {
            val pass=ByteArray(64) {(it+offset).toByte()}
            val material=StorageKeys.deriveLegacy(pass,salt)
            assertEquals(expected,material.take(32).joinToString("") {"%02x".format(it)})
            assertArrayEquals(salt,material.copyOfRange(32,48))
            assertTrue(material.copyOfRange(48,64).all {it==0.toByte()})
            assertArrayEquals(ByteArray(64) {(it+offset).toByte()},pass)
            val input=StorageKeys.rawInput(material)
            assertEquals("x'$expected"+salt.joinToString("") {"%02x".format(it)}+"'",input.toString(Charsets.US_ASCII))
            pass.fill(0);material.fill(0);input.fill(0)
        }
    }
    @Test fun rejectsMalformedMaterial() {
        for(size in listOf(0,48,63,65)) assertThrows(IdentityProviderException::class.java) {StorageKeys.rawInput(ByteArray(size))}
        assertThrows(IdentityProviderException::class.java) {StorageKeys.rawInput(ByteArray(64).also {it[63]=1})}
        assertThrows(IdentityProviderException::class.java) {StorageKeys.deriveLegacy(ByteArray(63),ByteArray(16))}
        assertThrows(IdentityProviderException::class.java) {StorageKeys.deriveLegacy(ByteArray(64),ByteArray(15))}
    }
}
