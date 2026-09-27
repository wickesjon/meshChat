package org.meshchat.transport

import java.util.ArrayDeque

/** Called under the radio monitor. A server characteristic may have only one
 * native notification in flight, across all peers and legacy mutable values.
 * Core already reserved each bounded frame and owns its absolute deadline.
 */
class NotificationQueue(
    private val submit: (Long, ByteArray) -> Boolean,
    private val complete: (Long, Boolean) -> Unit,
) {
    private val pending = ArrayDeque<Pair<Long, ByteArray>>(6)
    private var active: Long? = null
    fun offer(id: Long, bytes: ByteArray): Boolean {
        if (bytes.isEmpty() || bytes.size > 512 || pending.size + (if (active == null) 0 else 1) >= 6 ||
            active == id || pending.any { it.first == id }) return false
        pending.addLast(id to bytes)
        drain()
        return true
    }
    fun sent(id: Long, success: Boolean) {
        if (active != id) return
        active = null
        complete(id, success)
        drain()
    }
    /** The owner has closed this token. A submitted notification still owns
     * the shared characteristic until its callback or a fresh server epoch. */
    fun retire(id: Long): Boolean {
        if (active == id) return false
        pending.removeAll { it.first == id }
        return true
    }
    private fun drain() {
        while (active == null && pending.isNotEmpty()) {
            val (id, bytes) = pending.removeFirst()
            active = id
            if (!submit(id, bytes)) { active = null; complete(id, false) }
        }
    }
    fun clear() { pending.clear(); active = null }
}
