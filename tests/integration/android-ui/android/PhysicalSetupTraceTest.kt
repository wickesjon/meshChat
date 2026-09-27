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
import org.meshchat.transport.GattDriver
import org.meshchat.transport.GattRadio
import uniffi.meshchat_core.SendPath
import uniffi.meshchat_core.TransportEvent
import uniffi.meshchat_core.NativeTransport
import uniffi.meshchat_core.LinkHandle
import uniffi.meshchat_core.TransportEffects
import uniffi.meshchat_core.TransportException
import uniffi.meshchat_core.TransportRole
import uniffi.meshchat_core.TransportConnection

/** Opt-in diagnostic capture, not a connectivity acceptance test. Every radio
 * call still reaches the real adapter; no packet/address/key content is logged. */
class PhysicalSetupTraceTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private fun field(owner: Any, name: String) = owner.javaClass.getDeclaredField(name).apply { isAccessible=true }

    @Suppress("UNCHECKED_CAST")
    @Test fun captureSetupClosePath() {
        val instrumentation=InstrumentationRegistry.getInstrumentation()
        val args=InstrumentationRegistry.getArguments()
        check(args.getString("physicalSetupTrace")=="true")
        val run=checkNotNull(args.getString("runId"))
        val role=checkNotNull(args.getString("role"))
        check(run.matches(Regex("[A-Za-z0-9]{1,16}")) && role in listOf("A","B"))
        val activity=ui.activity
        val model=(activity.application as MeshApplication).model
        val gate=checkNotNull(field(model,"coreGate").get(model))
        val entries=ArrayList<JSONObject>()
        var dropped=0
        fun record(event: String, vararg values: Pair<String,Any>) {
            if(entries.size>=1024) { dropped++; return }
            entries.add(JSONObject().put("event",event).put("mono_ms",SystemClock.elapsedRealtime()).apply {
                for((key,value) in values)put(key,value)
            })
        }
        fun stack()=Throwable().stackTrace.filter {it.className.startsWith("org.meshchat.transport.")}
            .take(12).joinToString(" > ") {"${it.className.substringAfterLast('.')}.${it.methodName}:${it.lineNumber}"}
        var restore: (() -> Unit)?=null
        assertFalse(activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        ui.runOnUiThread { activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON); model.load() }
        try {
            ui.waitUntil(120000) {model.screen.onboarded && !model.screen.locked}
            ui.runOnUiThread {model.select("#general");model.startRadio()}
            ui.waitUntil(30000) {synchronized(gate) {field(model,"radio").get(model)!=null}}
            synchronized(gate) {
                val radio=checkNotNull(field(model,"radio").get(model))
                val driver=field(radio,"driver").get(radio) as GattDriver
                assertTrue("Trace must attach before connections",driver.connections().isEmpty())
                val portField=field(driver,"radio")
                val original=portField.get(driver) as GattRadio
                val eventField=field(driver,"event")
                val originalEvent=eventField.get(driver) as (Long,TransportEvent)->Unit
                val coreField=field(driver,"core")
                val core=coreField.get(driver) as NativeTransport
                // An owned clone of the same Rust Arc, not a replacement core.
                // Delegate through the real FFI and preserve every exception.
                val observedCore=object:NativeTransport(core.uniffiClonePointer()) {
                    override fun nativeReady(admission:ULong,role:TransportRole,transmitBytes:UShort,receiveBytes:UShort,now:ULong):TransportConnection =
                        super.nativeReady(admission,role,transmitBytes,receiveBytes,now).also {
                            record("native_created","generation" to it.link.generation.toString(),"transport_role" to role.name)
                        }
                    override fun receive(link:LinkHandle,reportedBytes:ULong,bytes:ByteArray,now:ULong):TransportEffects {
                        try {return super.receive(link,reportedBytes,bytes,now)}
                        catch(e:TransportException) {
                            record("receive_exception","exception_type" to e.javaClass.simpleName)
                            throw e
                        }
                    }
                }
                restore={portField.set(driver,original);eventField.set(driver,originalEvent);coreField.set(driver,core);observedCore.close()}
                coreField.set(driver,observedCore)
                fun call(name: String,id: Long,action: ()->Boolean):Boolean {
                    try {
                        return action().also {record(name,"id" to id,"accepted" to it)}
                    } catch(e: Exception) {
                        record(name,"id" to id,"exception_type" to e.javaClass.simpleName)
                        throw e
                    }
                }
                portField.set(driver,object:GattRadio {
                    override fun connect(id:Long,address:String)=call("connect",id) {original.connect(id,address)}
                    override fun discover(id:Long)=call("discover",id) {original.discover(id)}
                    override fun requestMtu(id:Long)=call("request_mtu",id) {original.requestMtu(id)}
                    override fun subscribe(id:Long)=call("subscribe",id) {original.subscribe(id)}
                    override fun send(id:Long,path:SendPath,bytes:ByteArray)=call("send_${path.name}",id) {original.send(id,path,bytes)}
                    override fun close(id:Long) {
                        record("radio_close","id" to id,"stack" to stack())
                        original.close(id)
                    }
                })
                eventField.set(driver,{id:Long,event:TransportEvent ->
                    if(event is TransportEvent.Closed)record("native_closed","id" to id,"stack" to stack())
                    if(event is TransportEvent.Admitted)record("native_admitted","id" to id,"generation" to event.link.generation.toString())
                    originalEvent(id,event)
                })
                record("attached")
            }
            val snapshots=PhysicalConnectionTrace(model)
            repeat(40) {
                assertFalse(activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
                assertFalse(model.screen.locked)
                synchronized(gate) {
                    val value=snapshots.snapshot()
                    if(entries.size<1024)entries.add(value) else dropped++
                }
                SystemClock.sleep(1000)
            }
        } finally {
            synchronized(gate) {record("cleanup_begin")}
            try {
                ui.runOnUiThread {model.stop();activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}
                ui.waitUntil(30000) {model.screen.status=="Nearby connection is off"}
            } finally { synchronized(gate) {
                restore?.invoke()
                for(value in entries) {
                    value.put("run",run).put("role",role)
                    instrumentation.sendStatus(0,Bundle().apply {putString("stream","MC025S $value\n")})
                }
                instrumentation.sendStatus(0,Bundle().apply {putString("stream","MC025S "+JSONObject()
                    .put("run",run).put("role",role).put("event","trace_summary")
                    .put("entries",entries.size).put("dropped",dropped)+"\n")})
            } }
        }
        assertEquals("Trace overflow makes diagnosis incomplete",0,dropped)
    }
}
