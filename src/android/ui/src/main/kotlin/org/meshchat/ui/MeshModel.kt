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
import org.meshchat.billing.*
import uniffi.meshchat_core.*
import java.security.SecureRandom
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicBoolean

data class DirectThread(val keys: ByteArray, val petname: String, val archived: Boolean = false)
data class DirectRow(val message: DirectMessage, val sendState: String)
data class ChatRow(val message: ChannelMessage, val sendState: String)
data class MeshScreenState(
    val friends: List<FriendCard> = emptyList(), val friendCode: FriendProposal? = null,
    val proposal: FriendProposal? = null, val proposalScanned: Boolean = false, val replacingFriend: FriendCard? = null,
    val direct: DirectThread? = null, val directRows: List<DirectRow> = emptyList(), val archives: List<FriendProposal> = emptyList(),
    val loading: Boolean = true, val onboarded: Boolean = false, val locked: Boolean = false,
    val nickname: String = "", val avatar: UByte = 1u, val light: Boolean = false,
    val supporter: Boolean = false, val theme: String = "afterhours", val nicknameRgb: UInt? = null,
    val billingStatus: String = "Purchases are not configured in this build.", val supporterPrice: String? = null,
    val channels: List<ChannelInfo> = emptyList(), val selected: ChannelInfo? = null,
    val channelProposal: ChannelInfo? = null,
    val rows: List<ChatRow> = emptyList(), val peers: Int = 0,
    val status: String = "Nearby connection is off", val error: String? = null,
    val waitSeconds: Int = 0, val muted: Boolean = false,
    val previews: Map<String, String> = emptyMap(), val unread: Map<String, Int> = emptyMap(),
    val posted: Long = 0,
    val power: TransportPowerSetting = TransportPowerSetting.AUTO,
    val beaconRequested: Boolean = false, val autoBeacon: Boolean = false, val beacon: BeaconStatus? = null,
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
    private var transport: NativeTransport? = null
    private var cards = emptyList<FriendCard>()
    private var myCode: FriendProposal? = null
    private var proposal: FriendProposal? = null
    private var proposalScanned = false
    private var channelProposal: ChannelInfo? = null
    private var replacingFriend: FriendCard? = null
    private var direct: DirectThread? = null
    private var archives = emptyList<FriendProposal>()
    private var radio: AndroidGattRadio? = null
    private var starting = false
    private var epoch = 0L
    private var cookie = 3uL
    private var profile = ""
    private var avatar: UByte = 1u
    private var light = false
    private var supporter = false
    private var theme = "afterhours"
    private var nicknameRgb: UInt? = null
    private var billingStatus = "Purchases are not configured in this build."
    private var supporterPrice: String? = null
    private var power = TransportPowerSetting.AUTO
    private var beaconRequested = false
    private var autoBeacon = false
    private var beaconExits = 0uL
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
            catch (_: MessagingException.Invalid) { report("Check the friend code or message. No friend was added.") }
            catch (_: MessagingException.Stale) { report("This friend identity changed or is awaiting replacement. Scan and confirm its code again.") }
            catch (_: MessagingException.Busy) { report("Not sent. Connect to nearby people or wait for queue space, then retry.") }
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
        put("ui-beacon-v1", "${if(beaconRequested)1 else 0}|${if(autoBeacon)1 else 0}")
        put("ui-cosmetics-v1", "$theme|${nicknameRgb?.toString() ?: ""}")
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
        supporter = setting("supporter-v1") == "1"
        val cosmetics = setting("ui-cosmetics-v1")?.split('|', limit=2)
        theme = cosmetics?.firstOrNull() ?: if(light)"daylight" else "afterhours"
        nicknameRgb = cosmetics?.getOrNull(1)?.toUIntOrNull()?.takeIf { it<=0xffffffu }
        light = Themes.selected(theme,supporter).light
        power = setting("ui-power-v1")?.let { TransportPowerSetting.valueOf(it) } ?: TransportPowerSetting.AUTO
        val beaconSetting = setting("ui-beacon-v1")?.split('|')
        beaconRequested = beaconSetting?.getOrNull(0)=="1"; autoBeacon=beaconSetting?.getOrNull(1)=="1"; beaconExits=0uL
        joined = (setting("ui-channels-v1")?.split('\n') ?: joined).take(33).map { channelInfo(it).name }.distinct().toMutableList()
        muted.clear(); muted.addAll(setting("ui-muted-v1")?.split('\n')?.filter { it in joined } ?: emptyList())
        owner = NativeChannels(info.identity, now()); initializeMessaging(); loaded = true
        previews.clear(); unread.clear()
        for (name in joined) previews[name] = coreWork { storage.messagingHistory(it, checkNotNull(owner), name, profile) }.lastOrNull()?.text ?: "Quiet so far"
        refresh()
    }
    /** Called only from the final explicit onboarding action. */
    fun create(nickname: String, chosenAvatar: UByte) = work {
        if (loaded) return@work
        profile = channelNickname(nickname); avatar = chosenAvatar
        val info = if (replacement) identity.load() else identity.create()
        if (!replacement) storage.create()
        owner = NativeChannels(info.identity, now()); initializeMessaging(); loaded = true; persist(); refresh()
        replacement = false
    }
    fun updateProfile(nickname: String, chosenAvatar: UByte, useLight: Boolean, chosenTheme: String? = null, color: UInt? = null) = work {
        profile = channelNickname(nickname); avatar = chosenAvatar
        theme = Themes.selected(chosenTheme ?: if(useLight)"daylight" else "afterhours",supporter).id
        light = Themes.selected(theme,supporter).light
        nicknameRgb = color?.takeIf { supporter && it<=0xffffffu }
        persist(); myCode = storage.friendCode(profile); refresh()
    }
    /** Native store callback; acknowledgement is allowed only after protected save.
     * Store failure changes no transport or authentication decision. */
    fun storeResult(result: StoreResult, message: String, saved: (Boolean)->Unit) = work {
        if(!loaded) { main.post { saved(false) };return@work }
        val next = EntitlementPolicy.active(supporter,result)
        try { put("supporter-v1",if(next)"1" else "0") }
        catch (_: Exception) {
            billingStatus="Unable to save Supporter access. Try Restore when storage reopens."
            main.post { screen=screen.copy(billingStatus=billingStatus);saved(false) };return@work
        }
        supporter=next;light=Themes.selected(theme,supporter).light;billingStatus=message
        refresh();main.post { saved(true) }
    }
    fun storePrice(price: String?) = work { supporterPrice=price;refresh() }
    private fun canJoin(channel: ChannelInfo): Boolean = !channel.private ||
        EntitlementPolicy.canAddPrivate(supporter,joined.count { channelInfo(it).private })
    private fun slotNotice() = "Private channel slots are full. Leave one to add another. Existing channels stay available."
    fun power(value: TransportPowerSetting) = work {
        beaconRequested=false; autoBeacon=false; power=value; persist()
        radio?.configureBeacon(false,false); radio?.powerSetting(value); refresh()
    }
    fun beacon(manual: Boolean, automatic: Boolean) = work {
        beaconRequested=manual; autoBeacon=automatic; power=TransportPowerSetting.AUTO
        persist(); radio?.configureBeacon(manual,automatic); refresh()
    }
    fun exitBeacon() = beacon(false,false)
    fun select(name: String?) = work { notice = null; direct = null; selected = name?.let { channelInfo(it).name }; selected?.let { unread[it] = 0 }; refresh() }
    fun join(name: String) = work {
        val channel = channelInfo(name)
        if (channel.name !in joined) {
            if (!canJoin(channel)) { refresh(slotNotice()); return@work }
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
        stopRadio(); clearMessaging(); owner?.close(); owner = null; identity.reset(); storage.create()
        profile = ""; selected = null; receipts.clear(); loaded = false
        joined = mutableListOf("#general", "#event updates", "#confessions"); muted.clear(); avatar = 1u; light = false
        supporter=false;theme="afterhours";nicknameRgb=null
        previews.clear(); unread.clear()
        replacement = true
        power = TransportPowerSetting.AUTO; beaconRequested=false; autoBeacon=false; beaconExits=0uL
        // Reset creates a replacement identity; finish its profile explicitly.
        publish(MeshScreenState(loading = false, error = "Identity reset. Choose a new nickname to finish setup."))
    }
    private fun unavailable() {
        stopRadio(); clearMessaging(); owner?.close(); owner = null; loaded = false; profile = ""; selected = null
        publish(MeshScreenState(loading = false, locked = true,
            error = "Protected data is unavailable. Unlock your device and reopen. If keys were lost, reset identity and local data."))
    }
    private fun refresh(error: String? = notice) {
        if (!loaded) return
        notice = error
        val beaconStatus = radio?.beaconStatus()
        if (beaconStatus != null && beaconStatus.batteryExitCount > beaconExits) {
            beaconExits=beaconStatus.batteryExitCount; beaconRequested=false
            notice="Beacon Mode ended at 30% battery. Your channels and history are unchanged."
            persist(); radio?.configureBeacon(false,autoBeacon)
        }
        val n = checkNotNull(owner)
        n.setCosmetics(supporter,if(supporter)nicknameRgb else null)
        val rows = selected?.let { name -> coreWork { storage.messagingHistory(it, n, name, profile) } } ?: emptyList()
        cards = coreWork { storage.friendCards(it, now()) }
        val dmRows = direct?.let { thread -> coreWork { storage.directHistory(it, thread.keys, now()) } } ?: emptyList()
        selected?.let { previews[it] = rows.lastOrNull()?.text ?: "Quiet so far" }
        val wait = selected?.let { n.waitMs(it, false, now()) } ?: 0uL
        publish(MeshScreenState(friends = cards, friendCode = myCode, proposal = proposal, proposalScanned = proposalScanned,
            replacingFriend = replacingFriend, direct = direct, archives = archives.filter { old -> cards.none { it.keys.contentEquals(old.keys) } },
            directRows = dmRows.map { DirectRow(it, receipts[key(it.id)] ?: if(it.own) "Stored locally - delivery unknown" else "Encrypted") },
            loading = false, onboarded = true, nickname = profile, avatar = avatar, light = light,
            supporter = supporter, theme = theme, nicknameRgb = nicknameRgb, billingStatus = billingStatus, supporterPrice = supporterPrice,
            channels = joined.map(::channelInfo), selected = selected?.let(::channelInfo), channelProposal = channelProposal,
            rows = rows.map { ChatRow(it, receipts[key(it.id)] ?: if(it.own) "Stored locally · delivery unknown" else "Unverified") },
            peers = links.size, status = status, error = notice, waitSeconds = ((wait + 999uL) / 1000uL).toInt(), muted = selected in muted,
            previews = previews.toMap(), unread = unread.toMap(), posted = posted, power = power, beaconRequested=beaconRequested, autoBeacon=autoBeacon, beacon=beaconStatus))
    }
    fun send(text: String) = work {
        notice = null
        val name = selected ?: return@work
        if (links.isEmpty() || radio == null) { refresh("Not sent. Connect to nearby people, then retry."); return@work }
        if (pending.size >= 32) { refresh("Not sent. The send queue is full; try again shortly."); return@work }
        val bytes = checkNotNull(owner).compose(name, profile, avatar, text, (System.currentTimeMillis()/1000).toUInt(), now())
        if (channelInfo(name).private && cards.isNotEmpty()) {
            if (submitProtected { core, token -> storage.sendSigned(core, bytes, token, now()) }) posted++
            refresh()
        } else if (submit(bytes)) { posted++; refresh() }
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
        val transport = checkNotNull(transport); val generation = ++epoch
        val started = MeshTransportService.start(context, transport, { id, event -> work { if (generation == epoch) this.event(id, event) } },
            { engine -> work { if (generation == epoch) { starting = false; radio = engine; engine.powerSetting(power); if(beaconRequested || autoBeacon)engine.configureBeacon(beaconRequested,autoBeacon); announceAt = 0uL; refresh() } else engine.stop() } },
            { state -> work { if (generation == epoch) radioState(state) } },
            { send, submit -> storage.messageEgress(transport, send, submit) })
        if (!started) { starting = false; status = "Allow Bluetooth access, then try connecting again." }
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
            is TransportEvent.Admitted -> {
                links[id] = event.link; announceAt = 0uL
                radio?.operation { core -> storage.proof(core, event.link, now()) }
            }
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
                    if (event.intake == TransportIntake.PENDING) {
                        coreWork { storage.authenticate(it, event.link, bytes, now()) }
                    }
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
    private fun initializeMessaging() {
        transport?.close()
        val instance=generateSequence { random.nextLong().toULong() }.first { it!=0uL }
        transport=storage.transport(instance,now()); myCode=storage.friendCode(profile); archives=storage.archives()
    }
    private fun clearMessaging() {
        channelProposal=null
        transport?.close(); transport=null; cards=emptyList(); myCode=null; proposal=null
        proposalScanned=false; replacingFriend=null; direct=null; archives=emptyList()
    }
    /** Read/auth work shares the radio monitor with ticks, native callbacks and
     * protected sends; timestamp collection occurs inside that serialization. */
    private fun <T> coreWork(action: (NativeTransport)->T): T {
        val active=radio ?: return action(checkNotNull(transport))
        var result: Result<T>?=null
        active.operation { core -> result=runCatching { action(core) }; TransportEffects(emptyList(),emptyList()) }
        return (result ?: throw MessagingException.Busy()).getOrThrow()
    }
    private fun submitProtected(action: (NativeTransport, ULong)->MessageSubmission): Boolean {
        val active=radio ?: return false
        if(pending.size>=32) {notice="Not sent. The send queue is full.";return false}
        val token=cookie++;var result: MessageSubmission?=null
        active.operation { core -> action(core,token).also { result=it }.effects }
        val sent=result ?: return false
        val id=key(sent.id)
        if(receipts.size>=100)receipts.remove(receipts.keys.first())
        receipts[id]=if(sent.queued) "Queued - delivery unknown" else "Stored locally - not sent"
        if(sent.queued)pending[token]=id to links.filterValues {it in sent.queuedLinks}.keys.toMutableSet()
        return sent.queued
    }
    fun friendInput(uri: String, scanned: Boolean = false) = work {
        channelProposal=null; proposal=null; proposalScanned=false
        val decoded=friendProposal(uri)
        if(replacingFriend!=null && !scanned) {report("Replacement requires a fresh scan of the new device's code.");return@work}
        proposal=decoded;proposalScanned=scanned;notice=null;refresh()
    }
    fun cancelProposal() = work {proposal=null;proposalScanned=false;refresh()}
    fun shareInput(uri: String, scanned: Boolean = false) = work {
        channelProposal=null; proposal=null; proposalScanned=false
        val channel=try { channelLink(uri) } catch (_: ChannelException.Invalid) { null }
        if(channel!=null) {
            channelProposal=channel;notice=null;refresh()
        } else {
            val decoded=try {friendProposal(uri)} catch (_: MessagingException.Invalid) {
                report("Check the channel or friend link. Nothing was joined or pinned.");return@work
            }
            if(replacingFriend!=null && !scanned) {report("Replacement requires a fresh scan of the new device's code.");return@work}
            proposal=decoded;proposalScanned=scanned;notice=null;refresh()
        }
    }
    fun cancelChannelProposal() = work {channelProposal=null;refresh()}
    fun confirmChannel(name: String) = work {
        if(!loaded || channelProposal?.name!=name)return@work
        if(name !in joined) {
            if(!canJoin(channelInfo(name))) {refresh(slotNotice());return@work}
            joined.add(name);persist()
        }
        channelProposal=null;direct=null;selected=name;unread[name]=0;refresh()
    }
    fun scannerUnavailable() = work {report("Camera access is unavailable. Enter channel words or paste a link and verify its source. Friend replacement still requires a fresh scan.")}
    fun confirmFriend(petname: String) = work {
        val decoded=proposal ?: return@work
        if(replacingFriend!=null && !proposalScanned) {report("Scan the replacement device's code first.");return@work}
        stopRadio()
        storage.confirmFriend(checkNotNull(transport),decoded.uri,channelNickname(petname),replacingFriend?.handle,now())
        proposal=null;proposalScanned=false;replacingFriend=null;archives=storage.archives()
        refresh("Friend pinned. Show them your code so it is mutual. Reconnect when ready.")
    }
    fun changeFriend(friend: FriendCard, replace: Boolean) = work {
        if(!storage.archiveFriend(friend)) {report("Old conversation list is full. Delete an old conversation before changing this friend.");return@work}
        stopRadio()
        storage.changeFriend(checkNotNull(transport),friend.handle,replace,now())
        archives=storage.archives();proposal=null;proposalScanned=false;direct=null
        replacingFriend=if(replace)friend else null
        refresh(if(replace)"Sending to the old identity is suspended. Scan the new device, then compare both fingerprints." else "Friend removed. Old history is kept separately. Reconnect when ready.")
    }
    fun openDirect(friend: FriendCard) = work { notice=null;selected=null;direct=DirectThread(friend.keys,friend.petname);refresh() }
    fun openArchive(friend: FriendProposal) = work {notice=null;selected=null;direct=DirectThread(friend.keys,friend.nickname,true);refresh()}
    fun closeDirect() = work {direct=null;refresh()}
    fun deleteOldConversation() = work {
        val old=direct ?: return@work
        if(!old.archived)return@work
        storage.deleteArchive(old.keys);archives=storage.archives();direct=null;refresh()
    }
    fun sendDirect(value: String) = work {
        val thread=direct ?: return@work
        val pin=cards.firstOrNull {it.keys.contentEquals(thread.keys) && !it.replacing}
        if(thread.archived || pin==null) {report("This old identity has no active pin. There is no plaintext fallback.");return@work}
        if(radio==null || links.isEmpty()) {report("Not sent. Connect to nearby people, then retry.");return@work}
        if(submitProtected {core,token->storage.sendDirect(core,pin.handle,DirectContent.Chat(value),token,now())})posted++
        refresh()
    }
    fun reactDirect(message: DirectMessage, code: UByte) = work {
        val thread=direct ?: return@work
        val pin=cards.firstOrNull {it.keys.contentEquals(thread.keys) && !it.replacing}
        if(thread.archived || pin==null || radio==null || links.isEmpty()) {report("Reaction not sent. This thread needs an active friend pin and connection.");return@work}
        submitProtected {core,token->storage.sendDirect(core,pin.handle,DirectContent.Reaction(message.id,message.ownReaction==code,code),token,now())}
        refresh()
    }
    private val pulseQueued = AtomicBoolean(false)
    private val pulse = object : Runnable {
        override fun run() {
            if(pulseQueued.compareAndSet(false,true)) work {
                try { if (loaded) {
                    if(links.isNotEmpty()) coreWork {storage.retryMessages(it,now())}
                    if (links.isNotEmpty() && now() >= announceAt) {
                        val bytes = checkNotNull(transport).beaconAnnounce(checkNotNull(owner).announce(profile, avatar, links.size.toUByte(), (System.currentTimeMillis()/1000).toUInt()))
                        submitProtected { core, token -> storage.sendSigned(core, bytes, token, now()) }
                        announceAt = now() + (radio?.currentPower()?.announceMs ?: 30000uL)
                    }
                    if (selected != null || direct != null || links.isNotEmpty() || beaconRequested || autoBeacon) refresh()
                } } finally {pulseQueued.set(false)}
            }
            main.postDelayed(this, 1000)
        }
    }
    init { main.post(pulse); load() }
}

class MeshApplication : Application() {
    val model by lazy { MeshModel(this) }
}
