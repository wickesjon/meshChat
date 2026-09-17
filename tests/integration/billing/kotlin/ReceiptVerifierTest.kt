package org.meshchat.billing

import org.junit.Assert.*
import org.junit.Test
import java.security.KeyPairGenerator
import java.security.Signature
import java.util.Base64

class ReceiptVerifierTest {
    @Test fun forgedMalformedAndWrongKeyReceiptsCannotGrant() {
        val pair=KeyPairGenerator.getInstance("RSA").apply { initialize(2048) }.generateKeyPair()
        val json="{\"productId\":\"synthetic.supporter\",\"purchaseState\":0}"
        val signed=Signature.getInstance("SHA1withRSA").run {
            initSign(pair.private);update(json.toByteArray());Base64.getEncoder().encodeToString(sign())
        }
        val verifier=ReceiptVerifier(Base64.getEncoder().encodeToString(pair.public.encoded))
        assertTrue(verifier.valid(json,signed))
        assertFalse(verifier.valid(json.replace("0","1"),signed))
        assertFalse(verifier.valid(json,"invalid signature"))
        assertFalse(verifier.valid("a".repeat(65537),signed))
        val other=KeyPairGenerator.getInstance("RSA").apply { initialize(2048) }.generateKeyPair()
        assertFalse(ReceiptVerifier(Base64.getEncoder().encodeToString(other.public.encoded)).valid(json,signed))
        assertFalse(ReceiptVerifier("").configured)
        assertFalse(ReceiptVerifier("").valid(json,signed))
    }
}
