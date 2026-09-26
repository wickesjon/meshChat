package org.meshchat.ui

import android.app.KeyguardManager
import android.os.Bundle
import android.os.SystemClock
import android.view.WindowManager
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity
import org.meshchat.storage.EncryptedStorage
import org.meshchat.transport.AndroidGattRadio
import org.meshchat.transport.GattDriver
import uniffi.meshchat_core.*
import java.nio.ByteBuffer
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/** Injected source cache/age fixture; real paired Bluetooth request, transfer
 * and protected history processing. Existing identity/history are preserved.
 */
class PhysicalCatchupTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private lateinit var activity: MainActivity
    private lateinit var model: MeshModel
    private lateinit var run: String
    private lateinit var role: String
    private var traced=true
    private fun now() = SystemClock.elapsedRealtime().toULong()
    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)
    private fun field(name: String) = MeshModel::class.java.getDeclaredField(name).apply { isAccessible=true }
    private fun <T> serial(action: () -> T): T =
        (field("queue").get(model) as ThreadPoolExecutor).submit<T> {
            synchronized(checkNotNull(field("coreGate").get(model))) { action() }
        }.get(60, TimeUnit.SECONDS)
    private fun screen(): MeshScreenState {
        lateinit var value: MeshScreenState
        main { value=model.screen }
        return value
    }
    private fun log(event: String, vararg fields: Pair<String, Any>) {
        val json=JSONObject().put("run",run).put("role",role).put("event",event)
            .put("mono_ms",SystemClock.elapsedRealtime()).put("utc_ms",System.currentTimeMillis())
        for ((key,value) in fields) json.put(key,value)
        android.util.Log.i("MC025C",json.toString())
        instrumentation.sendStatus(0,Bundle().apply {putString("stream","MC025C $json\n")})
    }
    private fun await(timeout: Long, condition: (MeshScreenState) -> Boolean): Boolean {
        val end=SystemClock.elapsedRealtime()+timeout
        var nextLog=0L
        do {
            assertFalse(activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
            val state=screen();assertFalse("Protected profile unavailable",state.locked)
            traceRadio()
            if(condition(state))return true
            if(SystemClock.elapsedRealtime()>=nextLog) {
                log("waiting", "peers" to state.peers, "status" to state.status,
                    "catchup" to (state.catchup ?: "none"),
                    "fixture_rows" to state.rows.count {it.message.text.startsWith("MC25C${run}_")},
                    "rx_frames" to (state.contribution?.receivedFrames?.toString() ?: "none"),
                    "tx_frames" to (state.contribution?.completedFrames?.toString() ?: "none"))
                nextLog=SystemClock.elapsedRealtime()+15000
            }
            SystemClock.sleep(100)
        } while(SystemClock.elapsedRealtime()<end)
        return false
    }
    @Suppress("UNCHECKED_CAST")
    private fun traceRadio() {
        if(traced)return
        serial {
            val radio=field("radio").get(model) as AndroidGattRadio? ?: return@serial
            val driverField=AndroidGattRadio::class.java.getDeclaredField("driver").apply {isAccessible=true}
            val driver=driverField.get(radio) as GattDriver
            val eventField=GattDriver::class.java.getDeclaredField("event").apply {isAccessible=true}
            val original=eventField.get(driver) as (Long,TransportEvent)->Unit
            eventField.set(driver,{ id:Long,event:TransportEvent ->
                if(event is TransportEvent.Received && event.intake==TransportIntake.DEFERRED_SYNC) {
                    val bytes=event.bytes
                    log("sync_rx", "size" to bytes.size,
                        "header" to bytes.take(11).joinToString("") {"%02x".format(it.toInt() and 255)})
                }
                if(event is TransportEvent.Closed)log("link_closed")
                original(id,event)
            })
            val egress=driver.protectedEgress
            driver.protectedEgress={send,submit ->
                val accepted=checkNotNull(egress)(send,submit)
                log("frame_tx", "size" to send.bytes.size, "accepted" to accepted,
                    "header" to send.bytes.take(4).joinToString("") {"%02x".format(it.toInt() and 255)})
                accepted
            }
            traced=true
        }
    }
    private fun text(index: Int) = "MC25C${run}_$index"
    private fun digest(value: String) = MessageDigest.getInstance("SHA-256").digest(value.toByteArray())
    private fun packet(index: Int): ByteArray {
        val bytes=text(index).toByteArray()
        val payload=ByteBuffer.allocate(13+bytes.size)
            .putInt((System.currentTimeMillis()/1000).toInt())
            .put(byteArrayOf(0,1,65,0,0,0,0)).putShort(bytes.size.toShort()).put(bytes).array()
        return byteArrayOf(1,1,0,7)+digest("$run-$index").copyOf(8)+digest("$run-source-${index%3}").copyOf(8)+
            digest("meshfest-v1|#general").copyOf(4)+
            byteArrayOf((payload.size shr 8).toByte(),payload.size.toByte())+payload
    }
    private fun seedLink(core: NativeTransport, at: ULong, indices: IntRange, address: Byte) {
        val admission=core.admitConnection(ByteArray(16){address},at)
        val link=core.nativeReady(admission,TransportRole.CENTRAL,512u,512u,at).link
        val hello=core.tick(at).sends.single()
        assertEquals(59,hello.bytes.size)
        val remote=hello.bytes.copyOf()
        remote[6]=(1-remote[6].toInt()).toByte()
        val seed=ByteArray(64).also {SecureRandom().nextBytes(it)}
        IdentityKeySession.importUnlocked(seed,ByteArray(16){address}).use { identity ->
            identity.publicIdentity().signingKey.copyInto(remote,27)
            identity.invalidate()
        }
        seed.fill(0)
        ByteArray(16).also {SecureRandom().nextBytes(it)}.copyInto(remote,11)
        core.complete(link,hello.token,true,at)
        assertTrue(core.receive(link,remote.size.toULong(),remote,at).events.any {it is TransportEvent.Admitted})
        for(index in indices) {
            val arrival=at+(index-indices.first).toULong()*12_000uL
            val raw=packet(index)
            val frame=byteArrayOf(0,0,(raw.size shr 8).toByte(),raw.size.toByte())+raw
            val effects=core.receive(link,frame.size.toULong(),frame,arrival)
            assertTrue("Fixture item $index must pass native live admission",effects.events.any {it is TransportEvent.Received})
        }
        core.disconnect(link,at+(indices.last-indices.first).toULong()*12_000uL)
    }
    private fun seedSource() = serial {
        assertNull(field("radio").get(model))
        val storage=field("storage").get(model) as EncryptedStorage
        val current=now()
        assertTrue(current>1_020_000uL)
        val start=current-1_020_000uL
        val fixture=storage.transport(SecureRandom().nextLong().toULong().or(1uL),start)
        try {
            seedLink(fixture,start,99..99,41)
            assertEquals(1u,fixture.beaconStatus(start).cachedMessages)
            // Monotonic age preconditioning, not an actual 17-minute outage.
            // Three stable claimed authors, three posts each. Isolate the
            // item cap from single-sender burst refusal; no budget changes.
            seedLink(fixture,now()-120_000uL,0..8,42)
            assertEquals("Expired item must be gone",9u,fixture.beaconStatus(now()).cachedMessages)
            (field("transport").get(model) as NativeTransport).close()
            field("transport").set(model,fixture)
        } catch (failure: Throwable) {fixture.close();throw failure}
        log("fixture", "recent" to 9, "expired" to 1, "authors" to 3, "age_ms" to 1_020_000)
    }
    @Suppress("UNCHECKED_CAST")
    private fun link(): LinkHandle = serial {
        (field("links").get(model) as Map<Long,LinkHandle>).values.single().copy()
    }
    @Test fun newestEightRecoveredOnceWithoutExpiredOrLiveReplay() {
        val args=InstrumentationRegistry.getArguments()
        check(args.getString("physicalCatchup")=="true")
        run=args.getString("runId") ?: error("Unique runId required")
        role=args.getString("role") ?: error("A/B role required")
        traced=args.getString("traceCatchup")!="true"
        check(run.matches(Regex("[A-Za-z0-9]{1,16}")) && role in listOf("A","B"))
        activity=ui.activity;model=(activity.application as MeshApplication).model
        assertFalse(activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        main {activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);model.load()}
        try {
            ui.waitUntil(120000) {screen().onboarded && !screen().locked}
            main {model.stop();model.select("#general")}
            assertTrue(await(120000){it.status=="Nearby connection is off" && it.selected?.name=="#general"})
            assertTrue("Run ID must not match retained history",screen().rows.none {it.message.text.startsWith("MC25C${run}_")})
            if(role=="A")seedSource()
            main {model.startRadio()}
            assertTrue("No physical peer",await(360000){it.peers==1})
            log("connected")
            if(role=="A") {
                assertTrue("Receiver never confirmed bounded recovery",await(360000){state ->
                    state.rows.any {!it.message.own && it.message.text=="MC25C${run}OK"}
                })
                log("receiver_confirmed")
                SystemClock.sleep(5000)
            } else {
                val established=link()
                val start=SystemClock.elapsedRealtime()
                val expected=(1..8).map(::text).toSet()
                assertTrue("Catch-up did not finish",await(180000){state ->
                    state.catchup!=null && state.catchup!="Checking for recent messages…"
                })
                assertEquals("Selected set did not arrive",8,screen().rows.count {it.message.text in expected})
                val duration=SystemClock.elapsedRealtime()-start
                val stableStart=SystemClock.elapsedRealtime()
                var stableChecks=0
                do {
                    SystemClock.sleep(1000);assertEquals(established,link());stableChecks++
                } while(SystemClock.elapsedRealtime()-stableStart<30000)
                val stableDuration=SystemClock.elapsedRealtime()-stableStart
                val state=screen()
                val recovered=state.rows.filter {it.message.text.startsWith("MC25C${run}_")}
                assertEquals(expected,recovered.map {it.message.text}.toSet())
                assertTrue(recovered.all {it.recovered && !it.message.own && !it.message.signed && it.message.verifiedPetname==null})
                val cache=serial {(field("transport").get(model) as NativeTransport).beaconStatus(now()).cachedMessages}
                assertEquals("Recovered data must not enter live forwarding cache",0u,cache)
                assertEquals(0uL,checkNotNull(state.contribution).relayedChatCopies)
                log("recovered", "count" to recovered.size, "observed_ms" to duration,
                    "oldest_excluded" to true, "expired_excluded" to true,
                    "stable_extra_ms" to stableDuration, "stable_checks" to stableChecks,
                    "live_cache" to cache.toString())
                assertTrue(await(30000){it.waitSeconds==0})
                main {model.send("MC25C${run}OK")}
                assertTrue(await(30000){it.rows.any {row -> row.message.own && row.message.text=="MC25C${run}OK" && row.sendState=="Handed to mesh · delivery unknown"}})
            }
        } finally {
            main {model.stop();activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}
            ui.waitUntil(30000){screen().status=="Nearby connection is off"}
        }
    }
}
