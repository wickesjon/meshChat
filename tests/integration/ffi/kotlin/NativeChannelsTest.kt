package org.meshchat.ffi

import java.io.File
import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_core.*

class NativeChannelsTest {
    private fun root(): File = generateSequence(File(requireNotNull(System.getProperty("user.dir")))) { it.parentFile }
        .first { File(it,"AGENTS.md").isFile }
    @Test fun channelsUseRealSanitizationAnonymousEncodingAndPersistentReactions() {
        TransportDatabase(root()).use { db ->
            IdentityKeySession.importUnlocked(ByteArray(64){124},ByteArray(16){124}).use { identity ->
                val public=identity.publicIdentity()
                EncryptedStore.open(db,public.generation,true,200000L).use { store ->
                    NativeChannels(public,0uL).use { channels ->
                        val first=channels.compose("#confessions","Alice",0x23u,"🎉".repeat(70),200000u,0uL)
                        val next=channels.compose("#confessions","Alice",0x23u,"Another",200000u,0uL)
                        assertFalse(first.copyOfRange(12,20).contentEquals(next.copyOfRange(12,20)))
                        assertTrue(channels.accept(store,ChannelReceipt(null,first,TransportIntake.UNVERIFIED,true,200000,0u)))
                        val message=channels.history(store,"#confessions","Alice").single()
                        assertEquals("Anonymous",message.nickname);assertEquals(0u.toUByte(),message.avatar)
                        val reaction=channels.reaction("#confessions",message.id,3u,false,0u)
                        assertArrayEquals(public.senderId,reaction.copyOfRange(12,20))
                        assertTrue(channels.accept(store,ChannelReceipt(null,reaction,TransportIntake.UNVERIFIED,true,200000,0u)))
                        assertEquals(1u.toUShort(),channels.history(store,"#confessions","Alice").single().reactions[3])
                        assertThrows(ChannelException.Invalid::class.java) { channels.compose("#general","Alice",1u,"🎉".repeat(71),200000u,0u) }
                        assertThrows(ChannelException.Invalid::class.java) { channels.compose("#general","Alice",1u,"bad\u202etext",200000u,0u) }
                    }
                }
                identity.invalidate()
            }
        }
    }
    @Test fun nativeNamesAndBadgeSanitizationMatchCore() {
        assertArrayEquals(channelInfo("MELODIC|techno|VALLEY").id,channelInfo("melodic|techno|valley").id)
        assertEquals("Alice",channelNickname("✓ Official Alice"))
        assertEquals(20,channelWords().genres.size)
    }
}
