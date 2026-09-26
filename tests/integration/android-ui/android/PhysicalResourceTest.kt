package org.meshchat.ui

import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import android.os.Debug
import android.os.Process
import android.os.SystemClock
import android.os.PowerManager
import java.util.concurrent.ThreadPoolExecutor
import java.security.MessageDigest
import java.io.File
import androidx.test.platform.app.InstrumentationRegistry
import org.json.JSONObject
import org.junit.Test

/** Opt-in passive observer of a manually configured workload. Never changes
 * radio/power settings, sends messages, wakes the screen or resets counters.
 * Start the app/workload normally while the observer waits for its ready state. */
class PhysicalResourceTest {
    @Test fun observeConfiguredWorkload() {
        val instrumentation=InstrumentationRegistry.getInstrumentation()
        val args=InstrumentationRegistry.getArguments()
        check(args.getString("physicalResources")=="true")
        val run=checkNotNull(args.getString("runId"))
        check(run.matches(Regex("[A-Za-z0-9]{1,16}")))
        val workload=checkNotNull(args.getString("workload"))
        check(workload in listOf("disconnected","connected_idle","messaging","beacon"))
        val seconds=checkNotNull(args.getString("durationSeconds")).toLong()
        check(seconds in 60..21600)
        val context=instrumentation.targetContext
        val model=(context.applicationContext as MeshApplication).model
        val queue=MeshModel::class.java.getDeclaredField("queue").apply {isAccessible=true}.get(model) as ThreadPoolExecutor
        val battery=context.getSystemService(BatteryManager::class.java)
        val power=context.getSystemService(PowerManager::class.java)
        fun screen():MeshScreenState {
            lateinit var value:MeshScreenState
            instrumentation.runOnMainSync {value=model.screen}
            return value
        }
        fun valid(s:MeshScreenState):Boolean = s.onboarded && !s.locked && when(workload) {
            "disconnected" -> s.status=="Nearby connection is off" && !s.beaconRequested && !s.autoBeacon
            "beacon" -> s.beacon?.active==true
            else -> s.peers==1 && s.beacon?.active!=true
        }
        fun log(row:JSONObject) {
            row.put("run",run).put("workload",workload)
            instrumentation.sendStatus(0,android.os.Bundle().apply {putString("stream","MC025R $row\n")})
        }
        val digest=MessageDigest.getInstance("SHA-256")
        File(context.applicationInfo.sourceDir).inputStream().use {input ->
            val buffer=ByteArray(8192)
            while(true) {val n=input.read(buffer);if(n<0)break;digest.update(buffer,0,n)}
        }
        log(JSONObject().put("event","plan").put("duration_seconds",seconds).put("sample_seconds",60)
            .put("model",android.os.Build.MODEL).put("sdk",android.os.Build.VERSION.SDK_INT)
            .put("apk_sha256",digest.digest().joinToString("") {"%02x".format(it)}))
        val readyUntil=SystemClock.elapsedRealtime()+180000
        while(!valid(screen()) && SystemClock.elapsedRealtime()<readyUntil)SystemClock.sleep(1000)
        check(valid(screen())) {"Configure the declared workload in the normal app; observer never starts it"}
        val start=SystemClock.elapsedRealtime()
        var next=start
        var index=0
        while(true) {
            val now=SystemClock.elapsedRealtime()
            val s=screen()
            val info=Debug.MemoryInfo().also {Debug.getMemoryInfo(it)}
            val intent=context.registerReceiver(null,IntentFilter(Intent.ACTION_BATTERY_CHANGED))
            val charge=battery.getIntProperty(BatteryManager.BATTERY_PROPERTY_CHARGE_COUNTER)
            val percent=battery.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY)
            val stats=s.contribution
            log(JSONObject().put("event","sample").put("index",index++)
                .put("elapsed_ms",now-start).put("cpu_ms",Process.getElapsedCpuTime())
                .put("completed_tasks",queue.completedTaskCount).put("queued_tasks",queue.queue.size)
                .put("pss_kib",info.totalPss).put("battery_percent",if(percent in 0..100)percent else JSONObject.NULL)
                .put("charge_uah",if(charge!=Int.MIN_VALUE && charge>0)charge else JSONObject.NULL)
                .put("plugged",intent?.getIntExtra(BatteryManager.EXTRA_PLUGGED,-1) ?: -1)
                .put("temperature_tenths_c",intent?.getIntExtra(BatteryManager.EXTRA_TEMPERATURE,-1) ?: -1)
                .put("screen_interactive",power.isInteractive).put("workload_valid",valid(s))
                .put("peers",s.peers).put("scheduled_frames",stats?.scheduledFrames?.toString() ?: JSONObject.NULL)
                .put("received_frames",stats?.receivedFrames?.toString() ?: JSONObject.NULL)
                .put("stats_elapsed_ms",stats?.elapsedMs?.toString() ?: JSONObject.NULL))
            if(now-start>=seconds*1000)break
            next=minOf(next+60000,start+seconds*1000)
            while(SystemClock.elapsedRealtime()<next)SystemClock.sleep(minOf(1000,next-SystemClock.elapsedRealtime()).coerceAtLeast(1))
        }
        log(JSONObject().put("event","summary").put("samples",index).put("elapsed_ms",SystemClock.elapsedRealtime()-start))
    }
}
