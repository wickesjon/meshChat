package org.meshchat.ffi

import java.io.File
import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_core.*
import com.google.zxing.*
import com.google.zxing.qrcode.QRCodeWriter
import com.google.zxing.common.HybridBinarizer

class MessagingBindingTest {
    private fun root():File=generateSequence(File(requireNotNull(System.getProperty("user.dir")))) {it.parentFile}.first {File(it,"AGENTS.md").isFile}
    @Test fun nativeProposalOpaqueHandleAndExplicitReplacementCrossGeneratedBinding() {
        TransportDatabase(root()).use {db ->
            IdentityKeySession.importUnlocked(ByteArray(64){71},ByteArray(16){71}).use {a ->
                IdentityKeySession.importUnlocked(ByteArray(64){72},ByteArray(16){72}).use {b ->
                    EncryptedStore.open(db,a.publicIdentity().generation,true,200000).use {store ->
                        NativeTransport(store,a.publicIdentity(),71u,0u).use {core ->
                            val proposal=friendCode(b.publicIdentity(),"Bob / é")
                            assertArrayEquals(proposal.keys,friendProposal(proposal.uri).keys)
                            assertEquals(64,proposal.keys.size);assertEquals(79,proposal.fingerprint.length)
                            assertTrue(core.friendCards(store,0u).isEmpty())
                            core.confirmFriend(store,a,proposal.uri,"My Bob",null,0u)
                            val pin=core.friendCards(store,0u).single()
                            assertEquals("My Bob",pin.petname);assertFalse(pin.fresh)
                            assertThrows(MessagingException.Invalid::class.java) {core.confirmFriend(store,a,proposal.uri,"Duplicate",null,0u)}
                            core.changeFriend(store,pin.handle,true,0u)
                            assertTrue(core.friendCards(store,0u).single().replacing)
                            core.changeFriend(store,pin.handle,false,0u)
                            assertTrue(core.friendCards(store,0u).isEmpty())
                            assertTrue(core.directHistory(store,pin.keys,0u,200000).isEmpty())
                            assertThrows(MessagingException.Invalid::class.java) {friendProposal("meshfest://friend/bad/claimed")}
                        }
                    }
                }
            }
        }
    }
    @Test fun offlineQrDecoderRoundTripsExactFullTupleProposal() {
        IdentityKeySession.importUnlocked(ByteArray(64){73},ByteArray(16){73}).use {identity ->
            val code=friendCode(identity.publicIdentity(),"Camera fixture")
            val matrix=QRCodeWriter().encode(code.uri,BarcodeFormat.QR_CODE,512,512)
            val pixels=IntArray(512*512) {i -> if(matrix[i%512,i/512])0xff000000.toInt() else 0xffffffff.toInt()}
            val decoded=MultiFormatReader().decode(BinaryBitmap(HybridBinarizer(RGBLuminanceSource(512,512,pixels))))
            val proposal=friendProposal(decoded.text)
            assertArrayEquals(code.keys,proposal.keys);assertEquals(code.fingerprint,proposal.fingerprint)
        }
    }
}
