package org.meshchat.ffi
import java.io.File
import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_core.*
class StatsBindingTest {
    @Test fun generatedSnapshotPowerResetAndShare() {
        val root=generateSequence(File(requireNotNull(System.getProperty("user.dir")))) {it.parentFile}.first {File(it,"AGENTS.md").isFile}
        TransportDatabase(root).use {db->IdentityKeySession.importUnlocked(ByteArray(64){44},ByteArray(16){44}).use {identity->
            EncryptedStore.open(db,identity.publicIdentity().generation,true,200000).use {store->NativeTransport(store,identity.publicIdentity(),44u,0u).use {core->
                assertNull(core.contributionStats(0u).batteryPercent)
                core.updatePower(TransportPowerSetting.SAVER,32u,false,0u,1u)
                val s=core.contributionStats(2u);assertEquals(32.toUByte(),s.batteryPercent);assertEquals("Saver",s.powerMode)
                val text=contributionShareText(s.copy(receivedFrames=999u,completedFrames=888u,relayedChatCopies=777u,powerMode="PRIVATE_SENTINEL"),false,false,true)
                assertTrue(text.contains("777"));assertFalse(text.contains("999"));assertFalse(text.contains("888"));assertFalse(text.contains("PRIVATE_SENTINEL"));assertTrue(text.contains("Delivery unknown"))
                core.resetContributionStats(3u);assertEquals(0uL,core.contributionStats(3u).elapsedMs)
                assertNull(core.contributionStats(60002u).batteryPercent)
            }}
        }}
    }
}
