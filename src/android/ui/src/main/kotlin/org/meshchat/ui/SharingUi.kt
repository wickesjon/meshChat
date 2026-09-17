package org.meshchat.ui

import android.content.ClipboardManager
import android.content.Intent
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import uniffi.meshchat_core.ChannelInfo

@Composable
internal fun ShareQr(uri: String, description: String) {
    val bitmap=remember(uri) {Sharing.qr(uri)}
    Image(bitmap.asImageBitmap(),description,Modifier.fillMaxWidth().aspectRatio(1f).background(Color.White).padding(8.dp))
}

@Composable
internal fun ShareActions(uri: String) {
    val context=LocalContext.current
    var error by remember(uri) {mutableStateOf<String?>(null)}
    Text("Prefer QR or the share sheet over copying.")
    OutlinedButton(onClick={
        try {context.startActivity(Intent.createChooser(Sharing.qrIntent(context,uri),"Share offline QR"));error=null}
        catch (_: Exception) {error="QR export is unavailable. Show this code on your screen instead."}
    }) {Text("Share as QR")}
    Button(onClick={
        try {context.startActivity(Intent.createChooser(Sharing.linkIntent(uri),"Share link"));error=null}
        catch (_: Exception) {error="Sharing is unavailable. Show the QR instead."}
    }) {Text("Share link")}
    Text(Sharing.HTTPS_NOTICE)
    TextButton(onClick={context.getSystemService(ClipboardManager::class.java).setPrimaryClip(Sharing.clip(uri))}) {Text("Copy link")}
    Text("Copy leaves the link in the system clipboard. Other apps or devices may be able to read it.")
    error?.let {Text(it,color=MaterialTheme.colorScheme.error)}
}

@Composable
internal fun ChannelShare(channel: ChannelInfo, close: ()->Unit) {
    val uri=remember(channel.name) {Sharing.channelUri(channel.name)}
    AlertDialog(onDismissRequest=close,title={Text("Share channel")},text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(12.dp)) {
        MeshIcon("open-lock");Text("Anyone with these words can read. This channel is not encrypted.")
        ShareQr(uri,"Offline channel QR")
        Text("Scanning works fully offline in the app.")
        channel.name.split('|').forEach {Text(it,style=MaterialTheme.typography.headlineMedium,fontWeight=FontWeight.Bold)}
        Text("Say them out loud — same channel.")
        ShareActions(uri)
    }},confirmButton={TextButton(onClick=close){Text("Done")}})
}

@Composable
internal fun ChannelConfirmation(model: MeshModel, s: MeshScreenState) {
    s.channelProposal?.let {channel -> AlertDialog(onDismissRequest=model::cancelChannelProposal,
        title={Text("Join this channel?")},text={Column(verticalArrangement=Arrangement.spacedBy(12.dp)) {
            Text(channel.name.replace('|',' '),fontWeight=FontWeight.Bold)
            Text("Anyone with these words can read this channel. It is not encrypted. Nothing is joined until you confirm.")
        }},confirmButton={Button(onClick={model.confirmChannel(channel.name)}){Text("Join channel")}},
        dismissButton={TextButton(onClick=model::cancelChannelProposal){Text("Cancel")}}) }
}

@Composable
internal fun ShareInput(model: MeshModel, close: ()->Unit) {
    var uri by remember {mutableStateOf("")}
    AlertDialog(onDismissRequest=close,title={Text("Paste a channel or friend link")},text={Column {
        Text("Review channel words or compare a friend's fingerprint before confirming.")
        OutlinedTextField(uri,{if(it.length<=2048)uri=it},label={Text("Shared link")},maxLines=5)
    }},confirmButton={Button(onClick={model.shareInput(uri);close()},enabled=uri.isNotEmpty()){Text("Review link")}},
        dismissButton={TextButton(onClick=close){Text("Cancel")}})
}
