package org.meshchat.ffi

import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_core.*

class FoundationTest {
    private fun trace(): List<String> = Core(Limits(1u, 64u), 42uL, 100uL).use { core ->
        val link = (core.connected(Capacities(20u, 12u, 30u)).uiEvents.single() as UiEvent.LinkConnected).link
        assertEquals(LinkHandle(42uL, 1uL), link)
        core.handleEvent(DriverEvent.TimeAdvanced(101uL))
        val bytes = byteArrayOf(0, 127, -1)
        val sent = core.prepareSend(link, SendPath.WRITE, bytes).sends.single()
        assertArrayEquals(bytes, sent.bytes)
        assertEquals(link, sent.link)
        assertEquals(SendPath.WRITE, sent.path)
        assertEquals(3u.toUShort(), (core.handleEvent(DriverEvent.InboundBytes(link, bytes)).uiEvents.single() as UiEvent.InboundObserved).byteCount)
        assertThrows(CoreException.InvalidValue::class.java) { core.handleEvent(DriverEvent.InboundBytes(link, byteArrayOf())) }
        assertThrows(CoreException.InvalidValue::class.java) { core.prepareSend(link, SendPath.NOTIFY, ByteArray(13)) }
        assertThrows(CoreException.TimeRegression::class.java) { core.handleEvent(DriverEvent.TimeAdvanced(99uL)) }
        core.handleEvent(DriverEvent.PowerChanged(PowerState.LOW_POWER))
        core.handleEvent(DriverEvent.Disconnected(link))
        assertThrows(CoreException.UnknownLink::class.java) { core.prepareSend(link, SendPath.WRITE, bytes) }
        assertThrows(CoreException.UnknownLink::class.java) { core.handleEvent(DriverEvent.Disconnected(link)) }
        val next = (core.connected(Capacities(20u, 12u, 30u)).uiEvents.single() as UiEvent.LinkConnected).link
        assertEquals(2uL, next.generation)
        listOf(link.generation.toString(), sent.bytes.joinToString(), next.generation.toString())
    }

    @Test fun deterministicBinaryRoundTripAndErrors() {
        assertEquals(trace(), trace())
    }
}
