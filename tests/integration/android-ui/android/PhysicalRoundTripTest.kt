package org.meshchat.ui

import android.app.KeyguardManager
import android.os.Bundle
import android.os.SystemClock
import android.view.WindowManager
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.lifecycle.Lifecycle
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity

/** Paired, opt-in physical smoke. Uses the installed profile and production send
 * path; never resets counters, alters admission, or assumes a shared clock.
 */
class PhysicalRoundTripTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private lateinit var activity: MainActivity
    private lateinit var model: MeshModel
    private lateinit var run: String
    private lateinit var role: String
    private lateinit var plan: MeasurementPlan
    private var trace: PhysicalLatencyTrace? = null

    private fun main(action: () -> Unit) = instrumentation.runOnMainSync(action)
    private fun screen(): MeshScreenState {
        lateinit var value: MeshScreenState
        main { value = model.screen }
        return value
    }
    private fun log(event: String, vararg fields: Pair<String, Any>) {
        val value = JSONObject().put("run", run).put("role", role).put("event", event)
            .put("mono_ms", SystemClock.elapsedRealtime()).put("utc_ms", System.currentTimeMillis())
        for ((key, item) in fields) value.put(key, item)
        instrumentation.sendStatus(0, Bundle().apply { putString("stream", "MC025 $value\n") })
    }
    private fun await(timeout: Long, condition: (MeshScreenState) -> Boolean): Boolean {
        val deadline = SystemClock.elapsedRealtime() + timeout
        do {
            assertFalse("Device locked during physical measurement",
                activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
            val state = screen()
            assertFalse("Protected profile became unavailable", state.locked)
            if (condition(state)) return true
            SystemClock.sleep(100)
        } while (SystemClock.elapsedRealtime() < deadline)
        return false
    }
    private fun message(index: Int, echo: Boolean): String {
        val prefix = "MC25_${run}_${index}_${if (echo) "E" else "Q"}"
        return if (plan.long(index)) prefix.padEnd(280, if (echo) 'e' else 'q') else prefix
    }
    private fun received(state: MeshScreenState, text: String) =
        state.rows.any { !it.message.own && it.message.text == text }

    private fun counters(label: String) {
        val state = screen()
        val s = checkNotNull(state.contribution)
        log("counters", "label" to label, "elapsed_ms" to s.elapsedMs.toString(),
            "rx_frames" to s.receivedFrames.toString(), "rx_bytes" to s.receivedBytes.toString(),
            "rx_packets" to s.receivedPackets.toString(), "rx_chat" to s.receivedChatPackets.toString(),
            "scheduled_frames" to s.scheduledFrames.toString(), "completed_frames" to s.completedFrames.toString(),
            "completed_bytes" to s.completedBytes.toString(), "completed_objects" to s.completedObjects.toString(),
            "relayed_chat" to s.relayedChatCopies.toString(), "peers" to state.peers,
            "battery" to (s.batteryPercent?.toInt() ?: -1), "charging" to s.charging, "power" to s.powerMode)
    }

    @Test fun pairedAppRoundTrips() {
        val args = InstrumentationRegistry.getArguments()
        check(args.getString("physicalMeasure") == "true")
        run = args.getString("runId") ?: error("Missing unique runId")
        role = args.getString("role") ?: error("Missing A/B role")
        check(run.matches(Regex("[A-Za-z0-9]{1,16}")) && role in listOf("A", "B"))
        plan=MeasurementPlan(args.getString("mode") ?: "smoke",args.getString("initiator") ?: "A")
        log("plan","mode" to plan.mode,"initiator" to plan.initiator,"scheduled_pairs" to plan.pairs,
            "poll_ms" to 100,"traced" to (args.getString("latencyTrace")=="true"))
        activity = ui.activity
        model = (activity.application as MeshApplication).model
        assertFalse(activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        main { activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON); model.load() }
        try {
            ui.waitUntil(120000) { screen().onboarded && !screen().locked }
            main { model.select("#general"); model.startRadio() }
            assertTrue("No real peer within six minutes", await(360000) {
                it.selected?.name == "#general" && it.peers == 1 && it.contribution != null
            })
            log("connected")
            assertTrue("Initial bounded catch-up did not settle",await(180000) {
                it.catchup!=null && it.catchup!="Checking for recent messages…"
            })
            if(args.getString("latencyTrace")=="true") {
                trace=PhysicalLatencyTrace(model,run).also {it.attach()}
            }
            SystemClock.sleep(6000)
            counters("before")
            val completed = if (role == plan.initiator) initiate() else respond()
            SystemClock.sleep(5000)
            counters("after")
            log("summary", "completed_pairs" to completed, "scheduled_pairs" to plan.pairs)
            assertEquals("All predeclared pairs must be accounted for", plan.pairs, completed)
            main {model.stop()}
            assertTrue(await(30000){it.status=="Nearby connection is off"})
            trace?.storageBaseline()
        } finally {
            main {
                model.stop()
                activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
            // Let ActivityScenario own teardown. Launching/reordering an
            // activity here races its transition to DESTROYED.
            ui.waitUntil(30000) { screen().status == "Nearby connection is off" }
            trace?.flush { value ->
                value.put("run",run).put("role",role)
                instrumentation.sendStatus(0,Bundle().apply {putString("stream","MC025L $value\n")})
            }
        }
    }

    private fun initiate(): Int {
        var completed = 0
        repeat(plan.pairs) { index ->
            // The final request follows the responder's background transition.
            if (plan.background(index)) SystemClock.sleep(5000)
            val ready = await(30000) { it.peers == 1 && it.waitSeconds == 0 }
            val request = message(index, false)
            val start = SystemClock.elapsedRealtime()
            trace?.mark("request_start",index,false,start)
            log("request", "index" to index, "bytes" to request.length, "ready" to ready)
            if(ready)main { send(index,false,request) }
            val ok = ready && await(60000) { received(it, message(index, true)) }
            val elapsed = SystemClock.elapsedRealtime() - start
            if(ok)trace?.mark("observed",index,true,start+elapsed)
            log("roundtrip", "index" to index, "received" to ok, "elapsed_ms" to elapsed,
                "ready" to ready,"size" to (if(plan.long(index)) "long" else "short"),
                "own_accepted" to screen().rows.any { it.message.own && it.message.text == request })
            if (ok) completed++
        }
        return completed
    }

    private fun respond(): Int {
        val handled = mutableSetOf<Int>()
        val deadline = SystemClock.elapsedRealtime() + plan.pairs*95000L+60000L
        var completed=0
        while (handled.size < plan.pairs && SystemClock.elapsedRealtime() < deadline) {
            assertTrue("Device or profile became unavailable", await(1000) { !it.locked })
            val state = screen()
            for (index in 0 until plan.pairs) {
                if (index in handled || !received(state, message(index, false))) continue
                log("request_received", "index" to index)
                trace?.mark("observed",index,false)
                if (plan.background(index)) {
                    var background = false
                    main { background = !activity.lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED) }
                    log("background_receive", "background" to background)
                    assertTrue("Final request must arrive with responder activity stopped", background)
                }
                val ready=await(30000) { it.waitSeconds == 0 && it.peers == 1 }
                if(ready)main { send(index,true,message(index,true)) }
                val sent = ready && await(30000) { current -> current.rows.any {
                    it.message.own && it.message.text == message(index, true) &&
                        it.sendState == "Handed to mesh · delivery unknown"
                } }
                log("echo", "index" to index, "native_complete" to sent,"ready" to ready)
                if(sent)completed++
                handled.add(index)
                if (plan.mode=="smoke" && index == 5) {
                    var moved = false
                    main { moved = activity.moveTaskToBack(true) }
                    log("background", "moved" to moved)
                    assertTrue("Could not background responder", moved)
                }
            }
            SystemClock.sleep(100)
        }
        return completed
    }
    private fun send(index: Int, echo: Boolean, text: String) {
        val capture=trace
        if(capture==null)model.send(text) else capture.send(index,echo) {model.send(text)}
    }
}
