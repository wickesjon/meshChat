package org.meshchat.ui

import android.graphics.Bitmap
import android.os.SystemClock
import android.view.MotionEvent
import android.view.WindowManager
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import androidx.lifecycle.Lifecycle
import org.junit.Rule
import org.junit.Test
import org.junit.Assert.*
import org.meshchat.app.MainActivity
import org.meshchat.identity.*
import org.meshchat.storage.*
import uniffi.meshchat_core.*
import java.io.File

/** Only generated synthetic credentials, real protected Android storage and
 * production UI. The test peer uses SQLCipher; frames are exchanged in process. */
class OrganizerUiTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private val model get()=(ui.activity.application as MeshApplication).model
    private val context get()=InstrumentationRegistry.getInstrumentation().targetContext
    private val fixtures by lazy {InstrumentationRegistry.getInstrumentation().context.assets.open("events.tsv").bufferedReader().useLines {lines->lines.associate {val p=it.split('\t',limit=2);p[0] to p[1]}}}
    private fun awaitState(condition:()->Boolean) {ui.waitUntil(120000) {check(!model.screen.locked){"Protected app unavailable"};condition()}}
    private fun shown(text:String)=ui.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()
    private fun click(text:String) {val node=ui.onNodeWithText(text);runCatching {node.assertIsDisplayed()}.onFailure {node.performScrollTo()};node.performClick()}
    private fun screenshot(name:String) {
        check(model.screen.staffProposal==null)
        ui.waitForIdle();val bitmap=checkNotNull(InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot())
        File(context.filesDir,"mc030-$name.png").outputStream().use {bitmap.compress(Bitmap.CompressFormat.PNG,100,it)}
    }
    @Test fun productionOrganizerAcceptance() {
        awaitState {model.screen.onboarded}
        when(InstrumentationRegistry.getArguments().getString("phase")) {
            "import"->provision()
            "reopen"->reopen()
            "forget"->forget()
            else->error("Explicit synthetic phase required")
        }
    }
    private fun provision() {
        assertTrue(model.screen.events.isEmpty());model.shareInput(fixtures.getValue("event"),true)
        awaitState {model.screen.eventProposal!=null};assertTrue(model.screen.events.isEmpty())
        click("Cancel");awaitState {model.screen.eventProposal==null};assertTrue(model.screen.events.isEmpty())
        model.shareInput(fixtures.getValue("event"),true);awaitState {shown("Adopt event")};click("Adopt event")
        awaitState {model.screen.events.size==1 && model.screen.eventProposal==null}
        model.select("#event updates");awaitState {shown("Only event staff can post here")}
        model.staffInput(fixtures.getValue("staff"));awaitState {shown("Import staff key")}
        assertFalse(model.screen.staffPresent)
        assertTrue(ui.activity.window.attributes.flags and WindowManager.LayoutParams.FLAG_SECURE!=0)
        // Activity obscured-touch refusal is exercised with a real flagged event.
        ui.runOnUiThread {
            val event=MotionEvent.obtain(0,0,MotionEvent.ACTION_DOWN,1, arrayOf(MotionEvent.PointerProperties().apply {id=0}),arrayOf(MotionEvent.PointerCoords().apply {x=10f;y=10f}),0,0,1f,1f,0,0,0,MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)
            try {assertTrue(ui.activity.dispatchTouchEvent(event))}finally{event.recycle()}
        }
        click("Cancel");awaitState {model.screen.staffProposal==null};assertFalse(model.screen.staffPresent)
        model.staffInput(fixtures.getValue("staff"));awaitState {model.screen.staffProposal!=null}
        ui.activityRule.scenario.moveToState(Lifecycle.State.CREATED);ui.activityRule.scenario.moveToState(Lifecycle.State.RESUMED)
        awaitState {model.screen.staffProposal==null};assertFalse(model.screen.staffPresent)
        model.staffInput(fixtures.getValue("staff"));awaitState {shown("Import staff key")};click("Import staff key")
        awaitState {model.screen.staffReady && model.screen.staffProposal==null}
        model.staffInput(fixtures.getValue("expired"));awaitState {shown("Import staff key")};click("Import staff key")
        awaitState {model.screen.staffProposal==null && model.screen.error?.startsWith("Event authority")==true};assertTrue(model.screen.staffReady)
        model.beacon(true,false);awaitState {model.screen.error?.startsWith("Forget the staff key")==true};assertFalse(model.screen.beaconRequested)
        exchange()
        model.load();awaitState {model.screen.eventRows.any {it.staffLabel!=null}}
        assertTrue(model.screen.eventRows.any {it.pinned});screenshot("verified")
        val encrypted=File(context.noBackupFilesDir,"meshchat-staff-v1/identity.enc").readBytes()
        assertFalse(encrypted.toString(Charsets.UTF_8).contains("meshfest://staff"))
    }
    private fun reopen() {
        assertTrue(model.screen.staffReady);assertEquals(1,model.screen.events.size)
        model.select("#event updates");awaitState {model.screen.eventRows.any {it.staffLabel!=null}}
        assertTrue(model.screen.eventRows.any {it.pinned});screenshot("reopened")
    }
    private fun forget() {
        model.select("#event updates");awaitState {model.screen.selected?.name=="#event updates"}
        model.forgetStaff();awaitState {!model.screen.staffPresent};assertFalse(model.screen.staffReady)
        val key=model.screen.events.single().event.key
        model.removeEvent(key);awaitState {model.screen.events.isEmpty()}
        assertTrue(model.screen.eventRows.all {it.staffLabel==null && !it.pinned});screenshot("unverified")
        model.staffInput(fixtures.getValue("staff"));awaitState {shown("Import staff key")};click("Import staff key")
        awaitState {model.screen.staffProposal==null};assertFalse(model.screen.staffPresent)
    }
    private fun exchange() {
        val vault=StorageVault.android(context);val identity=IdentityProvider.android(context,vault)
        val storage=EncryptedStorage(identity,vault,StaffKeyVault.android(context))
        val wall=System.currentTimeMillis()/1000;var now=SystemClock.elapsedRealtime().toULong()
        val file=File.createTempFile("mc030-peer-",".db",context.cacheDir).also {check(it.delete())}
        try {IdentityKeySession.importUnlocked(ByteArray(64){30},ByteArray(16){30}).use {peer->
            CipherConnection.open(file,ByteArray(64){30},true).use {db->EncryptedStore.open(db,peer.publicIdentity().generation,true,wall).use {store->
                NativeTransport(store,peer.publicIdentity(),3030u,now).use {b->storage.transport(3031u,now).use {a->
                    b.confirmEvent(store,fixtures.getValue("event"),now,wall)
                    val al=a.nativeReady(a.admitConnection(ByteArray(16){1},now),TransportRole.CENTRAL,512u,512u,now).link
                    val bl=b.nativeReady(b.admitConnection(ByteArray(16){2},now),TransportRole.PERIPHERAL,512u,512u,now).link
                    val af=a.tick(now).sends.single();val bf=b.tick(now).sends.single()
                    b.receive(bl,af.bytes.size.toULong(),af.bytes,now);a.receive(al,bf.bytes.size.toULong(),bf.bytes,now)
                    a.complete(al,af.token,true,now);b.complete(bl,bf.token,true,now)
                    now+=1000u;val sent=storage.postEvent(a,"Stage","Synthetic official update",1u,(wall+300).toUInt(),30u,now);assertTrue(sent.queued)
                    repeat(15) {now+=1000u
                        for(frame in a.tick(now).sends) {
                            storage.messageEgress(a,frame) {b.receive(bl,frame.bytes.size.toULong(),frame.bytes,now);true}
                            a.complete(al,frame.token,true,now)
                        }
                        b.organizerTick(store,null,now,wall)
                    }
                    val rows=b.eventMessages(store,wall,now);assertTrue(rows.any {it.staffLabel=="Ops" && it.pinned})
                    a.disconnect(al,now);b.disconnect(bl,now)
                }}
            }}
        }} finally {file.delete()}
    }
}
