package org.meshchat.transport

import java.util.ArrayDeque
import uniffi.meshchat_core.*

/** Native operations only. Protocol bytes and admission belong to NativeTransport. */
interface GattRadio {
    fun connect(id: Long, address: String): Boolean
    fun discover(id: Long): Boolean
    fun requestMtu(id: Long): Boolean
    fun subscribe(id: Long): Boolean
    fun send(id: Long, path: SendPath, bytes: ByteArray): Boolean
    fun close(id: Long)
}

/** All entry points run under AndroidGattRadio's single monitor, including ticks.
 * No per-value Handler queue: admitted values cross FFI synchronously. The only
 * receive staging is two bounded values per setup link (at most twelve total).
 */
class GattDriver(
    private val core: NativeTransport,
    private val radio: GattRadio,
    private val clock: () -> ULong,
    private val event: (Long, TransportEvent) -> Unit,
) {
    /** Configured by the protected application adapter before start. Without
     * it, protected outgoing operations fail closed. Called under radio gate. */
    var protectedEgress: ((TransportSend, () -> Boolean) -> Boolean)? = null
    fun operation(work: (NativeTransport) -> TransportEffects): Boolean = guarded(false) {
        if (stopped) return@guarded false
        effects(work(core)); true
    }
    private class Peer(val id: Long, val address: String, val role: TransportRole, val admission: ULong, val born: ULong) {
        var link: LinkHandle? = null
        var capacity = 0
        var subscribed = false
        var pending: ULong? = null
        val early = ArrayDeque<Pair<ULong, ByteArray>>(2)
    }
    private val peers = linkedMapOf<Long, Peer>()
    private var serial = 0L
    private var stopped = false
    private var linkLimit = 6
    var stagingDrops = 0uL
        private set
    private fun dropped() { if (stagingDrops < ULong.MAX_VALUE) stagingDrops++ }

    private fun reserve(address: String, role: TransportRole): Peer? {
        if (stopped || peers.size >= linkLimit || peers.values.any { it.address == address && it.role == role }) return null
        val digest = java.security.MessageDigest.getInstance("SHA-256").digest(address.toByteArray(Charsets.US_ASCII)).copyOf(16)
        val permit = try { core.admitConnection(digest, clock()) } catch (_: TransportException.Busy) { return null }
        if (serial == Long.MAX_VALUE) { core.cancelConnection(permit, clock()); stop(); return null }
        val peer = Peer(++serial, address, role, permit, clock())
        peers[peer.id] = peer
        return peer
    }
    fun connect(address: String): Long? = guarded(null) {
        val p = reserve(address, TransportRole.CENTRAL) ?: return@guarded null
        if (!radio.connect(p.id, address)) { close(p.id); null } else p.id
    }
    fun incoming(address: String): Long? = guarded(null) { reserve(address, TransportRole.PERIPHERAL)?.id }
    fun connected(id: Long, success: Boolean) = guarded(Unit) {
        val p = peers[id] ?: return@guarded
        if (!success || (p.role == TransportRole.CENTRAL && !radio.discover(id))) close(id)
    }
    fun services(id: Long, valid: Boolean) = guarded(Unit) {
        val p = peers[id] ?: return@guarded
        if (p.role != TransportRole.CENTRAL || p.link != null || !valid || !radio.requestMtu(id)) close(id)
    }
    fun mtu(id: Long, mtu: Int, success: Boolean) = guarded(Unit) {
        val p = peers[id] ?: return@guarded
        val capacity = if (success && mtu in 23..517) minOf(mtu - 3, 512) else 0
        if (capacity < 146 || (p.capacity != 0 && p.capacity != capacity)) { close(id); return@guarded }
        if (p.capacity == capacity) return@guarded
        p.capacity = capacity
        if (p.role == TransportRole.CENTRAL && !radio.subscribe(id)) { close(id); return@guarded }
        ready(p)
    }
    fun subscribed(id: Long, success: Boolean) = guarded(Unit) {
        val p = peers[id] ?: return@guarded
        if (!success) { close(id); return@guarded }
        p.subscribed = true
        ready(p)
    }
    /** A server descriptor request is committed only after Android accepts the
     * response. Commit native subscription state before any core-ready effect.
     */
    fun subscriptionRequest(id: Long, enabled: Boolean, acknowledge: () -> Boolean, commit: (Boolean) -> Unit) = guarded(Unit) {
        if (id !in peers) return@guarded
        val accepted = acknowledge() && enabled
        commit(accepted)
        subscribed(id, accepted)
    }
    private fun ready(p: Peer) {
        if (p.link != null || !p.subscribed || p.capacity == 0) return
        val connection = try { core.nativeReady(p.admission, p.role, p.capacity.toUShort(), p.capacity.toUShort(), clock()) }
        catch (e: TransportException) {
            if (e is TransportException.Refused) effects(e.effects)
            close(p.id)
            if (e is TransportException.Unavailable) throw e
            return
        }
        p.link = connection.link
        effects(connection.effects)
        while (peers[p.id] === p && p.early.isNotEmpty()) {
            val (reported, bytes) = p.early.removeFirst()
            effects(core.receive(connection.link, reported, bytes, clock()))
        }
    }
    fun value(id: Long, bytes: ByteArray) = guarded(Unit) {
        val p = peers[id] ?: return@guarded
        val reported = bytes.size.toULong()
        // Never copy an oversized native value. Core still charges its reported
        // length and one frame, including malformed input.
        val bounded = if (bytes.size <= 512) bytes else byteArrayOf()
        val link = p.link
        if (link == null) {
            if (p.early.size == 2) { dropped(); return@guarded }
            p.early.addLast(reported to bounded.copyOf())
        } else effects(core.receive(link, reported, bounded, clock()))
    }
    fun completed(id: Long, success: Boolean) = guarded(Unit) {
        val p = peers[id] ?: return@guarded
        val token = p.pending ?: return@guarded
        p.pending = null
        effects(core.complete(requireNotNull(p.link), token, success, clock()))
    }
    fun enqueue(id: Long, bytes: ByteArray, traffic: TransportTraffic, cookie: ULong): Boolean = guarded(false) {
        val link = peers[id]?.link ?: return@guarded false
        effects(core.enqueue(link, bytes, traffic, cookie, clock()))
        true
    }
    /** Caller supplies a newly unlocked session, never retained by the driver. */
    fun proof(id: Long, provider: IdentityKeySession): Boolean {
        try { return guarded(false) {
            val link = peers[id]?.link ?: return@guarded false
            try { effects(core.prepareProof(link, provider, clock())); true } catch (_: TransportException.Busy) { false }
        } } finally { provider.invalidate() }
    }
    fun connections(): List<ConnectionInfo> = guarded(emptyList()) {
        val observations = core.observations(clock()).associateBy { it.link }
        peers.values.map { p ->
            val observation = observations[p.link]
            ConnectionInfo(p.id, p.address, p.born, observation?.firstValidMs, observation?.novelty?.toInt())
        }
    }
    fun power(setting: TransportPowerSetting?, battery: Int, charging: Boolean, visible: Int): TransportPower? = guarded(null) {
        if (battery !in 0..100) return@guarded null
        val result = core.updatePower(setting, battery.toUByte(), charging, visible.coerceIn(0, 64).toUShort(), clock())
        linkLimit = result.linkLimit.toInt()
        peers.values.filter { it.admission in result.cancelledAdmissions }.map { it.id }.forEach(::close)
        effects(result.effects)
        result
    }
    fun tick() = guarded(Unit) {
        val now = clock()
        peers.values.filter { it.link == null && now >= it.born + 30_000uL }.map { it.id }.forEach(::close)
        effects(core.tick(now))
    }
    private fun effects(batch: TransportEffects) {
        for (e in batch.events) {
            val link = when (e) {
                is TransportEvent.Admitted -> e.link
                is TransportEvent.Received -> e.link
                is TransportEvent.Finished -> e.link
                is TransportEvent.Closed -> e.link
            }
            val p = peers.values.firstOrNull { it.link == link } ?: continue
            if (e is TransportEvent.Closed) { peers.remove(p.id); p.early.clear(); radio.close(p.id) }
            event(p.id, e)
        }
        for (send in batch.sends) {
            val p = peers.values.firstOrNull { it.link == send.link } ?: continue
            check(p.pending == null && send.bytes.size <= p.capacity && send.bytes.size <= 512)
            p.pending = send.token
            val submitted = try {
                val submit = { radio.send(p.id, send.path, send.bytes) }
                protectedEgress?.invoke(send, submit) ?: (!core.messageNeedsAuthorization(send.link, send.token) && submit())
            } catch (_: Exception) { false }
            if (!submitted) completed(p.id, false)
        }
    }
    fun lost(id: Long) = guarded(Unit) { close(id) }
    private fun close(id: Long) {
        val p = peers.remove(id) ?: return
        p.early.clear()
        try {
            if (p.link == null) core.cancelConnection(p.admission, clock())
            else {
                val batch = core.disconnect(requireNotNull(p.link), clock())
                batch.events.forEach { event(id, it) }
            }
        } finally { radio.close(id) }
    }
    fun stop() {
        if (stopped) return
        stopped = true
        for (id in peers.keys.toList()) {
            try { close(id) } catch (_: TransportException) { /* native close is in finally */ }
        }
    }
    private inline fun <T> guarded(fallback: T, work: () -> T): T = try { work() } catch (e: TransportException.Refused) {
        effects(e.effects); fallback
    } catch (_: TransportException.Invalid) { fallback
    } catch (_: TransportException.Stale) { fallback
    } catch (_: TransportException) { stop(); fallback }
}
