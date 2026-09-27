package org.meshchat.transport

import org.junit.Assert.*
import org.junit.Test

class NotificationQueueTest {
    @Test fun retirementDropsUnsentWorkWithoutAdvancingAnotherPeersNotification() {
        val sent = mutableListOf<Long>()
        val done = mutableListOf<Pair<Long, Boolean>>()
        val queue = NotificationQueue({ id, _ -> sent += id; true }) { id, success -> done += id to success }
        for (id in 1L..6L) assertTrue(queue.offer(id, byteArrayOf(1)))
        assertFalse(queue.retire(1)) // active callback still owns the characteristic
        assertFalse(queue.offer(7, byteArrayOf(1))) // failed retirement changes nothing
        assertTrue(queue.retire(2))
        assertTrue(queue.offer(7, byteArrayOf(1)))
        queue.sent(2, true) // late completion from the retired peer cannot advance
        assertEquals(listOf(1L), sent)
        assertTrue(done.isEmpty())
        queue.sent(1, true)
        assertEquals(listOf(1L, 3L), sent)
        for (id in 3L..7L) queue.sent(id, true)
        assertEquals(listOf(1L, 3L, 4L, 5L, 6L, 7L), sent)
        assertEquals(sent.map { it to true }, done)
        assertTrue(queue.retire(1))
    }
    @Test fun completionCanRetireQueuedPeerReentrantly() {
        val sent = mutableListOf<Long>()
        lateinit var queue: NotificationQueue
        queue = NotificationQueue({ id, _ -> sent += id; true }) { id, _ ->
            if (id == 1L) assertTrue(queue.retire(2))
        }
        for (id in 1L..3L) queue.offer(id, byteArrayOf(1))
        queue.sent(1, true)
        assertEquals(listOf(1L, 3L), sent)
    }
    @Test fun serializedAcrossPeersBoundedAndCallbackDriven() {
        val sent = mutableListOf<Long>()
        val done = mutableListOf<Pair<Long, Boolean>>()
        val queue = NotificationQueue({ id, bytes -> assertEquals(512, bytes.size); sent += id; true }) { id, success -> done += id to success }
        for (id in 1L..6L) assertTrue(queue.offer(id, ByteArray(512)))
        assertFalse(queue.offer(7, ByteArray(512)))
        assertFalse(queue.offer(1, ByteArray(512)))
        assertEquals(listOf(1L), sent)
        queue.sent(6, true) // unrelated/stale callback cannot advance the queue
        assertEquals(listOf(1L), sent)
        queue.sent(1, true)
        assertEquals(listOf(1L, 2L), sent)
        queue.sent(2, false)
        assertEquals(listOf(1L to true, 2L to false), done)
        queue.clear()
        queue.sent(3, true)
        assertEquals(2, done.size)
        assertTrue(queue.offer(9, ByteArray(512)))
        assertEquals(listOf(1L, 2L, 3L, 9L), sent)
    }
    @Test fun submissionFailureAndReentrantTeardownDoNotLeakQueuedSends() {
        val sent = mutableListOf<Long>()
        val done = mutableListOf<Pair<Long, Boolean>>()
        lateinit var queue: NotificationQueue
        queue = NotificationQueue({ id, _ -> sent += id; id == 1L }) { id, success ->
            done += id to success
            if (id == 2L) queue.clear()
        }
        assertFalse(queue.offer(1, ByteArray(513)))
        queue.offer(1, byteArrayOf(1)); queue.offer(2, byteArrayOf(2)); queue.offer(3, byteArrayOf(3))
        queue.sent(1, true)
        assertEquals(listOf(1L, 2L), sent)
        assertEquals(listOf(1L to true, 2L to false), done)
    }
}
