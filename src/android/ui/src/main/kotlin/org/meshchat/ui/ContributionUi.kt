package org.meshchat.ui

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Typeface
import android.text.Layout
import android.text.StaticLayout
import android.text.TextPaint
import androidx.core.content.FileProvider
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import uniffi.meshchat_core.*
import java.io.File

/** Only the fixed numeric allowlist in contributionShareText reaches this image. */
internal object ContributionExport {
    fun image(stats:TransportStats,received:Boolean,sent:Boolean,relayed:Boolean):Bitmap {
        require(received||sent||relayed)
        val text=contributionShareText(stats,received,sent,relayed)
        val paint=TextPaint().apply {color=Color.rgb(24,35,32);textSize=40f;typeface=Typeface.create("sans-serif",Typeface.NORMAL);isAntiAlias=true}
        val layout=StaticLayout.Builder.obtain(text,0,text.length,paint,960).setAlignment(Layout.Alignment.ALIGN_NORMAL).setLineSpacing(12f,1f).build()
        return Bitmap.createBitmap(1080,layout.height+120,Bitmap.Config.ARGB_8888).also {bitmap->
            val canvas=Canvas(bitmap);canvas.drawColor(Color.rgb(246,250,247));canvas.translate(60f,60f);layout.draw(canvas)
        }
    }
    fun intent(context:Context,stats:TransportStats,received:Boolean,sent:Boolean,relayed:Boolean):Intent {
        val directory=File(context.cacheDir,"share-stats").apply {check(mkdirs()||isDirectory)}
        directory.listFiles()?.filter {System.currentTimeMillis()-it.lastModified()>24*60*60*1000L}?.forEach {it.delete()}
        check((directory.listFiles()?.size ?: 0)<8) {"Contribution export cache is full"}
        val bitmap=image(stats,received,sent,relayed)
        val file=File.createTempFile("contribution-",".png",directory)
        try {file.outputStream().use {check(bitmap.compress(Bitmap.CompressFormat.PNG,100,it))}}
        catch(failure:Exception){file.delete();throw failure}finally{bitmap.recycle()}
        val uri=FileProvider.getUriForFile(context,context.packageName+".share",file)
        return Intent(Intent.ACTION_SEND).setType("image/png").putExtra(Intent.EXTRA_STREAM,uri).apply {
            clipData=ClipData.newUri(context.contentResolver,"Local contribution",uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
    }
}

@Composable
internal fun ContributionPanel(model:MeshModel,state:MeshScreenState,close:()->Unit) {
    DisposableEffect(model) {model.contributionVisible(true);onDispose {model.contributionVisible(false)}}
    var reset by remember {mutableStateOf(false)}
    var export by remember {mutableStateOf<TransportStats?>(null)}
    val stats=state.contribution
    AlertDialog(onDismissRequest=close,title={Text("Contribution & power")},text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(10.dp)) {
        Text("Private on this device. No telemetry or leaderboard.")
        Text("Counters cover this app session; reset, app restart or protected identity reopening starts a new window. Stopping the radio keeps the counters.")
        if(stats!=null) {
            Text("Window: ${stats.elapsedMs/60000u} minutes")
            Text("Received frames: ${stats.receivedFrames} · includes rejected input")
            Text("Reassembled packets: ${stats.receivedPackets} · includes repeated copies")
            Text("Received chat packets: ${stats.receivedChatPackets}")
            Text("Frames offered to radio: ${stats.scheduledFrames} · may be refused")
            Text("Native frame completions: ${stats.completedFrames}")
            Text("Completed outgoing objects: ${stats.completedObjects}")
            Text("Live chat relay copies: ${stats.relayedChatCopies}")
            Text("Completions and relay copies count each outgoing connection. They do not establish delivery, unique people, reach or bridge coverage.")
            Text("Power preference: ${state.power.name.lowercase().replaceFirstChar {it.uppercase()}}")
            if(stats.batteryPercent!=null) {
                Text("Device battery: ${stats.batteryPercent}% · ${if(stats.charging)"charging" else "unplugged"}")
                Text("Active policy: ${stats.powerMode}")
            } else Text("Device battery reading unavailable or older than one minute.")
            Text("App battery use unavailable. No reliable app-specific measurement or calibrated estimate is available; device charge is not app consumption.")
            OutlinedButton(onClick={export=stats}) {Text("Preview contribution card")}
            TextButton(onClick={reset=true}) {Text("Reset local counters")}
        }
    }},confirmButton={TextButton(onClick=close){Text("Done")}})
    if(reset)AlertDialog(onDismissRequest={reset=false},title={Text("Reset these counters?")},text={Text("This clears only local contribution counts. Connections, traffic limits, history and trust are unchanged.")},confirmButton={TextButton(onClick={model.resetContribution();reset=false}){Text("Reset counters")}},dismissButton={TextButton(onClick={reset=false}){Text("Cancel")}})
    export?.let {ContributionPreview(it) {export=null}}
}

@Composable
private fun ContributionPreview(stats:TransportStats,close:()->Unit) {
    val context=LocalContext.current
    var received by remember {mutableStateOf(false)};var sent by remember {mutableStateOf(false)};var relayed by remember {mutableStateOf(false)}
    var error by remember {mutableStateOf<String?>(null)}
    AlertDialog(onDismissRequest=close,title={Text("Choose what to share")},text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(8.dp)) {
        Text("Frozen preview. Only selected local aggregates are included in the image.")
        Row {Checkbox(received,{received=it},Modifier.semantics {contentDescription="Share received activity"});Text("Received activity")}
        Row {Checkbox(sent,{sent=it},Modifier.semantics {contentDescription="Share native handoffs"});Text("Native handoffs")}
        Row {Checkbox(relayed,{relayed=it},Modifier.semantics {contentDescription="Share live chat relays"});Text("Live chat relays")}
        if(received||sent||relayed)Text(contributionShareText(stats,received,sent,relayed))
        error?.let {Text(it,color=MaterialTheme.colorScheme.error)}
    }},confirmButton={TextButton(enabled=received||sent||relayed,onClick={
        try {context.startActivity(Intent.createChooser(ContributionExport.intent(context,stats,received,sent,relayed),"Share contribution image"));close()}
        catch(_:Exception){error="Unable to export this card. Try again after old temporary exports expire."}
    }) {Text("Share image")}},dismissButton={TextButton(onClick=close){Text("Cancel")}})
}
