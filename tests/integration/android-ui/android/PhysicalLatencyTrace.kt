package org.meshchat.ui

import android.os.SystemClock
import org.json.JSONObject
import org.meshchat.storage.EncryptedStorage
import org.meshchat.transport.AndroidGattRadio
import org.meshchat.transport.GattDriver
import uniffi.meshchat_core.*
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/** Test-only observations. No packet/body/key output, native admission changes,
 * retained storage handles, replacement radio ports or synchronous trace I/O.
 */
internal class PhysicalLatencyTrace(private val model: MeshModel, private val run: String) {
    private data class Sample(val index: Int, val echo: Boolean)
    private data class Entry(val event: String, val at: Long, val values: Map<String, Any>)
    private val entries=ArrayList<Entry>()
    private var dropped=0
    private val frameSamples=linkedMapOf<String,Sample>()
    private val pattern=Regex("MC25_${Regex.escape(run)}_([0-9]{1,2})_([QE])")
    private fun field(name: String)=MeshModel::class.java.getDeclaredField(name).apply {isAccessible=true}
    private val queue=field("queue").get(model) as ThreadPoolExecutor
    private val gate=checkNotNull(field("coreGate").get(model))
    private fun now()=SystemClock.elapsedRealtime()
    private fun record(event: String, at: Long=now(), vararg values: Pair<String,Any>) {
        synchronized(entries) {
            if(entries.size<4096)entries.add(Entry(event,at,values.toMap())) else dropped++
        }
    }
    private fun sample(raw: ByteArray): Sample? {
        val found=pattern.find(raw.toString(Charsets.UTF_8)) ?: return null
        return Sample(found.groupValues[1].toInt(),found.groupValues[2]=="E")
    }
    private fun note(event: String, sample: Sample, at: Long=now()) =
        record(event,at,"index" to sample.index,"echo" to sample.echo)
    fun mark(event: String, index: Int, echo: Boolean, at: Long=now()) = note(event,Sample(index,echo),at)
    fun send(index: Int, echo: Boolean, action: ()->Unit) {
        val sample=Sample(index,echo)
        note("send_call",sample)
        queue.execute {note("send_queue_before",sample)}
        action()
        queue.execute {note("send_queue_after",sample)}
    }
    private fun <T> serial(action: ()->T): T = queue.submit<T> {
        synchronized(gate) {action()}
    }.get(60,TimeUnit.SECONDS)

    @Suppress("UNCHECKED_CAST")
    fun attach() = serial {
        val radio=field("radio").get(model) as AndroidGattRadio
        val driverField=AndroidGattRadio::class.java.getDeclaredField("driver").apply {isAccessible=true}
        val driver=driverField.get(radio) as GattDriver
        val eventField=GattDriver::class.java.getDeclaredField("event").apply {isAccessible=true}
        val original=eventField.get(driver) as (Long,TransportEvent)->Unit
        eventField.set(driver,{id:Long,event:TransportEvent ->
            val received=if(event is TransportEvent.Received && event.intake!=TransportIntake.DEFERRED_SYNC)
                sample(event.bytes) else null
            if(received!=null) {
                note("native_receive",received)
                queue.execute {note("receive_queue_before",received)}
            }
            if(event is TransportEvent.Closed)record("link_closed")
            original(id,event)
            if(received!=null)queue.execute {note("receive_queue_after",received)}
        })
        val egress=checkNotNull(driver.protectedEgress)
        driver.protectedEgress={send,submit ->
            val entered=now()
            val bytes=send.bytes
            var index=0
            var count=1
            val measured=when(bytes.firstOrNull()?.toInt()) {
                0 -> sample(bytes)
                1 -> {
                    index=bytes[14].toInt() and 255;count=bytes[15].toInt() and 255
                    val key=bytes.copyOfRange(4,14).joinToString("") {"%02x".format(it.toInt() and 255)}
                    val found=if(index==0)sample(bytes.copyOfRange(18,bytes.size)) else frameSamples[key]
                    if(found!=null) {
                        if(frameSamples.size>=64)frameSamples.remove(frameSamples.keys.first())
                        frameSamples[key]=found
                    }
                    if(index==count-1)frameSamples.remove(key)
                    found
                }
                else -> null
            }
            var submitAt=-1L
            var submitEnd=-1L
            var nativeAccepted=false
            var allowed=false
            try {
                allowed=egress(send) {
                    submitAt=now()
                    try {submit().also {nativeAccepted=it}} finally {submitEnd=now()}
                }
                allowed
            } finally {
                record("egress",entered,"index" to (measured?.index ?: -1),
                    "echo" to (measured?.echo ?: false),"fragment" to index,"fragments" to count,
                    "bytes" to bytes.size,"submit_ms" to submitAt,"submit_end_ms" to submitEnd,
                    "end_ms" to now(),"native_accepted" to nativeAccepted,"allowed" to allowed)
            }
        }
        record("attached")
    }

    /** Ordinary protected reads after the radio is stopped. No handles escape. */
    fun storageBaseline() = serial {
        check(field("radio").get(model)==null)
        val storage=field("storage").get(model) as EncryptedStorage
        val core=field("transport").get(model) as NativeTransport
        repeat(5) { index ->
            val start=now()
            check(storage.getRecord(RecordKind.SETTING,"ui-profile-v1".toByteArray())!=null)
            record("protected_read",start,"index" to index,"duration_ms" to (now()-start))
            val nativeStart=now()
            core.beaconStatus(nativeStart.toULong())
            record("native_status",nativeStart,"index" to index,"duration_ms" to (now()-nativeStart))
        }
    }
    fun flush(write: (JSONObject)->Unit) {
        val snapshot=synchronized(entries) {entries.toList()}
        for(entry in snapshot) {
            val json=JSONObject().put("event",entry.event).put("mono_ms",entry.at)
            for((name,value) in entry.values)json.put(name,value)
            write(json)
        }
        write(JSONObject().put("event","trace_summary").put("entries",snapshot.size).put("dropped",dropped))
        check(dropped==0) {"Latency trace overflow; attribution unavailable"}
    }
}
