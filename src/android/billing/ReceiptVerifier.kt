package org.meshchat.billing

import java.security.KeyFactory
import java.security.Signature
import java.security.spec.X509EncodedKeySpec
import java.util.Base64

/** Google Play's signed JSON is checked before any of its purchase fields grant
 * local cosmetics. This is not a mesh identity or a payment attestation to peers. */
class ReceiptVerifier(encodedKey:String) {
    private val key=runCatching {
        KeyFactory.getInstance("RSA").generatePublic(X509EncodedKeySpec(Base64.getDecoder().decode(encodedKey)))
    }.getOrNull()
    val configured:Boolean get()=key!=null
    fun valid(json:String,signature:String):Boolean {
        if(key==null || json.length>65536 || signature.length>4096)return false
        return runCatching {
            Signature.getInstance("SHA1withRSA").run {
                initVerify(key);update(json.toByteArray(Charsets.UTF_8))
                verify(Base64.getDecoder().decode(signature))
            }
        }.getOrDefault(false)
    }
}
