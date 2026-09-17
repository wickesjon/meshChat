package org.meshchat.transport

/** Bounded local observations. Addresses/RSSI/digests are selection hints, not identities. */
data class ConnectionInfo(val id: Long, val address: String, val born: ULong, val validAt: ULong?, val novelty: Int?)
data class Selection(val close: List<Long>, val connect: String?)

class ConnectionPolicy(private val clock: () -> ULong) {
    private class Seen(var at: ULong, var rssi: Int?) {
        var tried = false
        var retryAt = 0uL
        var blockedUntil = 0uL
        var novelty: Int? = null
        var noveltyAt = 0uL
    }
    private val seen = linkedMapOf<String, Seen>()
    private var rotationAt = clock() + 60_000uL
    private var blockedOverflowUntil = 0uL
    val records: Int get() = seen.size
    fun visible(peers: List<ConnectionInfo> = emptyList()): Int =
        (seen.filterValues { clock() - it.at < 60_000uL }.keys + peers.map { it.address }).toSet().size.coerceAtMost(64)
    private fun record(address: String): Seen? {
        if (address.length != 17) return null
        seen[address]?.let { return it }
        val now = clock()
        if (seen.size == 64) {
            val expired = seen.entries.firstOrNull { now - it.value.at >= 300_000uL && now >= it.value.blockedUntil }
                ?: return null
            seen.remove(expired.key)
        }
        return Seen(now, null).also { seen[address] = it }
    }
    fun discovered(address: String, rssi: Int?): Boolean {
        val item = record(address) ?: return false
        item.at = clock()
        item.rssi = rssi?.takeIf { it in -127..20 }
        return true
    }
    fun clearRssi() { seen.values.forEach { it.rssi = null } }
    fun allowed(address: String, peers: List<ConnectionInfo>, limit: Int, inbound: Boolean = false): Boolean {
        val now = clock()
        val item = record(address) ?: return false
        if (now < blockedOverflowUntil || now < item.blockedUntil || (!inbound && now < item.retryAt) || peers.size >= limit) return false
        val rssi = item.rssi
        // Fixed ten-dB bins are an advisory heuristic, not an identity/attacker test.
        if (rssi != null && peers.count { seen[it.address]?.rssi?.let { value -> Math.floorDiv(value, 10) == Math.floorDiv(rssi, 10) } == true } >= 3) return false
        return true
    }
    fun attempted(address: String) { record(address)?.let { it.tried = true; it.retryAt = clock() + 5_000uL } }
    fun disconnected(address: String) { seen[address]?.let { it.retryAt = maxOf(it.retryAt, clock() + 5_000uL) } }
    fun plan(peers: List<ConnectionInfo>, limit: Int): Selection {
        val now = clock()
        val idle = peers.filter { (it.validAt == null || it.validAt > it.born + 20_000uL) && now - it.born >= 20_000uL }
        idle.forEach { peer ->
            val item = record(peer.address)
            if (item == null) blockedOverflowUntil = maxOf(blockedOverflowUntil, now + 300_000uL)
            else item.blockedUntil = now + 300_000uL
        }
        if (idle.isNotEmpty()) return Selection(idle.map { it.id }, null)
        peers.forEach { peer -> seen[peer.address]?.let { it.novelty = peer.novelty; it.noveltyAt = now } }
        val reevaluate = now >= rotationAt
        if (reevaluate) {
            rotationAt = now + 60_000uL
            val crowded = peers.filter { seen[it.address]?.rssi != null }
                .groupBy { Math.floorDiv(requireNotNull(seen[it.address]?.rssi), 10) }.values
                .flatMap { group -> group.sortedWith(compareByDescending<ConnectionInfo> { it.novelty ?: -1 }
                    .thenByDescending { seen[it.address]?.rssi ?: -128 }.thenBy { it.born }).drop(3) }
            if (crowded.isNotEmpty()) {
                crowded.forEach { seen[it.address]?.retryAt = now + 60_000uL }
                return Selection(crowded.map { it.id }, null)
            }
        }
        val connected = peers.map { it.address }.toSet()
        val candidates = seen.entries.filter { (address, item) ->
            address !in connected && now - item.at < 60_000uL && now >= item.retryAt && now >= item.blockedUntil
        }.sortedWith(compareByDescending<Map.Entry<String, Seen>> { !it.value.tried }
            .thenByDescending { if (now - it.value.noveltyAt < 60_000uL) it.value.novelty ?: -1 else -1 }
            .thenByDescending { it.value.rssi ?: -128 }.thenBy { it.key })
        if (peers.size < limit) {
            val candidate = candidates.firstOrNull { allowed(it.key, peers, limit) }
            return Selection(emptyList(), candidate?.key)
        }
        if (!reevaluate) return Selection(emptyList(), null)
        if (candidates.isEmpty()) return Selection(emptyList(), null)
        val reserve = minOf(2, (limit - 1).coerceAtLeast(0))
        // Keep the best locally observed stable slots; ties preserve older links.
        val explorers = peers.sortedWith(compareByDescending<ConnectionInfo> { it.novelty ?: -1 }
            .thenByDescending { seen[it.address]?.rssi ?: -128 }.thenBy { it.born }).takeLast(reserve)
        val ranked = explorers.sortedWith(compareBy<ConnectionInfo> { it.novelty ?: -1 }
            .thenBy { seen[it.address]?.rssi ?: -128 }.thenBy { it.id })
        val victims = ranked.take(minOf(reserve, candidates.size))
        val retained = peers.filter { peer -> victims.none { it.id == peer.id } }
        val candidate = candidates.firstOrNull { allowed(it.key, retained, limit) } ?: return Selection(emptyList(), null)
        victims.forEach { seen[it.address]?.retryAt = now + 60_000uL }
        return Selection(victims.map { it.id }, candidate.key)
    }
}

enum class ScanMode { OFF, BALANCED, BURST }
/** Suspend-inclusive time; no catch-up scan bursts after a late tick. */
class ScanPolicy(private val clock: () -> ULong) {
    private var isolatedAt = clock()
    private var phaseAt = clock()
    private var wasForeground = true
    private var saver = false
    private var retryAt = 0uL
    private var failures = 0
    val retrying: Boolean get() = clock() < retryAt
    fun discovered() { isolatedAt = clock() }
    fun failed() { failures = minOf(failures + 1, 5); retryAt = clock() + minOf(60_000uL, 5_000uL * (1uL shl (failures - 1))) }
    fun received() { failures = 0 }
    fun mode(isSaver: Boolean, visible: Int, foreground: Boolean, permitted: Boolean): ScanMode {
        val now = clock()
        if (foreground && !wasForeground) { isolatedAt = now; phaseAt = now }
        wasForeground = foreground
        if (saver != isSaver) { saver = isSaver; phaseAt = now }
        if (!permitted || now < retryAt) return ScanMode.OFF
        if (isSaver) return if ((now - phaseAt) % 60_000uL < 10_000uL) {
            if (visible == 0) ScanMode.BURST else ScanMode.BALANCED
        } else ScanMode.OFF
        if (visible > 0) return ScanMode.BALANCED
        val isolated = now - isolatedAt
        val period = if (isolated >= 900_000uL) 60_000uL else if (isolated >= 300_000uL) 30_000uL else 20_000uL
        return if ((now - phaseAt) % period < 10_000uL) ScanMode.BURST
            else if (isolated < 300_000uL) ScanMode.BALANCED else ScanMode.OFF
    }
}

enum class RadioState { ACTIVE, DISCOVERY_PAUSED, BACKGROUND_LOCATION_REQUIRED, LOCATION_REQUIRED, PERMISSION_REQUIRED, RADIO_DISABLED, LOCKED, STOPPED }

/** The manifest deliberately does not assert neverForLocation. */
fun discoveryRestriction(api: Int, foreground: Boolean, fine: Boolean, locationEnabled: Boolean, background: Boolean): RadioState? =
    if (!fine || !locationEnabled) RadioState.LOCATION_REQUIRED
    else if (api <= 30 && !foreground && !background) RadioState.BACKGROUND_LOCATION_REQUIRED else null
