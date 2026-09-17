package org.meshchat.ui

import android.app.Application
import android.content.Context
import android.content.Intent
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.meshchat.identity.*
import org.meshchat.storage.*
import org.meshchat.transport.*
import uniffi.meshchat_core.*
import java.security.SecureRandom
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean

data class ChatRow(val message: ChannelMessage, val sendState: String)
data class MeshScreenState(
    val loading: Boolean = true, val onboarded: Boolean = false, val locked: Boolean = false,
    val nickname: String = "", val avatar: UByte = 1u, val light: Boolean = false,
    val channels: List<ChannelInfo> = emptyList(), val selected: ChannelInfo? = null,
    val rows: List<ChatRow> = emptyList(), val peers: Int = 0,
    val status: String = "Nearby connection is off", val error: String? = null,
    val waitSeconds: Int = 0, val muted: Boolean = false,
    val previews: Map<String, String> = emptyMap(), val unread: Map<String, Int> = emptyMap(),
    val posted: Long = 0,
    val power: TransportPowerSetting = TransportPowerSetting.AUTO,
)

/** A bounded serial feature owner. Protected adapters open/close the store per
 * operation. UI snapshots never carry a key, database handle or signing session. */
class MeshModel(private val context: Context) {
    var screen by mutableStateOf(MeshScreenState()); private set
    private val main = Handler(Looper.getMainLooper())
    private val queue = ThreadPoolExecutor(1, 1, 0, TimeUnit.MILLISECONDS, ArrayBlockingQueue(64))
    private val vault = StorageVault.android(context)
    private val identity = IdentityProvider.android(context, vault)
    private val storage = EncryptedStorage(identity, vault)
    private var owner: NativeChannels? = null
    private var radio: AndroidGattRadio? = null
    private var starting = false
    private var epoch = 0L
    private var cookie = 3uL
    private var profile = ""
    private var avatar: UByte = 1u
    private var light = false
    private var power = TransportPowerSetting.AUTO
    private var joined = mutableListOf("#general", "#event updates", "#confessions")
    private val muted = mutableSetOf<String>()
    private var selected: String? = null
    private var loaded = false
    private var replacement = false
    private val links = mutableMapOf<Long, LinkHandle>()
    private val receipts = linkedMapOf<String, String>()
    private val pending = mutableMapOf<ULong, Pair<String, MutableSet<Long>>>()
    private var status = "Nearby connection is off"
    private var notice: String? = null
    private var announceAt = 0uL
    private val previews = mutableMapOf<String, String>()
    private val unread = mutableMapOf<String, Int>()
    private var posted = 0L
    private val random = SecureRandom()
    private val overflow = AtomicBoolean(false)
    private fun now() = SystemClock.elapsedRealtime().toULong()
    private fun key(id: ByteArray) = id.joinToString("") { "%02x".format(it.toInt() and 255) }
    private fun publish(value: MeshScreenState) { main.post { screen = value } }
    private fun work(action: () -> Unit) {
        try { queue.execute {
            if (overflow.getAndSet(false)) { unavailable(); return@execute }
            try { action() }
            catch (_: ChannelException.Limited) { report("You can post again when the countdown ends.") }
            catch (_: ChannelException.Invalid) { report("Check the channel words and text byte limits. Unsupported invisible characters are not allowed.") }
            catch (_: IdentityProviderException) { unavailable() }
            catch (_: StorageException) { unavailable() }
            catch (_: ChannelException.Unavailable) { unavailable() }
            catch (_: Exception) { unavailable() }
        } } catch (_: RejectedExecutionException) { overflow.set(true) }
    }
    private fun report(error: String) {
        if (!loaded) { main.post { screen = screen.copy(error = error) }; return }
        try { refresh(error) } catch (_: Exception) { unavailable() }
    }
    private fun setting(name: String) = storage.getRecord(RecordKind.SETTING, name.toByteArray())?.toString(Charsets.UTF_8)
    private fun put(name: String, value: String) = storage.putRecord(RecordKind.SETTING, name.toByteArray(), value.toByteArray())
    private fun persist() {
        put("ui-profile-v1", "$avatar|${if(light) 1 else 0}|$profile")
        put("ui-channels-v1", joined.joinToString("\n"))
        put("ui-muted-v1", muted.joinToString("\n"))
        put("ui-power-v1", power.name)
    }
    fun load() = work {
        if (loaded) { refresh(); return@work }
        val info = try { identity.load() } catch (e: IdentityProviderException) {
            if (e.failure == IdentityFailure.MISSING) { publish(MeshScreenState(loading = false)); return@work }; throw e
        }
        storage.reopen()
        val saved = setting("ui-profile-v1")?.split('|', limit = 3)
        if (saved == null || saved.size != 3) { unavailable(); return@work }
        avatar = saved[0].toUByte(); light = saved[1] == "1"; profile = channelNickname(saved[2])
        power = setting("ui-power-v1")?.let { TransportPowerSetting.valueOf(it) } ?: TransportPowerSetting.AUTO
        joined = (setting("ui-channels-v1")?.split('\n') ?: joined).take(32).map { channelInfo(it).name }.distinct().toMutableList()
        muted.clear(); muted.addAll(setting("ui-muted-v1")?.split('\n')?.filter { it in joined } ?: emptyList())
        owner = NativeChannels(info.identity, now()); loaded = true
        previews.clear(); unread.clear()
        for (name in joined) previews[name] = storage.channelHistory(checkNotNull(owner), name, profile).lastOrNull()?.text ?: "Quiet so far"
        refresh()
    }
    /** Called only from the final explicit onboarding action. */
    fun create(nickname: String, chosenAvatar: UByte) = work {
        if (loaded) return@work
        profile = channelNickname(nickname); avatar = chosenAvatar
        val info = if (replacement) identity.load() else identity.create()
        if (!replacement) storage.create()
        owner = NativeChannels(info.identity, now()); loaded = true; persist(); refresh()
        replacement = false
    }
    fun updateProfile(nickname: String, chosenAvatar: UByte, useLight: Boolean) = work {
        profile = channelNickname(nickname); avatar = chosenAvatar; light = useLight; persist(); refresh()
    }
    fun power(value: TransportPowerSetting) = work { power = value; persist(); radio?.powerSetting(value); refresh() }
    fun select(name: String?) = work { notice = null; selected = name?.let { channelInfo(it).name }; selected?.let { unread[it] = 0 }; refresh() }
    fun join(name: String) = work {
        val channel = channelInfo(name)
        if (channel.name !in joined) {
            if (joined.size >= 32) { refresh("You can join up to 32 channels."); return@work }
            joined.add(channel.name); persist()
        }
        selected = channel.name; unread[channel.name] = 0; refresh()
    }
    fun mute() = work {
        selected?.let { if (!muted.add(it)) muted.remove(it) }; persist(); refresh()
    }
    fun leave() = work {
        val name = selected ?: return@work
        if (channelInfo(name).private) { joined.remove(name); muted.remove(name); previews.remove(name); unread.remove(name); selected = null; persist() }
        refresh()
    }
    fun clearHistory() = work {
        selected?.let { storage.deleteHistory(channelInfo(it).id, false) }; refresh()
    }
    fun resetIdentity() = work {
        stopRadio(); owner?.close(); owner = null; identity.reset(); storage.create()
        profile = ""; selected = null; receipts.clear(); loaded = false
        joined = mutableListOf("#general", "#event updates", "#confessions"); muted.clear(); avatar = 1u; light = false
        previews.clear(); unread.clear()
        replacement = true
        power = TransportPowerSetting.AUTO
        // Reset creates a replacement identity; finish its profile explicitly.
        publish(MeshScreenState(loading = false, error = "Identity reset. Choose a new nickname to finish setup."))
    }
    private fun unavailable() {
        stopRadio(); owner?.close(); owner = null; loaded = false; profile = ""; selected = null
        publish(MeshScreenState(loading = false, locked = true,
            error = "Protected data is unavailable. Unlock your device and reopen. If keys were lost, reset identity and local data."))
    }
    private fun refresh(error: String? = notice) {
        if (!loaded) return
        notice = error
        val n = checkNotNull(owner)
        val rows = selected?.let { storage.channelHistory(n, it, profile) } ?: emptyList()
        selected?.let { previews[it] = rows.lastOrNull()?.text ?: "Quiet so far" }
        val wait = selected?.let { n.waitMs(it, false, now()) } ?: 0uL
        publish(MeshScreenState(loading = false, onboarded = true, nickname = profile, avatar = avatar, light = light,
            channels = joined.map(::channelInfo), selected = selected?.let(::channelInfo),
            rows = rows.map { ChatRow(it, receipts[key(it.id)] ?: if(it.own) "Stored locally · delivery unknown" else "Unverified") },
            peers = links.size, status = status, error = error, waitSeconds = ((wait + 999uL) / 1000uL).toInt(), muted = selected in muted,
            previews = previews.toMap(), unread = unread.toMap(), posted = posted, power = power))
    }
    fun send(text: String) = work {
        notice = null
        val name = selected ?: return@work
        if (links.isEmpty() || radio == null) { refresh("Not sent. Connect to nearby people, then retry."); return@work }
        if (pending.size >= 32) { refresh("Not sent. The send queue is full; try again shortly."); return@work }
        val bytes = checkNotNull(owner).compose(name, profile, avatar, text, (System.currentTimeMillis()/1000).toUInt(), now())
        if (submit(bytes)) { posted++; refresh() }
    }
    fun react(message: ChannelMessage, code: UByte) = work {
        val name = selected ?: return@work
        if (links.isEmpty() || radio == null || pending.size >= 32) { refresh("Reaction not sent. No connection or queue space."); return@work }
        val bytes = checkNotNull(owner).reaction(name, message.id, code, message.ownReaction == code, now())
        submit(bytes)
    }
    private fun submit(bytes: ByteArray): Boolean {
        val radio = radio ?: return false
        val id = key(bytes.copyOfRange(4,12)); val token = cookie++
        val accepted = links.keys.filter { radio.send(it, bytes, TransportTraffic.OWN, token) }.toMutableSet()
        if (accepted.isEmpty()) { refresh("Not sent. The mesh queue refused this message; retry shortly."); return false }
        pending[token] = id to accepted
        if (receipts.size == 100) receipts.remove(receipts.keys.first())
        receipts[id] = "Queued · delivery unknown"
        storage.channelAccept(checkNotNull(owner), null, bytes, TransportIntake.UNVERIFIED, true, now())
        refresh()
        return true
    }
    fun startRadio() = work {
        if (!loaded || radio != null || starting) return@work
        notice = null; starting = true
        val instance = generateSequence { random.nextLong().toULong() }.first { it != 0uL }
        val transport = storage.transport(instance, now()); val generation = ++epoch
        val started = MeshTransportService.start(context, transport, { id, event -> work { if (generation == epoch) this.event(id, event) } },
            { engine -> work { if (generation == epoch) { starting = false; radio = engine; engine.powerSetting(power); announceAt = 0uL; refresh() } else engine.stop() } },
            { state -> work { if (generation == epoch) radioState(state) } })
        if (!started) { starting = false; transport.close(); status = "Allow Bluetooth access, then try connecting again." }
        else status = "Starting nearby connections…"
        refresh()
    }
    fun stop() = work { stopRadio(); refresh() }
    private fun stopPending() {
        for ((message, _) in pending.values) {
            if (receipts[message] != "Handed to mesh · delivery unknown") receipts[message] = "Not sent · retry"
        }
        pending.clear()
    }
    private fun stopRadio() {
        epoch++; starting = false; radio?.stop(); radio = null; links.clear(); stopPending()
        context.stopService(Intent(context, MeshTransportService::class.java)); status = "Nearby connection is off"
    }
    private fun radioState(state: RadioState) {
        if (state == RadioState.LOCKED) { unavailable(); return }
        status = when(state) {
            RadioState.ACTIVE -> "Searching for nearby people…"
            RadioState.PERMISSION_REQUIRED -> "Bluetooth access is needed"
            RadioState.RADIO_DISABLED -> "Turn on Bluetooth to connect"
            RadioState.LOCATION_REQUIRED -> "Allow location access for nearby discovery"
            RadioState.BACKGROUND_LOCATION_REQUIRED -> "Discovery is limited in the background"
            RadioState.DISCOVERY_PAUSED -> "Discovery paused to save power"
            else -> "Nearby connection is off"
        }
        if (state == RadioState.STOPPED || state == RadioState.PERMISSION_REQUIRED || state == RadioState.RADIO_DISABLED) {
            starting = false; radio = null; links.clear(); stopPending()
        }
        refresh()
    }
    private fun event(id: Long, event: TransportEvent) {
        val n = owner ?: return
        when(event) {
            is TransportEvent.Admitted -> { links[id] = event.link; announceAt = 0uL }
            is TransportEvent.Closed -> {
                links.remove(id); n.disconnected(event.link, now())
                val complete = pending.filterValues { (_, waiting) -> waiting.remove(id); waiting.isEmpty() }.keys.toList()
                for (token in complete) pending.remove(token)?.let { (message, _) ->
                    if (receipts[message] != "Handed to mesh · delivery unknown") receipts[message] = "Not sent · retry"
                }
            }
            is TransportEvent.Received -> {
                // Transport has already charged ingress and owns dedup. No encrypted
                // content becomes a plaintext UI item or trusted badge here.
                if (event.intake != TransportIntake.DUPLICATE && event.intake != TransportIntake.DEFERRED_SYNC) {
                    val bytes = event.bytes
                    if (bytes.size >= 26 && bytes[1].toInt() !in listOf(2,3,7)) {
                        for (other in links.keys.filter { it != id }) radio?.send(other, bytes, TransportTraffic.FORWARDED, cookie++)
                    }
                    val channel = joined.firstOrNull { channelInfo(it).id.contentEquals(bytes.copyOfRange(20.coerceAtMost(bytes.size), 24.coerceAtMost(bytes.size))) }
                    if (event.intake == TransportIntake.UNVERIFIED && channel != null && storage.channelAccept(n, event.link, bytes, event.intake, false, now())) {
                        previews[channel] = storage.channelHistory(n, channel, profile).lastOrNull()?.text ?: "Quiet so far"
                        if (channel != selected && channel !in muted) unread[channel] = ((unread[channel] ?: 0) + 1).coerceAtMost(999)
                    }
                }
            }
            is TransportEvent.Finished -> pending[event.cookie]?.let { (message, waiting) ->
                if (event.status == TransportStatus.NATIVE_COMPLETE) receipts[message] = "Handed to mesh · delivery unknown"
                waiting.remove(id)
                if (waiting.isEmpty()) {
                    if (receipts[message] != "Handed to mesh · delivery unknown") receipts[message] = "Not sent · retry"
                    pending.remove(event.cookie)
                }
            }
        }
        refresh()
    }
    private val pulse = object : Runnable {
        override fun run() {
            work {
                if (loaded) {
                    if (links.isNotEmpty() && now() >= announceAt) {
                        val bytes = checkNotNull(owner).announce(profile, avatar, links.size.toUByte(), (System.currentTimeMillis()/1000).toUInt())
                        for (id in links.keys) radio?.send(id, bytes, TransportTraffic.LOCAL, cookie++)
                        announceAt = now() + (radio?.currentPower()?.announceMs ?: 30000uL)
                    }
                    if (selected != null) refresh()
                }
            }
            main.postDelayed(this, 1000)
        }
    }
    init { main.post(pulse); load() }
}

class MeshApplication : Application() {
    val model by lazy { MeshModel(this) }
}
