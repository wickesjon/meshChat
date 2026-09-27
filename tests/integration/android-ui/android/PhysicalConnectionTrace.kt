package org.meshchat.ui

import android.os.SystemClock
import org.json.JSONArray
import org.json.JSONObject
import org.meshchat.transport.GattDriver

/** Opt-in setup diagnostics. No addresses, identities, payloads or key material.
 * Read bounded adapter state under its existing monitor; never change policy. */
internal class PhysicalConnectionTrace(private val model: MeshModel) {
    private fun field(owner: Any, name: String): Any? = owner.javaClass.getDeclaredField(name)
        .apply { isAccessible=true }.get(owner)

    fun snapshot(): JSONObject = synchronized(checkNotNull(field(model,"coreGate"))) {
        val now=SystemClock.elapsedRealtime()
        val state=model.screen
        val result=JSONObject().put("event","connection_state").put("mono_ms",now)
            .put("status",state.status).put("peers",state.peers).put("locked",state.locked)
            .put("loading",state.loading).put("onboarded",state.onboarded)
            .put("starting",field(model,"starting"))
        val radio=field(model,"radio") ?: return@synchronized result.put("radio",false)
        result.put("radio",true)
        for(name in listOf("running","state","scanMode","serverEpoch","refusedRetryAt"))
            result.put(name,field(radio,name).toString())
        for(name in listOf("server","scanner","advertiser"))result.put(name,field(radio,name)!=null)
        result.put("retired",(field(radio,"refusedAddresses") as Set<*>).size)
        val scan=checkNotNull(field(radio,"scanning"))
        result.put("scan_failures",field(scan,"failures")).put("scan_retry_at",field(scan,"retryAt"))
        val selection=checkNotNull(field(radio,"selection"))
        val seen=(field(selection,"seen") as Map<*,*>).values.filterNotNull()
        result.put("seen",seen.size)
            .put("blocked",seen.count {(field(it,"blockedUntil") as Long)>now})
            .put("retry_wait",seen.count {(field(it,"retryAt") as Long)>now})
        val driver=field(radio,"driver") as GattDriver
        val peers=(field(driver,"peers") as Map<*,*>).values.filterNotNull()
        result.put("native_peers",JSONArray().apply {for(peer in peers)put(JSONObject()
            .put("id",field(peer,"id")).put("role",field(peer,"role").toString())
            .put("age_ms",now-(field(peer,"born") as Long))
            .put("capacity",field(peer,"capacity")).put("subscribed",field(peer,"subscribed"))
            .put("native_ready",field(peer,"link")!=null))})
        result
    }
}
