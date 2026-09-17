package org.meshchat.ui

import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_core.*

class SharingBindingTest {
    @Test fun everyChannelSurvivesBothShareFormsWithSameNativeId() {
        val words=channelWords()
        for(a in words.descriptors)for(b in words.genres)for(c in words.locations) {
            val channel=channelInfo("$a|$b|$c")
            val custom=Sharing.channelUri(channel.name)
            assertArrayEquals(channel.id,channelLink(custom).id)
            assertArrayEquals(channel.id,channelLink(Sharing.https(custom)).id)
        }
        assertThrows(IllegalArgumentException::class.java) {Sharing.channelUri("#general")}
    }
    @Test fun friendShareRetainsFullTupleAndSingleDecodedName() {
        IdentityKeySession.importUnlocked(ByteArray(64){31},ByteArray(16){31}).use {identity ->
            val own=friendCode(identity.publicIdentity(),"Bob / é%")
            val decoded=friendProposal(Sharing.https(own.uri))
            assertArrayEquals(own.keys,decoded.keys)
            assertEquals(own.nickname,decoded.nickname)
            assertEquals(own.fingerprint,decoded.fingerprint)
            assertEquals(Sharing.https(own.uri),Sharing.https(Sharing.https(own.uri)))
        }
        for(uri in listOf("meshfest://j/melodic-techno-valley#x","https://meshfest.app:443/j/melodic-techno-valley","meshfest://staff/a/b")) {
            assertThrows(MessagingException.Invalid::class.java) {Sharing.https(uri)}
        }
    }
}
