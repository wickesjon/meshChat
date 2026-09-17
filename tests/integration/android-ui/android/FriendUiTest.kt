package org.meshchat.ui

import android.content.Intent
import android.graphics.Bitmap
import android.net.Uri
import android.os.SystemClock
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import com.google.zxing.*
import com.google.zxing.common.HybridBinarizer
import com.journeyapps.barcodescanner.BarcodeEncoder
import org.junit.Rule
import org.junit.Test
import org.junit.Assert.*
import org.meshchat.app.MainActivity
import org.meshchat.identity.IdentityProvider
import org.meshchat.storage.*
import uniffi.meshchat_core.*
import java.io.File

/** Explicit synthetic emulator phases. No fixture hook is compiled into the app.
 * Camera image decoding exercises the actual offline decoder; physical optics
 * and radio certification remain deferred. Protected SQLCipher stays enabled. */
class FriendUiTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private val model get()=(ui.activity.application as MeshApplication).model
    private val context get()=InstrumentationRegistry.getInstrumentation().targetContext
    private fun awaitState(condition:()->Boolean) {ui.waitUntil(120000) {check(!model.screen.locked){"Protected app unavailable"};condition()}}
    private fun shown(text:String)=ui.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()
    private fun waitFor(text:String)=awaitState {shown(text)}
    private fun click(text:String) {
        if(!shown(text))ui.onNode(hasScrollToNodeAction()).performScrollToNode(hasText(text))
        val node=ui.onNodeWithText(text)
        runCatching {node.assertIsDisplayed()}.onFailure {node.performScrollTo()}
        node.performClick()
    }
    private fun screenshot(name:String) {
        ui.waitForIdle()
        InstrumentationRegistry.getInstrumentation().uiAutomation.waitForIdle(500,5000)
        val bitmap=checkNotNull(InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot())
        File(context.filesDir,"mc029-$name.png").outputStream().use {bitmap.compress(Bitmap.CompressFormat.PNG,100,it)}
    }
    private fun proposal():FriendProposal=IdentityKeySession.importUnlocked(ByteArray(64){29},ByteArray(16){29}).use {friendCode(it.publicIdentity(),"Broadcast Bob")}
    @Test fun productionFriendsAcceptance() {
        val phase=InstrumentationRegistry.getArguments().getString("phase")
        try {when(phase) {
            "pair"->pair()
            "reopen"->reopen()
            "replace"->replace()
            else->error("Explicit synthetic emulator phase required")
        }} catch(failure:Throwable) {runCatching {screenshot("failed-$phase")};throw failure}
    }
    private fun pair() {
        waitFor("Your channels");click("Friends");waitFor("My friend code")
        click("Paste a friend link");ui.onNodeWithText("Friend link").performTextInput("meshfest://friend/invalid/name");click("Review code")
        awaitState {model.screen.error?.startsWith("Check the friend code")==true};assertTrue(model.screen.friends.isEmpty())
        // Actual ACTION_VIEW entry remains inert until the confirmation button.
        val code=proposal()
        val activity=ui.activity
        val launchIntent=activity.intent
        ui.runOnUiThread {ui.activity.startActivity(Intent(context,MainActivity::class.java).setAction(Intent.ACTION_VIEW).setData(Uri.parse(code.uri)).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP))}
        waitFor("Verify this friend");assertTrue(model.screen.friends.isEmpty())
        // ActivityScenario tracks lifecycle by the original launch intent. The
        // real ACTION_VIEW has been delivered; restore only its test bookkeeping
        // identity so permission/lifecycle transitions and teardown remain tracked.
        InstrumentationRegistry.getInstrumentation().runOnMainSync {activity.intent=launchIntent}
        ui.onNodeWithText("Pin verified identity").assertIsNotEnabled()
        ui.onNodeWithText("Private petname").performTextReplacement("Local Bob")
        click("Cancel");awaitState {model.screen.proposal==null};assertTrue(model.screen.friends.isEmpty())
        // The same production QR encoder and scanner's ZXing backend decode a
        // synthetic image. No camera result is injected into a production hook.
        val bitmap=BarcodeEncoder().encodeBitmap(code.uri,BarcodeFormat.QR_CODE,640,640)
        val pixels=IntArray(640*640);bitmap.getPixels(pixels,0,640,0,0,640,640)
        val decoded=MultiFormatReader().decode(BinaryBitmap(HybridBinarizer(RGBLuminanceSource(640,640,pixels))),mapOf(DecodeHintType.POSSIBLE_FORMATS to listOf(BarcodeFormat.QR_CODE)))
        assertEquals(code.uri,decoded.text)
        model.friendInput(decoded.text,scanned=true)
        waitFor("Verify this friend");ui.onNodeWithText("Private petname").performTextReplacement("Local Bob")
        ui.onNodeWithText("Private petname").performImeAction()
        ui.onNode(isToggleable()).performScrollTo().performClick()
        screenshot("confirmation");click("Pin verified identity")
        awaitState {model.screen.friends.size==1 && model.screen.proposal==null}
        assertEquals("Local Bob",model.screen.friends.single().petname)
        click("Friends")
        // The pin is already confirmed in model state above. A small CI viewport
        // can leave its LazyColumn item uncomposed below the entry controls.
        ui.onNode(hasScrollToNodeAction()).performScrollToNode(hasText("Local Bob"))
        waitFor("Local Bob");screenshot("friends")
        // Denial callback retains manual entry. The runner revokes camera access;
        // no test relaxes the app's camera or replacement requirements.
        click("Scan a friend's code")
        val automation=InstrumentationRegistry.getInstrumentation().uiAutomation
        var denied=false
        val deadline=SystemClock.elapsedRealtime()+15000
        while(!denied && SystemClock.elapsedRealtime()<deadline) {
            val buttons=automation.rootInActiveWindow?.findAccessibilityNodeInfosByViewId("com.android.permissioncontroller:id/permission_deny_button").orEmpty()
            denied=buttons.firstOrNull()?.performAction(android.view.accessibility.AccessibilityNodeInfo.ACTION_CLICK)==true
            if(!denied)SystemClock.sleep(100)
        }
        assertTrue("Actual camera permission denial button was required",denied)
        awaitState {model.screen.error?.startsWith("Camera access")==true}
        assertTrue(shown("Paste a friend link"))
        exchangeProtectedMessages()
        click("Open conversation");waitFor("Synthetic encrypted hello");screenshot("dm")
        assertEquals(2,model.screen.directRows.size)
        val incoming=model.screen.directRows.first { !it.message.own }.message
        assertEquals(1,incoming.reactions[2].toInt())
        ui.onNode(hasSetTextAction()).performTextInput("Offline encrypted draft");click("Send")
        awaitState {model.screen.error?.startsWith("Not sent.")==true}
        assertEquals(2,model.screen.directRows.size)
    }
    private fun reopen() {
        waitFor("Your channels");assertEquals("Local Bob",model.screen.friends.single().petname)
        click("Messages");click("Open conversation");waitFor("Synthetic encrypted hello")
        assertEquals(2,model.screen.directRows.size);assertEquals(1,model.screen.directRows.first { !it.message.own }.message.reactions[2].toInt())
        screenshot("reopened-dm")
    }
    private fun replace() {
        waitFor("Your channels")
        archiveCapacityRefusal()
        click("Friends");click("Manage");click("Replace device - require new scan")
        awaitState {model.screen.friends.single().replacing}
        val old=model.screen.friends.single().keys.copyOf()
        model.friendInput(proposal().uri,false)
        awaitState {model.screen.error?.startsWith("Replacement requires")==true};assertNull(model.screen.proposal)
        val next=IdentityKeySession.importUnlocked(ByteArray(64){30},ByteArray(16){30}).use {friendCode(it.publicIdentity(),"New Bob")}
        model.friendInput(next.uri,true);waitFor("Confirm new device keys");assertTrue(shown("Old fingerprint (Local Bob)"))
        ui.onNodeWithText("Private petname").performTextReplacement("New Bob")
        ui.onNodeWithText("Private petname").performImeAction()
        ui.onNode(isToggleable()).performScrollTo().performClick();click("Pin verified identity")
        awaitState {model.screen.friends.single().petname=="New Bob" && model.screen.proposal==null}
        assertFalse(old.contentEquals(model.screen.friends.single().keys));click("Messages")
        click("Local Bob - old identity");waitFor("Synthetic encrypted hello");assertTrue(model.screen.direct!!.archived)
        ui.onAllNodesWithText("Send").assertCountEquals(0);screenshot("old-identity")
        click("Back");awaitState {model.screen.direct==null};waitFor("Friends")
        click("Friends");click("Manage");click("Remove friend")
        awaitState {model.screen.friends.isEmpty()};assertTrue(model.screen.archives.any {it.keys.contentEquals(old)})
    }
    private fun archiveCapacityRefusal() {
        val vault=StorageVault.android(context)
        val storage=EncryptedStorage(IdentityProvider.android(context,vault),vault)
        val codes=(40 until 104).map { seed ->
            IdentityKeySession.importUnlocked(ByteArray(64){seed.toByte()},ByteArray(16){seed.toByte()}).use {friendCode(it.publicIdentity(),"Old $seed").uri}
        }
        for(slot in 0 until 8)storage.putRecord(RecordKind.SETTING,"ui-old-friends-$slot".toByteArray(),codes.drop(slot*8).take(8).joinToString("\n").toByteArray())
        // The model advances its connection generation on every stop, even with
        // no physical radio. Observe that lifecycle effect without a production
        // fixture hook or a claim that emulator Bluetooth is device evidence.
        val epoch=MeshModel::class.java.getDeclaredField("epoch").apply {isAccessible=true}
        val before=epoch.getLong(model)
        val friend=model.screen.friends.single()
        for(replace in listOf(true,false)) {
            model.changeFriend(friend,replace)
            awaitState {model.screen.error?.startsWith("Old conversation list is full")==true}
            assertEquals(before,epoch.getLong(model))
            assertFalse(model.screen.friends.single().replacing)
            assertTrue(friend.keys.contentEquals(model.screen.friends.single().keys))
            // Finish queued UI work and clear the notice before the next attempt.
            model.openDirect(friend);awaitState {model.screen.direct!=null}
            model.closeDirect();awaitState {model.screen.direct==null}
        }
        for(slot in 0 until 8)storage.deleteRecord(RecordKind.SETTING,"ui-old-friends-$slot".toByteArray())
    }
    private fun exchangeProtectedMessages() {
        val vault=StorageVault.android(context);val identity=IdentityProvider.android(context,vault);val storage=EncryptedStorage(identity,vault)
        val initial=SystemClock.elapsedRealtime().toULong();var now=initial
        val wall=System.currentTimeMillis()/1000
        // Separate synthetic peer uses real SQLCipher with a test-only key; the
        // application side always uses its production protected provider/vault.
        val file=File.createTempFile("mc029-peer-",".db",context.cacheDir).also {check(it.delete())}
        try {IdentityKeySession.importUnlocked(ByteArray(64){29},ByteArray(16){29}).use { bKey ->
            CipherConnection.open(file,ByteArray(64){29},true).use { db ->
                EncryptedStore.open(db,bKey.publicIdentity().generation,true,wall).use {bStore ->
                    NativeTransport(bStore,bKey.publicIdentity(),29u,now).use {b -> storage.transport(31u,now).use {a ->
                        b.confirmFriend(bStore,bKey,storage.friendCode("Synthetic Alice").uri,"Alice",null,now)
                        val aPin=storage.friendCards(a,now).single();val bPin=b.friendCards(bStore,now).single()
                        val al=a.nativeReady(a.admitConnection(ByteArray(16){1},now),TransportRole.CENTRAL,512u,512u,now).link
                        val bl=b.nativeReady(b.admitConnection(ByteArray(16){2},now),TransportRole.PERIPHERAL,512u,512u,now).link
                        val af=a.tick(now).sends.single();val bf=b.tick(now).sends.single()
                        b.receive(bl,af.bytes.size.toULong(),af.bytes,now);a.receive(al,bf.bytes.size.toULong(),bf.bytes,now)
                        a.complete(al,af.token,true,now);b.complete(bl,bf.token,true,now)
                        fun deliver(sender:NativeTransport,receiver:NativeTransport,sl:LinkHandle,rl:LinkHandle,toApp:Boolean) {
                            repeat(8) {
                                now+=1000u
                                val frame=sender.tick(now).sends.single()
                                if(toApp) sender.authorizeMessageEgress(bStore,sl,frame.token)
                                val receive={receiver.receive(rl,frame.bytes.size.toULong(),frame.bytes,now)}
                                val events=if(toApp)receive().events else {
                                    var effects:TransportEffects?=null
                                    assertTrue(storage.messageEgress(sender,frame) {effects=receive();true})
                                    checkNotNull(effects).events
                                }
                                val done=sender.complete(sl,frame.token,true,now).events.any {it is TransportEvent.Finished}
                                events.filterIsInstance<TransportEvent.Received>().forEach {event ->
                                    if(toApp)assertTrue(storage.authenticate(receiver,rl,event.bytes,now).authenticated)
                                    else assertTrue(receiver.authenticateMessage(bStore,bKey,rl,event.bytes,now,wall).authenticated)
                                }
                                if(done)return
                            };error("bounded exchange did not finish")
                        }
                        now+=1000u
                        val incoming=b.sendDirect(bStore,bKey,bPin.handle,DirectContent.Chat("Synthetic encrypted hello"),7u,now,wall)
                        deliver(b,a,bl,al,true)
                        now+=1000u;storage.sendDirect(a,aPin.handle,DirectContent.Chat("Synthetic encrypted reply"),8u,now)
                        deliver(a,b,al,bl,false)
                        now+=1000u;storage.sendDirect(a,aPin.handle,DirectContent.Reaction(incoming.id,false,2u),9u,now)
                        deliver(a,b,al,bl,false)
                        assertEquals(2,storage.directHistory(a,aPin.keys,now).size)
                        a.disconnect(al,now);b.disconnect(bl,now)
                    }}
                }
            }
        }} finally {file.delete()}
        model.load()
    }
}
