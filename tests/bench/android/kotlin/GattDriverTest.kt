package org.meshchat.transport

import java.io.File
import org.junit.Assert.*
import org.junit.Test
import org.meshchat.ffi.TransportDatabase
import uniffi.meshchat_core.*

class GattDriverTest {
    private class Radio : GattRadio {
        val calls = mutableListOf<String>()
        val sends = mutableListOf<Triple<Long, SendPath, ByteArray>>()
        var accepted = true
        override fun connect(id: Long, address: String): Boolean { calls += "connect:$id"; return accepted }
        override fun discover(id: Long): Boolean { calls += "discover:$id"; return accepted }
        override fun requestMtu(id: Long): Boolean { calls += "mtu:$id"; return accepted }
        override fun subscribe(id: Long): Boolean { calls += "subscribe:$id"; return accepted }
        override fun send(id: Long, path: SendPath, bytes: ByteArray): Boolean { sends += Triple(id, path, bytes.copyOf()); return accepted }
        override fun close(id: Long) { calls += "close:$id" }
    }
    private class Node(val core: NativeTransport) {
        var time = 0uL
        val radio = Radio()
        val events = mutableListOf<Pair<Long, TransportEvent>>()
        val driver = GattDriver(core, radio, { time }) { id, event -> events += id to event }
        fun central(address: String = "00:00:00:00:00:01", mtu: Int = 517): Long {
            val id = requireNotNull(driver.connect(address))
            driver.connected(id, true); driver.services(id, true); driver.mtu(id, mtu, true); driver.subscribed(id, true)
            return id
        }
        fun peripheral(): Long {
            val id = requireNotNull(driver.incoming("00:00:00:00:00:02"))
            driver.connected(id, true); driver.subscribed(id, true); driver.mtu(id, 185, true)
            return id
        }
    }
    private fun <T> node(seed: Int, action: (Node) -> T): T {
        val root = generateSequence(File(requireNotNull(System.getProperty("user.dir")))) { it.parentFile }.first { File(it, "AGENTS.md").isFile }
        TransportDatabase(root).use { database ->
            IdentityKeySession.importUnlocked(ByteArray(64) { seed.toByte() }, ByteArray(16) { seed.toByte() }).use { identity ->
                EncryptedStore.open(database, identity.publicIdentity().generation, true, 200000L).use { store ->
                    NativeTransport(store, identity.publicIdentity(), seed.toULong(), 0uL).use { core ->
                        val n = Node(core)
                        try { return action(n) } finally { n.driver.stop(); identity.invalidate() }
                    }
                }
            }
        }
    }
    private fun chat(id: Int, length: Int): ByteArray {
        val p = byteArrayOf(0, 3, 13, 64, 0, 1, 65, 0, 0, 0, 0, (length shr 8).toByte(), length.toByte()) + ByteArray(length) { 120 }
        // Committed MC-006 clear CHAT layout; payload parsing remains in Rust.
        val h = byteArrayOf(1, 1, 0, 7) + ByteArray(8) { id.toByte() } + ByteArray(8) { id.toByte() } +
            java.security.MessageDigest.getInstance("SHA-256").digest("meshfest-v1|#general".toByteArray()).copyOf(4) + byteArrayOf((p.size shr 8).toByte(), p.size.toByte())
        return h + p
    }
    @Test fun callbackHandshakeAndWholeFragmentedTrafficInBothDirections() = node(50) { a -> node(51) { b ->
        val al = a.central()
        val bl = b.peripheral()
        fun exchange(sender: Node, receiver: Node, receiveId: Long) {
            sender.driver.tick()
            val frames = sender.radio.sends.toList(); sender.radio.sends.clear()
            for ((id, _, value) in frames) {
                assertTrue(value.size <= 146)
                receiver.driver.value(receiveId, value)
                sender.driver.completed(id, true)
            }
        }
        exchange(a, b, bl); exchange(b, a, al)
        assertTrue(a.events.any { it.second is TransportEvent.Admitted })
        assertTrue(b.events.any { it.second is TransportEvent.Admitted })
        for ((sender, receiver, sendId, receiveId) in listOf(arrayOf(a, b, al, bl), arrayOf(b, a, bl, al))) {
            sender as Node; receiver as Node; sendId as Long; receiveId as Long
            for ((id, size) in listOf(70 to 5, 71 to 256)) {
                sender.time += 1000uL; receiver.time = sender.time
                val raw = chat(id, size)
                val before = receiver.events.count { it.second is TransportEvent.Received }
                sender.driver.enqueue(sendId, raw, TransportTraffic.OWN, id.toULong())
                repeat(if (size == 256) 3 else 1) {
                    exchange(sender, receiver, receiveId)
                    sender.time += 1000uL; receiver.time = sender.time
                }
                assertEquals(before + 1, receiver.events.count { it.second is TransportEvent.Received })
                val received = receiver.events.last { it.second is TransportEvent.Received }.second as TransportEvent.Received
                assertArrayEquals(raw, received.bytes)
                assertEquals(TransportIntake.UNVERIFIED, received.intake)
            }
        }
    } }
    @Test fun subscriptionAndMeasuredMtuAreBothRequiredAndEarlyValuesAreBounded() = node(52) { n ->
        val id = requireNotNull(n.driver.connect("00:00:00:00:00:03"))
        n.driver.connected(id, true); n.driver.services(id, true)
        n.driver.tick(); assertTrue(n.radio.sends.isEmpty())
        n.driver.mtu(id, 185, true)
        n.driver.tick(); assertTrue(n.radio.sends.isEmpty())
        // Drop newest before copy while the subscription callback is outstanding.
        repeat(20) { n.driver.value(id, ByteArray(1024)) }
        assertEquals(18uL, n.driver.stagingDrops)
        n.driver.subscribed(id, true)
        assertFalse(n.events.any { it.second is TransportEvent.Admitted || it.second is TransportEvent.Received })
        n.time = 11000uL; n.driver.tick()
        assertTrue(n.radio.calls.contains("close:$id")) // absent valid HELLO times out
    }
    @Test fun notificationBeforeCccdCompletionKeepsAnImmutableBoundedCopy() = node(56) { a -> node(57) { b ->
        val al = requireNotNull(a.driver.connect("00:00:00:00:00:11"))
        a.driver.connected(al, true); a.driver.services(al, true); a.driver.mtu(al, 517, true)
        val bl = b.peripheral()
        b.driver.tick()
        val early = b.radio.sends.single().third
        a.driver.value(al, early)
        early.fill(0) // Android may reuse legacy characteristic storage.
        b.driver.completed(bl, true)
        a.driver.subscribed(al, true)
        a.driver.tick()
        b.driver.value(bl, a.radio.sends.single().third)
        a.driver.completed(al, true)
        assertTrue(a.events.any { it.second is TransportEvent.Admitted })
        assertTrue(b.events.any { it.second is TransportEvent.Admitted })
    } }
    @Test fun setupReservationsExpireAndSmallCapacityNeverStartsHello() = node(58) { n ->
        val small = requireNotNull(n.driver.connect("00:00:00:00:00:12"))
        n.driver.mtu(small, 148, true)
        val stalled = requireNotNull(n.driver.connect("00:00:00:00:00:13"))
        n.driver.subscribed(stalled, true) // still no successful MTU callback
        n.driver.tick(); assertTrue(n.radio.sends.isEmpty())
        n.time = 30000uL; n.driver.tick()
        assertTrue(listOf(small, stalled).all { n.radio.calls.contains("close:$it") })
    }
    @Test fun failedSubscriptionsAndUnknownOrChangedCapacityCloseTheGeneration() = node(53) { n ->
        val a = requireNotNull(n.driver.connect("00:00:00:00:00:04"))
        n.driver.subscribed(a, false)
        val b = requireNotNull(n.driver.connect("00:00:00:00:00:05"))
        n.driver.mtu(b, 517, false)
        val c = n.central("00:00:00:00:00:06")
        n.driver.tick(); val count = n.radio.sends.size
        n.driver.mtu(c, 185, true)
        n.driver.completed(c, true) // stale native completion cannot revive c
        n.driver.value(c, byteArrayOf(1, 2, 3)); n.driver.tick()
        assertEquals(count, n.radio.sends.size)
        assertTrue(listOf(a, b, c).all { n.radio.calls.contains("close:$it") })
    }
    @Test fun refusedCccdResponseCannotCommitSubscriptionOrStartHello() = node(59) { n ->
        val id = requireNotNull(n.driver.incoming("00:00:00:00:00:14"))
        n.driver.connected(id, true); n.driver.mtu(id, 517, true)
        val committed = mutableListOf<Boolean>()
        var attempts = 0
        n.driver.subscriptionRequest(id, true, { attempts++; false }) { committed += it }
        n.driver.tick()
        assertEquals(1, attempts)
        assertEquals(listOf(false), committed)
        assertTrue(n.radio.calls.contains("close:$id"))
        assertTrue(n.radio.sends.isEmpty())
        val next = requireNotNull(n.driver.incoming("00:00:00:00:00:15"))
        n.driver.mtu(next, 517, true)
        n.driver.subscriptionRequest(next, true, { true }) { committed += it }
        n.driver.tick()
        assertEquals(listOf(false, true), committed)
        assertEquals(next, n.radio.sends.single().first)
    }
    @Test fun busyRefusalIsPacedAndNeverFakedAsSuccess() = node(54) { n ->
        val id = n.central()
        n.radio.accepted = false
        n.driver.tick(); assertEquals(1, n.radio.sends.size)
        n.time = 999uL; n.driver.tick(); assertEquals(1, n.radio.sends.size)
        n.time = 1000uL; n.driver.tick(); assertEquals(2, n.radio.sends.size)
        assertTrue(n.radio.calls.contains("close:$id"))
        assertFalse(n.events.any { it.second is TransportEvent.Admitted })
    }
    @Test fun callbackFailureTimeoutDisconnectAndPermissionStopDoNotReusePendingWork() = node(55) { n ->
        val first = n.central()
        n.driver.tick(); n.driver.completed(first, false)
        n.time = 1000uL; n.driver.tick(); n.driver.completed(first, true)
        n.driver.lost(first)
        val next = n.central("00:00:00:00:00:08")
        n.driver.completed(first, true)
        n.driver.tick(); n.time += 5000uL; n.driver.tick()
        assertTrue(n.radio.calls.contains("close:$next"))
        n.driver.completed(next, true)
        val pending = requireNotNull(n.driver.connect("00:00:00:00:00:09"))
        n.driver.stop() // actual permission/radio/lock callbacks enter this path
        assertTrue(n.radio.calls.contains("close:$pending"))
        assertNull(n.driver.connect("00:00:00:00:00:10"))
    }
}
