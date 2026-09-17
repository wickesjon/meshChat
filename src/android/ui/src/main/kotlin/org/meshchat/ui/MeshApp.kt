package org.meshchat.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import uniffi.meshchat_core.*

@Composable
fun MeshApp(model: MeshModel, permissions: () -> Unit, scan: () -> Unit = {}, buySupporter: ()->Unit = {}, restoreSupporter: ()->Unit = {}) {
    val s = model.screen
    val colors = meshColors(Themes.selected(s.theme,s.supporter))
    MaterialTheme(colorScheme=colors) {
        Surface(Modifier.fillMaxSize(), color=colors.background) {
            when {
                s.loading -> Box(Modifier.fillMaxSize(),contentAlignment=Alignment.Center) { CircularProgressIndicator() }
                s.locked -> Column(Modifier.padding(28.dp),verticalArrangement=Arrangement.spacedBy(20.dp)) {
                    Spacer(Modifier.height(40.dp)); MeshIcon("closed-lock")
                    Text("Your data is locked",fontSize=30.sp,fontWeight=FontWeight.Bold)
                    Text(s.error ?: "Unlock your device to reopen.")
                    Button(onClick=model::load) { Text("Reopen") }; ResetAction(model)
                }
                !s.onboarded -> Onboarding(model,s,permissions)
                s.beaconRequested || s.beacon?.active == true -> BeaconScreen(s,model::exitBeacon,model::startRadio,permissions)
                else -> Home(model,s,permissions,scan,buySupporter,restoreSupporter)
            }
        }
    }
}
internal fun meshColors(t:ThemeTokens):ColorScheme = (if(t.light)lightColorScheme() else darkColorScheme()).copy(
    primary=Color(t.accent),onPrimary=Color(t.onAccent),primaryContainer=Color(t.surface),onPrimaryContainer=Color(t.text),
    secondary=Color(t.accent),onSecondary=Color(t.onAccent),secondaryContainer=Color(t.surface),onSecondaryContainer=Color(t.text),
    tertiary=Color(t.accent),onTertiary=Color(t.onAccent),tertiaryContainer=Color(t.surface),onTertiaryContainer=Color(t.text),
    background=Color(t.background),onBackground=Color(t.text),surface=Color(t.surface),onSurface=Color(t.text),
    surfaceVariant=Color(t.surface),onSurfaceVariant=Color(t.secondary),outline=Color(t.secondary),outlineVariant=Color(t.secondary),
    surfaceContainer=Color(t.surface),surfaceContainerHigh=Color(t.surface),surfaceContainerHighest=Color(t.surface),
    surfaceContainerLow=Color(t.surface),surfaceContainerLowest=Color(t.background),surfaceBright=Color(t.surface),surfaceDim=Color(t.background),
    surfaceTint=Color.Transparent,error=Color(t.error),onError=Color(t.background),errorContainer=Color(t.surface),onErrorContainer=Color(t.error),
    inverseSurface=Color(t.text),inverseOnSurface=Color(t.surface),inversePrimary=Color(t.background))
@Composable
private fun Onboarding(model: MeshModel,s: MeshScreenState,permissions: () -> Unit) {
    var step by rememberSaveable { mutableIntStateOf(0) }
    var nick by rememberSaveable { mutableStateOf("") }
    var avatar by rememberSaveable { mutableIntStateOf(1) }
    Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(24.dp),verticalArrangement=Arrangement.spacedBy(20.dp)) {
        Spacer(Modifier.height(24.dp)); Text("MESHCHAT",color=MaterialTheme.colorScheme.primary,letterSpacing=4.sp)
        Text(listOf("Find your people.","Make room for nearby.","A little mesh know-how.")[step],fontSize=36.sp,fontWeight=FontWeight.Bold)
        Text("${step+1} / 3",color=MaterialTheme.colorScheme.onSurfaceVariant)
        when (step) {
            0 -> {
                Text("Choose a name and an animal. Public names and avatars are claims, not verified identities.")
                OutlinedTextField(nick,{if(it.toByteArray().size<=80)nick=it},label={Text("Nickname · 20 UTF-8 bytes")},singleLine=true,modifier=Modifier.fillMaxWidth())
                ProfilePicker(avatar.toUByte()) { avatar=it.toInt() }
            }
            1 -> {
                Text("Bluetooth connects you to nearby phones. Location access supports discovery and signal observations; notifications keep the connection visible.")
                Button(onClick=permissions) { Text("Review permissions") }
                Text("Android may limit background activity. Review battery settings if discovery stops with the screen off. We never change those settings for you.")
            }
            2 -> {
                Text("Channels are readable on air, including channels shared with three words. Your ordinary sender identifier stays stable and can be tracked.")
                Text("Confessions posts use a fresh identifier and a neutral mask. Reactions still use your stable identifier.")
                Text("Messages travel through nearby phones. There is no delivery guarantee. Airplane mode can save power if you turn Bluetooth back on.")
                Text("Development build: use synthetic messages until device protection testing is complete.",color=MaterialTheme.colorScheme.primary)
            }
        }
        s.error?.let { Text(it,color=MaterialTheme.colorScheme.error) }
        Button(onClick={if(step<2)step++ else model.create(nick,avatar.toUByte())},enabled=nick.isNotBlank()&&nick.toByteArray().size<=20,modifier=Modifier.fillMaxWidth().heightIn(min=52.dp)) { Text(if(step==2)"Create my profile" else "Continue") }
        if(step>0)TextButton(onClick={step--}) { Text("Back") }
    }
}
@Composable
private fun ProfilePicker(value: UByte,changed: (UByte)->Unit) {
    Column(verticalArrangement=Arrangement.spacedBy(8.dp)) {
        for(group in (1..8).chunked(4))Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceEvenly) {
            for(i in group)Box(Modifier.size(56.dp).border(if((value.toInt() and 15)==i)2.dp else 0.dp,MaterialTheme.colorScheme.primary,CircleShape).clickable { changed(((value.toInt() and 240) or i).toUByte()) }.semantics { contentDescription="Choose ${animals[i]}" },contentAlignment=Alignment.Center) { Avatar(((value.toInt() and 240) or i).toUByte()) }
        }
        for(group in (0..15).chunked(4))Row(Modifier.fillMaxWidth(),horizontalArrangement=Arrangement.SpaceEvenly) {
            for(i in group)Box(Modifier.size(48.dp).clickable { changed(((i shl 4) or (value.toInt() and 15)).toUByte()) }.semantics { contentDescription="Avatar color ${i+1}" },contentAlignment=Alignment.Center) { Box(Modifier.size(24.dp).background(palette[i],CircleShape)) }
        }
    }
}
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun Home(model: MeshModel,s: MeshScreenState,permissions: () -> Unit,scan: () -> Unit,buySupporter: ()->Unit,restoreSupporter: ()->Unit) {
    var tab by rememberSaveable { mutableIntStateOf(0) }
    var settings by remember { mutableStateOf(false) }
    var join by remember { mutableStateOf(false) }
    var info by remember { mutableStateOf(false) }
    var share by remember { mutableStateOf(false) }
    var linkInput by remember { mutableStateOf(false) }
    Scaffold(containerColor=MaterialTheme.colorScheme.background,topBar={Column {
        Row(Modifier.fillMaxWidth().padding(horizontal=20.dp,vertical=16.dp),verticalAlignment=Alignment.CenterVertically) {
            if(s.direct!=null)TextButton(onClick=model::closeDirect) { Text("Back") }
            if(s.selected!=null)TextButton(onClick={model.select(null)}) { Text("Back") }
            Column(Modifier.weight(1f)) { Text("NEARBY, TOGETHER",fontSize=10.sp,letterSpacing=2.sp,color=MaterialTheme.colorScheme.primary); Text(s.direct?.petname ?: s.selected?.name?.replace('|',' ') ?: "MeshChat",fontSize=26.sp,fontWeight=FontWeight.Bold) }
            TextButton(onClick={if(s.selected!=null)info=true else settings=true}) { Text(if(s.selected!=null)"Info" else "Settings") }
        }
        Row(Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surface).padding(horizontal=20.dp,vertical=8.dp),verticalAlignment=Alignment.CenterVertically) {
            Box(Modifier.size(7.dp).background(if(s.peers>0)MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,CircleShape)); Spacer(Modifier.width(10.dp))
            Text(if(s.peers>0)"${s.peers} direct connections · ${s.status}" else s.status,fontSize=12.sp,modifier=Modifier.weight(1f))
            TextButton(onClick=model::startRadio) { Text("Connect") }
        }
    }},bottomBar={if(s.selected==null && s.direct==null)NavigationBar {
        listOf("Channels","Messages","Friends").forEachIndexed { index,title -> NavigationBarItem(selected=tab==index,onClick={tab=index},icon={MeshIcon(if(index==0)"open-lock" else if(index==1)"closed-lock" else "cat")},label={Text(title)}) }
    }}) { pad ->
        Column(Modifier.padding(pad).fillMaxSize()) {
            s.error?.let { Text(it,Modifier.fillMaxWidth().padding(16.dp),color=MaterialTheme.colorScheme.error) }
            if(s.direct!=null)DirectChat(model,s) else if(s.selected!=null)Chat(model,s) else when(tab) {
                0 -> LazyColumn(Modifier.fillMaxSize(),contentPadding=PaddingValues(20.dp),verticalArrangement=Arrangement.spacedBy(12.dp)) {
                    item { Text("Your channels",fontSize=22.sp,fontWeight=FontWeight.SemiBold); Text("Open conversations. Readable on air.",color=MaterialTheme.colorScheme.onSurfaceVariant) }
                    items(s.channels,key={it.name}) { channel -> Surface(onClick={model.select(channel.name)},shape=RoundedCornerShape(18.dp),color=MaterialTheme.colorScheme.surface) {
                        Row(Modifier.fillMaxWidth().padding(18.dp),verticalAlignment=Alignment.CenterVertically) {
                            MeshIcon(channel.glyph); Spacer(Modifier.width(16.dp))
                            Column(Modifier.weight(1f)) { Text(channel.name.replace('|',' '),fontWeight=FontWeight.SemiBold); Text(if(channel.anonymous)"Fresh identity for every post" else if(channel.private)"Anyone with the words can read" else "Open to everyone",fontSize=12.sp,color=MaterialTheme.colorScheme.onSurfaceVariant); Text(s.previews[channel.name] ?: "Quiet so far",maxLines=1,overflow=TextOverflow.Ellipsis,fontSize=13.sp) }
                            s.unread[channel.name]?.takeIf{it>0}?.let{Badge{Text(it.toString())}}
                            MeshIcon("open-lock",modifier=Modifier.size(18.dp))
                        }
                    } }
                    item { OutlinedButton(onClick={join=true},modifier=Modifier.fillMaxWidth().heightIn(min=52.dp)) { Text("Join a word-triple channel") }; TextButton(onClick=permissions) { Text("Review connection permissions") } }
                }
                else -> FriendsPage(model,s,scan,tab==1)
            }
        }
    }
    FriendConfirmation(model,s)
    ChannelConfirmation(model,s)
    if(linkInput)ShareInput(model) {linkInput=false}
    if(share)s.selected?.takeIf {it.private}?.let {ChannelShare(it) {share=false}}
    if(join)JoinSheet({join=false},scan={join=false;scan()},paste={join=false;linkInput=true}) { model.join(it);join=false }
    if(settings)SettingsSheet(model,s,buySupporter,restoreSupporter) { settings=false }
    if(info)AlertDialog(onDismissRequest={info=false},title={Text("Channel details")},text={Column(verticalArrangement=Arrangement.spacedBy(12.dp)) {
        Text("Anyone with these words can read this channel. It is not encrypted.");Text(s.selected?.name?.replace('|',' ') ?: "")
        if(s.selected?.private==true)TextButton(onClick={info=false;share=true}){Text("Share channel")}
        TextButton(onClick=model::mute){Text(if(s.muted)"Unmute channel" else "Mute channel")}
        if(s.selected?.private==true)TextButton(onClick={model.leave();info=false}){Text("Leave channel")}
        TextButton(onClick={model.clearHistory();info=false}){Text("Delete local channel history")}
    }},confirmButton={TextButton(onClick={info=false}){Text("Done")}})
}
@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class, ExperimentalLayoutApi::class)
@Composable
private fun ColumnScope.Chat(model: MeshModel,s: MeshScreenState) {
    var reaction by remember { mutableStateOf<ChannelMessage?>(null) }
    val channel=checkNotNull(s.selected)
    Text(if(channel.anonymous)"Posts use a fresh identity. Reactions do not." else "Open channel · readable on air",Modifier.padding(horizontal=20.dp,vertical=10.dp),fontSize=12.sp,color=MaterialTheme.colorScheme.onSurfaceVariant)
    LazyColumn(Modifier.weight(1f).fillMaxWidth(),contentPadding=PaddingValues(16.dp),verticalArrangement=Arrangement.spacedBy(16.dp)) {
        if(s.rows.isEmpty())item{Text("Quiet so far. Messages appear when people within mesh range post.",Modifier.padding(vertical=40.dp),color=MaterialTheme.colorScheme.onSurfaceVariant)}
        items(s.rows,key={it.message.id.joinToString()+it.message.sender.joinToString()}) { row -> val message=row.message
            Row(Modifier.fillMaxWidth().combinedClickable(onClick={},onLongClick={reaction=message}),horizontalArrangement=Arrangement.spacedBy(12.dp)) {
                Avatar(message.avatar)
                Column(Modifier.weight(1f)) {
                    val tokens=Themes.selected(s.theme,s.supporter)
                    val rawColor=message.nicknameRgb?.toLong()?.or(0xff000000L) ?: (nicknameColor(message.sender).toArgb().toLong() and 0xffffffffL)
                    Row(verticalAlignment=Alignment.CenterVertically) { Text(message.nickname,Modifier.weight(1f),maxLines=1,overflow=TextOverflow.Ellipsis,fontWeight=FontWeight.Bold,color=if(channel.anonymous)MaterialTheme.colorScheme.onSurface else Color(Themes.readableNickname(rawColor,tokens)));Text(if(message.own)"You" else if(message.verifiedPetname!=null)"Verified friend" else if(message.signed)"Signed device" else "Unverified",fontSize=10.sp,color=MaterialTheme.colorScheme.onSurfaceVariant) }
                    if(!channel.anonymous && message.supporterHint)Text("Supporter flair · unverified",fontSize=11.sp,color=MaterialTheme.colorScheme.onSurfaceVariant)
                    message.claimWarning?.let {Text(it,fontSize=11.sp,color=MaterialTheme.colorScheme.error)}
                    if(message.confusable)Text("Name resembles yours; identity is unverified",fontSize=11.sp,color=MaterialTheme.colorScheme.error)
                    if(!channel.anonymous)Text(message.sender.takeLast(2).joinToString(""){"%02x".format(it.toInt() and 255)},fontSize=10.sp,color=MaterialTheme.colorScheme.onSurfaceVariant)
                    Text(message.text,Modifier.padding(vertical=6.dp),fontSize=16.sp,lineHeight=23.sp)
                    if(message.own)Text(row.sendState,fontSize=11.sp,color=MaterialTheme.colorScheme.onSurfaceVariant)
                    FlowRow(maxItemsInEachRow=4,horizontalArrangement=Arrangement.spacedBy(5.dp)) { message.reactions.forEachIndexed { code,count -> if(count>0u)TextButton(onClick={if(code<8)model.react(message,code.toUByte())},contentPadding=PaddingValues(3.dp)) { if(code<8)MeshIcon(reactions[code],modifier=Modifier.size(18.dp)) else Text("+1");Text(if(count>=30u)"30+" else count.toString()) } } }
                }
            }
        }
    }
    key(channel.name) { ChannelComposer(channel.anonymous,s.waitSeconds,s.posted,model::send) }
    reaction?.let { message -> AlertDialog(onDismissRequest={reaction=null},title={Text("React to this message")},text={Column {
        Text("Reactions use your stable identifier, including in Confessions.")
        for(group in reactions.indices.chunked(4))Row{group.forEach{code->IconButton(onClick={model.react(message,code.toUByte());reaction=null},modifier=Modifier.size(56.dp)){MeshIcon(reactions[code])}}}
    }},confirmButton={TextButton(onClick={reaction=null}){Text("Cancel")}}) }
}
@Composable
internal fun ChannelComposer(anonymous: Boolean,waitSeconds: Int,posted: Long,send: (String)->Unit) {
    var draft by remember { mutableStateOf("") }
    LaunchedEffect(posted) { if(posted>0)draft="" }
    Column(Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surface).padding(16.dp).imePadding()) {
        Row(verticalAlignment=Alignment.CenterVertically,horizontalArrangement=Arrangement.spacedBy(10.dp)) {
            OutlinedTextField(draft,{if(it.toByteArray().size<=1120)draft=it},placeholder={Text(if(anonymous)"Share anonymously…" else "Say something…")},modifier=Modifier.weight(1f),maxLines=5)
            Button(onClick={send(draft)},modifier=Modifier.heightIn(min=48.dp),enabled=draft.isNotBlank()&&draft.toByteArray().size<=280&&waitSeconds==0){Text("Send")}
        }
        Column(Modifier.fillMaxWidth()) { Text(if(waitSeconds>0)"Post again in ${waitSeconds}s" else "Best effort. No delivery receipts.",fontSize=11.sp,color=MaterialTheme.colorScheme.onSurfaceVariant);Text("${280-draft.toByteArray().size} bytes left",fontSize=11.sp) }
    }
}
@Composable
private fun JoinSheet(close: ()->Unit,scan: ()->Unit,paste: ()->Unit,join: (String)->Unit) {
    val words=remember{channelWords()};var a by remember{mutableIntStateOf(0)};var b by remember{mutableIntStateOf(0)};var c by remember{mutableIntStateOf(0)}
    val name="${words.descriptors[a]}|${words.genres[b]}|${words.locations[c]}"
    AlertDialog(onDismissRequest=close,title={Text("Find your channel")},text={Column(verticalArrangement=Arrangement.spacedBy(12.dp)) {
        Text("Choose the same three words as your friends. Anyone with the words can read this channel.")
        WordChoice(words.descriptors,a){a=it};WordChoice(words.genres,b){b=it};WordChoice(words.locations,c){c=it}
        Text(name.replace('|',' '),fontWeight=FontWeight.Bold)
        OutlinedButton(onClick=scan){Text("Scan QR instead")}
        TextButton(onClick=paste){Text("Paste a shared link")}
        TextButton(onClick={val r=java.security.SecureRandom();a=r.nextInt(20);b=r.nextInt(20);c=r.nextInt(20)}){Text("Randomize")}
    }},confirmButton={Button(onClick={join(name)}){Text("Join channel")}},dismissButton={TextButton(onClick=close){Text("Cancel")}})
}
@Composable
private fun WordChoice(words: List<String>,selected: Int,choose: (Int)->Unit) {
    var open by remember{mutableStateOf(false)}
    Box{OutlinedButton(onClick={open=true},modifier=Modifier.fillMaxWidth().heightIn(min=48.dp)){Text(words[selected])};DropdownMenu(expanded=open,onDismissRequest={open=false}){words.forEachIndexed{index,word->DropdownMenuItem(text={Text(word)},onClick={choose(index);open=false})}}}
}
@Composable
private fun SettingsSheet(model: MeshModel,s: MeshScreenState,buySupporter: ()->Unit,restoreSupporter: ()->Unit,close: ()->Unit) {
    var nick by remember{mutableStateOf(s.nickname)};var avatar by remember{mutableStateOf(s.avatar)};var light by remember{mutableStateOf(s.light)}
    var theme by remember{mutableStateOf(Themes.selected(s.theme,s.supporter).id)}
    var color by remember{mutableStateOf(s.nicknameRgb?.toString(16)?.padStart(6,'0') ?: "")}
    AlertDialog(onDismissRequest=close,title={Text("Make it yours")},text={Column(Modifier.verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(12.dp)) {
        OutlinedTextField(nick,{if(it.toByteArray().size<=80)nick=it},label={Text("Nickname")},singleLine=true)
        ProfilePicker(avatar){avatar=it};Row(verticalAlignment=Alignment.CenterVertically){Text("Light theme",Modifier.weight(1f));Switch(light,{light=it;theme=if(it)"daylight" else "afterhours"},modifier=Modifier.semantics { contentDescription="Light theme" })}
        Text("Theme")
        Themes.all.forEach { t -> TextButton(enabled=!t.paid || s.supporter,onClick={theme=t.id;light=t.light}) {
            Text("${if(theme==t.id)"Selected: " else ""}${t.title}${if(t.paid)" · Supporter" else ""}")
        } }
        if(s.supporter) {
            OutlinedTextField(color,{if(it.length<=6 && it.all { c -> c in "0123456789abcdefABCDEF" })color=it},label={Text("Nickname color · six hex digits")},singleLine=true)
            TextButton(onClick={color=""}) { Text("Use automatic nickname color") }
            Text("Receivers may adjust colors for readability. Suffix and trust labels stay separate.")
        }
        Text(if(s.supporter)"Supporter · ready offline" else "Supporter · optional")
        Text("Extra themes, nickname color and up to 30 private channel slots. Messaging, friends and relaying stay free.")
        Text(s.billingStatus)
        Button(onClick=buySupporter,enabled=s.supporterPrice!=null && !s.supporter) { Text(s.supporterPrice?.let { "Supporter · $it · one-time" } ?: "Purchases unavailable") }
        TextButton(onClick=restoreSupporter) { Text("Restore Supporter") }
        Text("Beacon Mode · Android")
        Text("Beacon Mode uses more power; keep your phone charged. It ends at 30% battery when unplugged. No delivery or coverage guarantee.")
        Button(onClick={model.beacon(true,s.autoBeacon);close()}) { Text("Start Beacon Mode") }
        Row(verticalAlignment=Alignment.CenterVertically) {
            Text("Auto-beacon while charging",Modifier.weight(1f))
            Switch(s.autoBeacon,{model.beacon(false,it)},modifier=Modifier.semantics { contentDescription="Auto-beacon while charging" })
        }
        Text("Automatic mode returns to ordinary operation when unplugged.")
        Text("Nearby connection power")
        TransportPowerSetting.entries.forEach { value ->
            Row(Modifier.fillMaxWidth().heightIn(min=48.dp).clickable { model.power(value) },verticalAlignment=Alignment.CenterVertically) {
                RadioButton(selected=s.power==value,onClick={model.power(value)});Text(value.name.lowercase().replaceFirstChar{it.uppercase()})
            }
        }
        TextButton(onClick=model::stop){Text("Stop nearby connection")};ResetAction(model)
    }},confirmButton={Button(onClick={model.updateProfile(nick,avatar,light,theme,color.toUIntOrNull(16));close()},enabled=nick.isNotBlank()&&nick.toByteArray().size<=20&&(color.isEmpty()||color.length==6)){Text("Save")}},dismissButton={TextButton(onClick=close){Text("Cancel")}})
}
@Composable
private fun ResetAction(model: MeshModel) {
    var confirm by remember{mutableStateOf(false)}
    TextButton(onClick={confirm=true}){Text("Reset identity and local data",color=MaterialTheme.colorScheme.error)}
    if(confirm)AlertDialog(onDismissRequest={confirm=false},title={Text("Reset this device?")},text={Text("This removes local history and pinned trust and creates a new identity. Friends must verify your new code. This cannot be undone.")},confirmButton={TextButton(onClick={confirm=false;model.resetIdentity()}){Text("Reset identity")}},dismissButton={TextButton(onClick={confirm=false}){Text("Cancel")}})
}
