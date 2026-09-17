package org.meshchat.ui

import android.app.Activity
import android.content.ContextWrapper
import android.view.WindowManager
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.window.DialogWindowProvider
import android.view.Window
import android.view.MotionEvent
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.window.SecureFlagPolicy
import java.time.Instant
import uniffi.meshchat_core.*

private fun date(seconds:UInt)=Instant.ofEpochSecond(seconds.toLong()).toString()
@Composable
internal fun OrganizerConfirmation(model:MeshModel,s:MeshScreenState) {
    val context=LocalContext.current
    val activity=generateSequence(context) {(it as? ContextWrapper)?.baseContext}.filterIsInstance<Activity>().firstOrNull()
    val private=s.staffProposal!=null
    DisposableEffect(private) {
        val wasSecure=activity?.window?.attributes?.flags?.and(WindowManager.LayoutParams.FLAG_SECURE)!=0
        if(private)activity?.window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        onDispose {if(private&&!wasSecure)activity?.window?.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)}
    }
    s.eventProposal?.let { event -> AlertDialog(onDismissRequest=model::cancelOrganizer,title={Text("Adopt this event?")},
        text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(12.dp)) {
            Text(event.name,fontWeight=FontWeight.Bold);Text("Only confirm a code from an official source. Discovery does not establish trust.")
            Text("Root fingerprint: ${event.fingerprint}");Text("Expires ${date(event.expiry)}")
            if(s.events.any {it.event.key.contentEquals(event.key)})Text("This updates an existing adoption. Compare its fingerprint and expiry before confirming.")
        }},confirmButton={Button(onClick=model::adoptEvent){Text("Adopt event")}},dismissButton={TextButton(onClick=model::cancelOrganizer){Text("Cancel")}}) }
    s.staffProposal?.let { staff -> AlertDialog(onDismissRequest=model::cancelOrganizer,title={Text("Import staff credential?")},
        properties=DialogProperties(securePolicy=SecureFlagPolicy.SecureOn),
        text={ProvisioningTouchGuard();Column(verticalArrangement=Arrangement.spacedBy(12.dp)) {
            Text(staff.label,fontWeight=FontWeight.Bold);Text("The matching official event must already be adopted. Importing a staff code never adopts a root.")
            Text("Event ID: ${staff.rootId.joinToString(""){"%02x".format(it.toInt() and 255)}}")
            Text("Valid ${date(staff.notBefore)} to ${date(staff.notAfter)}")
            if(s.staffPresent)Text("This replaces your current staff key only after validation succeeds.")
            Text("The private code is discarded on cancel, background or after one minute. Do not share or photograph it.")
        }},confirmButton={Button(onClick=model::confirmStaff){Text("Import staff key")}},dismissButton={TextButton(onClick=model::cancelOrganizer){Text("Cancel")}}) }
}
@Composable
internal fun ColumnScope.EventChat(model:MeshModel,s:MeshScreenState,scan:()->Unit,scanStaff:()->Unit) {
    var manage by remember {mutableStateOf(false)}
    var expanded by remember {mutableStateOf(false)}
    var pin by remember {mutableStateOf(false)}
    val adopted=s.events.any {it.active}
    Column(Modifier.fillMaxWidth().padding(horizontal=16.dp),verticalArrangement=Arrangement.spacedBy(6.dp)) {
        Text(if(adopted)"Official codes adopted · readable on air" else "No current official code adopted. Event updates can be spoofed.")
        if(s.discoveries.isNotEmpty())Text("Nearby event: ${s.discoveries.first()}. Scan an official QR to enable verified updates.")
        Row {TextButton(onClick=scan){Text("Scan official QR")};TextButton(onClick={manage=true}){Text("Event & staff keys")}}
        s.staffStatus?.let {Text(it,color=MaterialTheme.colorScheme.error)}
    }
    val verified=s.eventRows.filter {it.staffLabel!=null}
    val unverified=s.eventRows.filter {it.staffLabel==null}
    LazyColumn(Modifier.weight(1f).fillMaxWidth(),contentPadding=PaddingValues(16.dp),verticalArrangement=Arrangement.spacedBy(14.dp)) {
        items(verified.sortedByDescending {it.pinned},key={it.id.joinToString()+"verified"}) {row->EventRow(row)}
        if(adopted&&unverified.isNotEmpty())item {TextButton(onClick={expanded=!expanded}) {Text("${if(expanded)"Hide" else "Show"} unverified messages (${unverified.size})")};Text("Unverified or awaiting a credential. No staff or pin authority.")}
        if(!adopted||expanded)items(unverified,key={it.id.joinToString()+"unverified"}) {row->EventRow(row)}
        if(s.eventRows.isEmpty())item {Text("No event updates yet.")}
    }
    if(s.staffReady) {
        Row(Modifier.padding(horizontal=16.dp)) {Checkbox(pin,{pin=it});Text("Pin for up to 1 hour (within credential expiry)")}
        ChannelComposer(false,0,s.posted) {value->model.sendEvent(value,if(pin)minOf(checkNotNull(s.staff).notAfter,(System.currentTimeMillis()/1000+3600).toUInt())else null)}
    } else if(adopted)Text("Only event staff can post here",Modifier.fillMaxWidth().padding(24.dp))
    else ChannelComposer(false,s.waitSeconds,s.posted,model::send)
    if(manage)AlertDialog(onDismissRequest={manage=false},title={Text("Event & staff keys")},text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(12.dp)) {
        for(card in s.events) {Text(card.event.name,fontWeight=FontWeight.Bold);Text(if(card.active)"Expires ${date(card.event.expiry)}" else "Expired · no current authority");Text(card.event.fingerprint);TextButton(onClick={model.removeEvent(card.event.key)}){Text("Remove ${card.event.name}")}}
        s.staff?.let {Text("Staff: ${it.label} · expires ${date(it.notAfter)}")}
        Text("Private staff keys are device-protected. Expired or lost credentials require explicit reprovisioning; expiry is never extended.")
        Button(onClick={manage=false;scanStaff()}) {Text("Scan private staff QR")}
        if(s.staffPresent)TextButton(onClick=model::forgetStaff) {Text("Forget staff key")}
    }},confirmButton={TextButton(onClick={manage=false}){Text("Done")}})
}
@Composable private fun EventRow(row:EventMessage) {
    Column(verticalArrangement=Arrangement.spacedBy(5.dp)) {
        Text(row.nickname,fontWeight=FontWeight.Bold)
        if(row.staffLabel!=null)Surface(color=MaterialTheme.colorScheme.primaryContainer) {Text("✓ Event Staff · ${row.staffLabel}",Modifier.padding(6.dp))}
        else Text("Unverified",color=MaterialTheme.colorScheme.onSurfaceVariant)
        if(row.pinned)Text("Pinned update",fontWeight=FontWeight.Bold)
        Text(row.text)
    }
}

/** Dialogs have their own Window: block both full and partial obscured touches
 * there as well as on the hosting Activity and camera screens. */
@Composable private fun ProvisioningTouchGuard() {
    val view=LocalView.current
    DisposableEffect(view) {
        val window=(view.parent as? DialogWindowProvider)?.window
        val original=window?.callback
        val guard=original?.let {callback->object:Window.Callback by callback {
            override fun dispatchTouchEvent(event:MotionEvent):Boolean {
                if(event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)!=0)return true
                return callback.dispatchTouchEvent(event)
            }
        }}
        if(guard!=null)window.callback=guard
        onDispose {if(window?.callback===guard && original!=null)window.callback=original}
    }
}
