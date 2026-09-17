package org.meshchat.ui

import android.content.ClipboardManager
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.net.Uri
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import com.google.zxing.*
import com.google.zxing.common.HybridBinarizer
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity
import uniffi.meshchat_core.*
import java.io.File

/** Real protected app, native parser and offline QR backend. Synthetic emulator
 * evidence does not certify camera optics or OS website association. */
class SharingUiTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private val model get()=(ui.activity.application as MeshApplication).model
    private val context get()=InstrumentationRegistry.getInstrumentation().targetContext
    private fun awaitState(condition:()->Boolean) = ui.waitUntil(120000) {
        check(!model.screen.locked) {"Protected app unavailable"};condition()
    }
    private fun shown(text:String)=ui.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()
    private fun click(text:String) {
        val node=ui.onNodeWithText(text)
        runCatching {node.assertIsDisplayed()}.onFailure {node.performScrollTo()}
        node.performClick()
    }
    private fun decode(bitmap:Bitmap):String {
        val width=bitmap.width;val height=bitmap.height
        val pixels=IntArray(width*height);bitmap.getPixels(pixels,0,width,0,0,width,height)
        // Four modules plus centering padding stay opaque white in exported PNGs.
        for(i in 0 until width)for(j in 0 until 16) {
            assertEquals(Color.WHITE,bitmap.getPixel(i,j));assertEquals(Color.WHITE,bitmap.getPixel(i,height-1-j))
            assertEquals(Color.WHITE,bitmap.getPixel(j,i));assertEquals(Color.WHITE,bitmap.getPixel(width-1-j,i))
        }
        return MultiFormatReader().decode(BinaryBitmap(HybridBinarizer(RGBLuminanceSource(width,height,pixels)))).text
    }
    private fun screenshot(name:String) {
        ui.waitForIdle()
        val automation=InstrumentationRegistry.getInstrumentation().uiAutomation
        automation.waitForIdle(500,5000)
        val bitmap=checkNotNull(automation.takeScreenshot())
        File(context.filesDir,"mc031-$name.png").outputStream().use {bitmap.compress(Bitmap.CompressFormat.PNG,100,it)}
    }
    private fun incoming(uri:String) {
        val activity=ui.activity;val original=activity.intent
        val intent=Intent(Intent.ACTION_VIEW,Uri.parse(uri)).setPackage(context.packageName)
            .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        assertNotNull(intent.resolveActivity(context.packageManager))
        ui.runOnUiThread {activity.startActivity(intent)}
        awaitState {model.screen.channelProposal!=null || model.screen.proposal!=null || model.screen.error!=null}
        InstrumentationRegistry.getInstrumentation().runOnMainSync {activity.intent=original}
    }
    @Test fun offlineShareAcceptance() {
        awaitState {model.screen.onboarded}
        when(InstrumentationRegistry.getArguments().getString("phase")) {
            "share" -> share()
            "reopen" -> {
                assertTrue(model.screen.channels.any {it.name=="hard|house|cave"})
                assertNull(model.screen.channelProposal);assertNull(model.screen.proposal)
            }
            else -> error("Explicit sharing phase required")
        }
    }
    private fun share() {
        val count=model.screen.channels.size
        val uri="meshfest://j/hard-house-cave"
        incoming(uri)
        awaitState {shown("Join this channel?")}
        assertEquals(count,model.screen.channels.size)
        assertEquals("hard|house|cave",model.screen.channelProposal?.name)
        click("Cancel");awaitState {model.screen.channelProposal==null}
        assertEquals(count,model.screen.channels.size)
        model.shareInput(decode(Sharing.qr(uri)),true)
        awaitState {shown("Join this channel?")}
        screenshot("channel-confirmation")
        click("Join channel");awaitState {model.screen.selected?.name=="hard|house|cave"}
        assertEquals(count+1,model.screen.channels.size)
        assertArrayEquals(channelInfo("hard|house|cave").id,model.screen.selected!!.id)
        click("Info");click("Share channel");awaitState {shown("Share as QR")}
        screenshot("channel-qr")
        click("Copy link")
        ui.runOnIdle {
            val clip=checkNotNull(context.getSystemService(ClipboardManager::class.java).primaryClip)
            assertEquals(Sharing.https(uri),clip.getItemAt(0).text.toString())
            assertTrue(clip.description.extras!!.getBoolean("android.content.extra.IS_SENSITIVE"))
        }
        screenshot("channel-actions");click("Done")
        val export=Sharing.qrIntent(context,uri)
        assertEquals("image/png",export.type)
        assertEquals(Intent.FLAG_GRANT_READ_URI_PERMISSION,export.flags and Intent.FLAG_GRANT_READ_URI_PERMISSION)
        assertEquals(0,export.flags and Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        val content=checkNotNull(export.clipData).getItemAt(0).uri
        assertEquals("content",content.scheme)
        context.contentResolver.openInputStream(content).use {assertEquals(uri,decode(BitmapFactory.decodeStream(it)))}
        assertTrue(Sharing.linkIntent(uri).getStringExtra(Intent.EXTRA_TEXT)!!.contains("not verified yet"))
        val another=Sharing.qrIntent(context,uri)
        assertNotEquals(content,checkNotNull(another.clipData).getItemAt(0).uri)
        // HTTPS reaches the same confirmation, without promising OS association.
        incoming(Sharing.https(uri));awaitState {shown("Join this channel?")};click("Cancel")
        awaitState {model.screen.channelProposal==null}
        for(bad in listOf(uri+"?join=1",uri+"/",uri+"#x","meshfest://j/madeup-house-cave","meshfest://staff/x/y")) {
            model.shareInput(bad,true)
            awaitState {model.screen.error?.startsWith("Check the channel, friend or public event link")==true}
            assertNull(model.screen.channelProposal);assertNull(model.screen.proposal)
            assertEquals(count+1,model.screen.channels.size)
        }
        click("Back");awaitState {shown("Your channels")};click("Friends");click("My friend code")
        awaitState {shown("Your public friend code") || ui.onAllNodesWithContentDescription("Your public friend code").fetchSemanticsNodes().isNotEmpty()}
        screenshot("friend-qr");click("Done")
        val own=checkNotNull(model.screen.friendCode)
        assertEquals(own.uri,decode(Sharing.qr(own.uri)))
        val friendExport=Sharing.qrIntent(context,own.uri)
        context.contentResolver.openInputStream(checkNotNull(friendExport.clipData).getItemAt(0).uri).use {
            assertEquals(own.uri,decode(BitmapFactory.decodeStream(it)))
        }
        IdentityKeySession.importUnlocked(ByteArray(64){131.toByte()},ByteArray(16){31}).use {identity ->
            val friend=friendCode(identity.publicIdentity(),"Synthetic Bob")
            val pins=model.screen.friends.size
            model.shareInput(decode(Sharing.qr(friend.uri)),true)
            awaitState {model.screen.proposal!=null}
            assertArrayEquals(friend.keys,model.screen.proposal!!.keys);assertTrue(model.screen.proposalScanned)
            assertEquals(pins,model.screen.friends.size)
            click("Cancel");awaitState {model.screen.proposal==null}
            incoming(Sharing.https(friend.uri));awaitState {shown("Verify this friend")}
            assertFalse(model.screen.proposalScanned);assertArrayEquals(friend.keys,model.screen.proposal!!.keys)
            ui.onNodeWithText("Pin verified identity").assertIsNotEnabled()
            assertEquals(pins,model.screen.friends.size);click("Cancel")
        }
        val cache=File(context.cacheDir,"share-qr")
        val fixtures=mutableListOf<File>()
        try {
            while(checkNotNull(cache.listFiles()).size<32)fixtures.add(File.createTempFile("fixture-",".png",cache))
            assertThrows(IllegalStateException::class.java) {Sharing.qrIntent(context,uri)}
            fixtures.first().setLastModified(System.currentTimeMillis()-25*60*60*1000L)
            assertEquals("image/png",Sharing.qrIntent(context,uri).type)
        } finally {fixtures.forEach {it.delete()}}
    }
}
