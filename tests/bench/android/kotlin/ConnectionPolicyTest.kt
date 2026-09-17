package org.meshchat.transport

import org.junit.Assert.*
import org.junit.Test

class ConnectionPolicyTest {
    private fun address(i: Int) = "00:00:00:00:%02X:%02X".format(i / 256, i % 256)
    @Test fun initialIdleWindowAndFiveMinuteBlacklistCannotBeResetByDiscoveries() {
        var now = 0uL
        val p = ConnectionPolicy { now }
        p.discovered(address(1), -60)
        val peer = ConnectionInfo(1, address(1), 0uL, null, null)
        now = 19_999uL
        assertTrue(p.plan(listOf(peer), 6).close.isEmpty())
        now = 20_000uL
        assertEquals(listOf(1L), p.plan(listOf(peer), 6).close)
        repeat(300) { p.discovered(address(1), -60) }
        assertFalse(p.allowed(address(1), emptyList(), 6, inbound = true))
        now = 319_999uL
        assertFalse(p.allowed(address(1), emptyList(), 6))
        now = 320_000uL
        assertTrue(p.allowed(address(1), emptyList(), 6))
        // A valid first update is sufficient; normal ANNOUNCE cadence is 30s.
        assertTrue(p.plan(listOf(peer.copy(validAt = 1uL)), 6).close.isEmpty())
    }
    @Test fun scanningCannotGrowRecordsAndRssiDenialDisablesClustering() {
        var now = 0uL
        val p = ConnectionPolicy { now }
        repeat(1000) { p.discovered(address(it), -60) }
        assertEquals(64, p.records)
        val peers = (0..2).map { ConnectionInfo(it.toLong(), address(it), 0uL, 1uL, null) }
        assertFalse(p.allowed(address(3), peers, 6))
        p.clearRssi()
        assertTrue(p.allowed(address(3), peers, 6))
        assertFalse(p.allowed(address(999), peers, 6))
        now = 300_000uL
        assertTrue(p.discovered(address(999), -90))
        assertEquals(64, p.records)
    }
    @Test fun simultaneousRolesAreAllowedAndExplorationSlotsRotateWithChurn() {
        var now = 0uL
        val p = ConnectionPolicy { now }
        p.discovered(address(1), null); p.attempted(address(1))
        val sole = ConnectionInfo(1, address(1), 0uL, 1uL, null)
        assertTrue(p.allowed(address(1), listOf(sole), 6, inbound = true))
        assertFalse(p.allowed(address(1), listOf(sole), 6))
        val peers = (1..6).map { i ->
            p.discovered(address(i), null); p.attempted(address(i))
            ConnectionInfo(i.toLong(), address(i), i.toULong(), 10uL, null)
        }
        now = 59_999uL
        p.discovered(address(7), -70); p.discovered(address(8), -50)
        assertTrue(p.plan(peers, 6).close.isEmpty())
        now = 60_000uL
        val rotate = p.plan(peers, 6)
        assertEquals(setOf(5L, 6L), rotate.close.toSet())
        assertEquals(address(8), rotate.connect)
        assertFalse(p.allowed(address(6), emptyList(), 6))
        // Saver preserves one stable slot, with two exploration slots.
        now = 120_000uL
        p.discovered(address(7), -70); p.discovered(address(8), -50)
        assertEquals(setOf(2L, 3L), p.plan(peers.take(3), 3).close.toSet())
    }
    @Test fun failedConnectionsHaveBoundedRetryAndUnseenPeersWinBeforeRssi() {
        var now = 0uL
        val p = ConnectionPolicy { now }
        p.discovered(address(1), -20); p.attempted(address(1))
        p.discovered(address(2), -100)
        now = 5_000uL
        assertEquals(address(2), p.plan(emptyList(), 6).connect)
        p.attempted(address(2)); p.disconnected(address(2))
        assertFalse(p.allowed(address(2), emptyList(), 6))
        now = 10_000uL
        assertTrue(p.allowed(address(2), emptyList(), 6))
    }
    @Test fun isolationSaverForegroundAndScanFailureSchedulesAreBounded() {
        var now = 0uL
        val p = ScanPolicy { now }
        fun mode(saver: Boolean = false, peers: Int = 0, foreground: Boolean = false, allowed: Boolean = true) = p.mode(saver, peers, foreground, allowed)
        assertEquals(ScanMode.BURST, mode())
        now = 10_000uL; assertEquals(ScanMode.BALANCED, mode())
        now = 300_000uL; assertEquals(ScanMode.BURST, mode())
        now = 310_000uL; assertEquals(ScanMode.OFF, mode())
        now = 330_000uL; assertEquals(ScanMode.BURST, mode())
        now = 900_000uL; assertEquals(ScanMode.BURST, mode())
        now = 930_000uL; assertEquals(ScanMode.OFF, mode())
        assertEquals(ScanMode.BURST, mode(foreground = true))
        assertEquals(ScanMode.OFF, mode(allowed = false))
        p.discovered(); assertEquals(ScanMode.BALANCED, mode(peers = 1))
        assertEquals(ScanMode.BALANCED, mode(saver = true, peers = 4))
        now += 10_000uL; assertEquals(ScanMode.OFF, mode(saver = true, peers = 4))
        now += 50_000uL; assertEquals(ScanMode.BALANCED, mode(saver = true, peers = 4))
        p.failed(); assertTrue(p.retrying); assertEquals(ScanMode.OFF, mode(saver = true, peers = 4))
        now += 4_999uL; assertTrue(p.retrying)
        now++; assertFalse(p.retrying)
        p.failed(); now += 9_999uL; assertTrue(p.retrying)
        now++; assertFalse(p.retrying)
        p.received(); p.failed(); now += 5_000uL; assertFalse(p.retrying)
    }
    @Test fun discoveryDenialAndBackgroundChangesRequireRealPermissionRecovery() {
        for (api in listOf(29, 30, 31, 36)) {
            assertEquals(RadioState.LOCATION_REQUIRED, discoveryRestriction(api, true, false, true, true))
            assertEquals(RadioState.LOCATION_REQUIRED, discoveryRestriction(api, true, true, false, true))
            assertNull(discoveryRestriction(api, true, true, true, false))
            assertEquals(if (api <= 30) RadioState.BACKGROUND_LOCATION_REQUIRED else null,
                discoveryRestriction(api, false, true, true, false))
            assertNull(discoveryRestriction(api, false, true, true, true))
        }
    }

    @Test fun freshNoveltyPromotesAnExplorerAndChangedRssiIsReevaluated() {
        var now = 0uL
        val p = ConnectionPolicy { now }
        val peers = (1..6).map { i ->
            p.discovered(address(i), null); p.attempted(address(i))
            ConnectionInfo(i.toLong(), address(i), i.toULong(), 10uL, if (i == 6) 1000 else 0)
        }
        now = 60_000uL
        p.discovered(address(7), null); p.discovered(address(8), null)
        val plan = p.plan(peers, 6)
        assertFalse(6L in plan.close)
        assertEquals(2, plan.close.size)
        now = 120_000uL
        peers.forEach { p.discovered(it.address, -60) }
        val crowded = p.plan(peers, 6)
        assertEquals(3, crowded.close.size)
        assertFalse(6L in crowded.close)
    }

    @Test fun activePeersRemainLocallyVisibleAfterTheirAdvertisementsExpire() {
        var now = 0uL
        val p = ConnectionPolicy { now }
        p.discovered(address(1), null)
        assertEquals(1, p.visible())
        now = 60_000uL
        assertEquals(0, p.visible())
        val peer = ConnectionInfo(1, address(1), 0uL, 1uL, null)
        assertEquals(1, p.visible(listOf(peer, peer.copy(id = 2))))
        p.discovered(address(2), null)
        assertEquals(2, p.visible(listOf(peer)))
    }

}
