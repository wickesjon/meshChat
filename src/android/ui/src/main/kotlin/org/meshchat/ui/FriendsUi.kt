package org.meshchat.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.google.zxing.BarcodeFormat
import com.journeyapps.barcodescanner.BarcodeEncoder
import uniffi.meshchat_core.*

@Composable
internal fun FriendsPage(model: MeshModel, s: MeshScreenState, scan: () -> Unit, messages: Boolean) {
    var code by remember { mutableStateOf(false) }
    var input by remember { mutableStateOf(false) }
    var change by remember { mutableStateOf<FriendCard?>(null) }
    LazyColumn(Modifier.fillMaxSize(), contentPadding=PaddingValues(20.dp),verticalArrangement=Arrangement.spacedBy(16.dp)) {
        item {
            Text(if(messages) "Your private conversations" else "People you know",fontSize=26.sp,fontWeight=FontWeight.Bold)
            Text("Only explicitly pinned friends can exchange encrypted messages. A nickname is not proof of identity.")
        }
        if(!messages) item {
            Button(onClick={code=true},modifier=Modifier.fillMaxWidth()) {Text("My friend code")}
            OutlinedButton(onClick=scan,modifier=Modifier.fillMaxWidth()) {Text("Scan a friend's code")}
            TextButton(onClick={input=true}) {Text("Paste a friend link")}
            s.replacingFriend?.let { old ->
                Text("Replacing ${old.petname}. Old sending is suspended; scan the new device.",color=MaterialTheme.colorScheme.error)
            }
        }
        if(s.friends.any {it.fresh}) item {
            Text("Recent friend responses",fontWeight=FontWeight.Bold)
            Text(s.friends.filter {it.fresh}.joinToString {it.petname})
            Text("A recent authenticated response does not prove distance or direct range.",fontSize=12.sp)
        }
        if(s.friends.isEmpty()) item {Text("No pinned friends yet. Exchange codes in person from the Friends tab.")}
        items(s.friends,key={it.keys.joinToString()}) { friend ->
            Surface(shape=MaterialTheme.shapes.medium,color=MaterialTheme.colorScheme.surfaceVariant) {
                Column(Modifier.fillMaxWidth().padding(16.dp),verticalArrangement=Arrangement.spacedBy(8.dp)) {
                    Avatar(friend.avatar)
                    Text(friend.petname,fontWeight=FontWeight.Bold,fontSize=20.sp)
                    Text(if(friend.replacing) "Replacement pending - sending blocked" else "Pinned full device identity",fontSize=12.sp)
                    Text(if(friend.fresh) "Recent authenticated response" else friend.responseAgeMs?.let {"Last authenticated response ${it/1000uL}s ago"} ?: "No authenticated response this session",fontSize=12.sp)
                    Row {TextButton(onClick={model.openDirect(friend)}){Text("Open conversation")};if(!messages)TextButton(onClick={change=friend}){Text("Manage")}}
                }
            }
        }
        if(messages && s.archives.isNotEmpty()) {
            item {Text("Old identities",fontWeight=FontWeight.Bold);Text("History stays with its original keys. These entries cannot send.",fontSize=12.sp)}
            items(s.archives,key={it.keys.joinToString()}) { old -> TextButton(onClick={model.openArchive(old)}){Text("${old.nickname} - old identity")} }
        }
    }
    if(code) s.friendCode?.let { own ->
        val context=LocalContext.current
        val bitmap=remember(own.uri) {BarcodeEncoder().encodeBitmap(own.uri,BarcodeFormat.QR_CODE,640,640)}
        AlertDialog(onDismissRequest={code=false},title={Text("My friend code")},text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(12.dp)) {
            Text(s.nickname,fontWeight=FontWeight.Bold)
            Image(bitmap.asImageBitmap(),"Your public friend code",Modifier.fillMaxWidth().aspectRatio(1f).background(Color.White).padding(8.dp))
            Text(own.fingerprint,fontSize=12.sp)
            Text("Compare this full fingerprint on both screens in person. Sharing a code does not add anyone automatically.")
            TextButton(onClick={val clipboard=context.getSystemService(android.content.ClipboardManager::class.java);clipboard.setPrimaryClip(android.content.ClipData.newPlainText("Public friend code",own.uri))}){Text("Copy public friend link")}
            OutlinedButton(onClick={code=false;scan()}){Text("Scan a friend's code")}
        }},confirmButton={TextButton(onClick={code=false}){Text("Done")}})
    }
    if(input) {
        var uri by remember {mutableStateOf("")}
        AlertDialog(onDismissRequest={input=false},title={Text("Paste a friend link")},text={Column {
            Text("A link received online may belong to someone else. Compare the fingerprint in person before pinning it.")
            OutlinedTextField(uri,{if(it.length<=2048)uri=it},label={Text("Friend link")},maxLines=5)
        }},confirmButton={Button(onClick={model.friendInput(uri);input=false},enabled=uri.isNotEmpty()){Text("Review code")}},dismissButton={TextButton(onClick={input=false}){Text("Cancel")}})
    }
    change?.let { friend ->
        AlertDialog(onDismissRequest={change=null},title={Text("Manage ${friend.petname}")},text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(12.dp)) {
            Text(friend.fingerprint,fontSize=12.sp)
            Text("Replacing or removing stops the connection and cancels queued sends. Old history stays under the old keys.")
            TextButton(onClick={model.changeFriend(friend,true);change=null}){Text("Replace device - require new scan")}
            TextButton(onClick={model.changeFriend(friend,false);change=null}){Text("Remove friend")}
        }},confirmButton={TextButton(onClick={change=null}){Text("Cancel")}})
    }
}

@Composable
internal fun FriendConfirmation(model: MeshModel,s: MeshScreenState) {
    val proposal=s.proposal ?: return
    key(proposal.uri) {
        var petname by remember {mutableStateOf(proposal.nickname)}
        var compared by remember {mutableStateOf(false)}
        AlertDialog(onDismissRequest=model::cancelProposal,title={Text(if(s.replacingFriend==null)"Verify this friend" else "Confirm new device keys")},text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(12.dp)) {
            Text("They broadcast as ${proposal.nickname}. This is a claim.")
            s.replacingFriend?.let {Text("Old fingerprint (${it.petname})",fontWeight=FontWeight.Bold);Text(it.fingerprint,fontSize=12.sp)}
            Text("New fingerprint - matches the code on their screen",fontWeight=FontWeight.Bold);Text(proposal.fingerprint,fontSize=12.sp)
            Text(if(s.proposalScanned)"Compare both screens in person. A displayed code can still be copied." else "This link has no in-person guarantee. It may be someone else's code; independently compare the full fingerprint.",color=MaterialTheme.colorScheme.error)
            OutlinedTextField(petname,{if(it.toByteArray().size<=80)petname=it},label={Text("Private petname")},singleLine=true,keyboardOptions=KeyboardOptions(imeAction=ImeAction.Done))
            Text("Only you see this name. A broadcast nickname cannot change it.",fontSize=12.sp)
            Row {Checkbox(compared,{compared=it});Text("I compared the fingerprint and trust this source",Modifier.padding(top=12.dp))}
            Text("Show them your code so it is mutual.",fontSize=12.sp)
        }},confirmButton={Button(onClick={model.confirmFriend(petname)},enabled=compared && petname.isNotBlank() && petname.toByteArray().size<=20){Text("Pin verified identity")}},dismissButton={TextButton(onClick=model::cancelProposal){Text("Cancel")}})
    }
}

@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class, ExperimentalLayoutApi::class)
@Composable
internal fun ColumnScope.DirectChat(model:MeshModel,s:MeshScreenState) {
    val thread=checkNotNull(s.direct)
    var reaction by remember {mutableStateOf<DirectMessage?>(null)}
    var delete by remember {mutableStateOf(false)}
    val blocked=thread.archived || s.friends.none {it.keys.contentEquals(thread.keys) && !it.replacing}
    Text(if(blocked)"Old or suspended identity - sending disabled" else "Encrypted for ${thread.petname}'s pinned device",Modifier.padding(16.dp),fontSize=13.sp)
    Text("Message contents are encrypted. Relays can see timing and stable identifiers. No delivery receipts or forward secrecy.",Modifier.padding(horizontal=16.dp),fontSize=11.sp)
    LazyColumn(Modifier.weight(1f).fillMaxWidth(),contentPadding=PaddingValues(16.dp),verticalArrangement=Arrangement.spacedBy(18.dp)) {
        if(s.directRows.isEmpty()) item {Text("No encrypted messages yet. Both people must pin each other's code.")}
        items(s.directRows,key={it.message.id.joinToString()+it.message.own}) { row -> val message=row.message
            Column(Modifier.fillMaxWidth().combinedClickable(onClick={},onLongClick={if(!blocked)reaction=message})) {
                Text(if(message.own)"You" else thread.petname,fontWeight=FontWeight.Bold)
                Text(message.text,Modifier.padding(vertical=8.dp),fontSize=16.sp)
                Text(row.sendState,fontSize=11.sp)
                FlowRow {message.reactions.forEachIndexed {code,count -> if(count>0)TextButton(onClick={if(code<8 && !blocked)model.reactDirect(message,code.toUByte())}){Text(if(code<8)reactions[code] else "other");Text(" $count")}}}
            }
        }
    }
    if(!blocked) key(thread.keys.joinToString()) {ChannelComposer(false,0,s.posted,model::sendDirect)}
    if(thread.archived) TextButton(onClick={delete=true}) {Text("Delete old conversation")}
    reaction?.let {message -> AlertDialog(onDismissRequest={reaction=null},title={Text("Encrypted reaction")},text={Column {
        for(group in reactions.indices.chunked(4))Row {group.forEach {code -> IconButton(onClick={model.reactDirect(message,code.toUByte());reaction=null},modifier=Modifier.size(56.dp)){MeshIcon(reactions[code])}}}
    }},confirmButton={TextButton(onClick={reaction=null}){Text("Cancel")}})}
    if(delete)AlertDialog(onDismissRequest={delete=false},title={Text("Delete old conversation?")},text={Text("This deletes local visible history for these old keys. Replay protection is retained.")},confirmButton={TextButton(onClick={delete=false;model.deleteOldConversation()}){Text("Delete history")}},dismissButton={TextButton(onClick={delete=false}){Text("Cancel")}})
}
