package org.meshchat.ui
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity
import uniffi.meshchat_core.*
import java.io.File
class StatsUiTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private val model get()=(ui.activity.application as MeshApplication).model
    private val context get()=InstrumentationRegistry.getInstrumentation().targetContext
    private fun awaitState(check:()->Boolean)=ui.waitUntil(120000) {assertFalse(model.screen.locked);check()}
    private fun click(text:String) {val n=ui.onNodeWithText(text);runCatching {n.assertIsDisplayed()}.onFailure {n.performScrollTo()};n.performClick()}
    private fun screenshot(name:String) {
        ui.mainClock.advanceTimeBy(1000);ui.waitForIdle();val automation=InstrumentationRegistry.getInstrumentation().uiAutomation;automation.waitForIdle(500,5000);val bitmap=checkNotNull(automation.takeScreenshot())
        File(context.filesDir,"mc042-$name.png").outputStream().use {bitmap.compress(Bitmap.CompressFormat.PNG,100,it)}
    }
    @Test fun localPanelAndExplicitAggregateImageExport() {
        awaitState {model.screen.onboarded && model.screen.contribution!=null}
        model.select(null);awaitState {model.screen.selected==null};click("Settings");click("Contribution & power")
        ui.onNodeWithText("Private on this device. No telemetry or leaderboard.").assertExists()
        ui.onNodeWithText("Device battery reading unavailable or older than one minute.").assertExists()
        screenshot("panel")
        click("Preview contribution card");ui.onNodeWithText("Share image").assertIsNotEnabled()
        ui.onNodeWithContentDescription("Share received activity").performClick()
        ui.onNodeWithText("Share image").assertIsEnabled()
        ui.onNodeWithText("MeshChat · local contribution",substring=true).assertExists();screenshot("preview")
        click("Cancel");click("Reset local counters");click("Reset counters")
        awaitState {model.screen.contribution!!.elapsedMs<5000uL};assertEquals(0uL,model.screen.contribution!!.completedFrames)
        click("Done")
        // Fixed aggregate snapshot exercises actual image/URI export, without launching any share recipient.
        val stats=model.screen.contribution!!.copy(elapsedMs=120000u,receivedFrames=999u,receivedPackets=12u,completedFrames=888u,relayedChatCopies=777u,powerMode="PRIVATE_SENTINEL")
        val text=contributionShareText(stats,true,false,true)
        assertTrue(text.contains("999"));assertTrue(text.contains("777"));assertFalse(text.contains("888"));assertFalse(text.contains("PRIVATE_SENTINEL"))
        val intent=ContributionExport.intent(context,stats,true,false,true)
        assertEquals("image/png",intent.type);assertEquals(Intent.FLAG_GRANT_READ_URI_PERMISSION,intent.flags and Intent.FLAG_GRANT_READ_URI_PERMISSION)
        assertEquals(0,intent.flags and Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        val uri=checkNotNull(intent.clipData).getItemAt(0).uri
        context.contentResolver.openInputStream(uri).use {input->
            val bitmap=checkNotNull(BitmapFactory.decodeStream(input));assertEquals(1080,bitmap.width);assertTrue(bitmap.height>300)
            File(context.filesDir,"mc042-card.png").outputStream().use {bitmap.compress(Bitmap.CompressFormat.PNG,100,it)}
            bitmap.recycle()
        }
        assertThrows(IllegalArgumentException::class.java) {ContributionExport.image(stats,false,false,false)}
        val directory=File(context.cacheDir,"share-stats");val fixtures=mutableListOf<File>()
        try {
            while(checkNotNull(directory.listFiles()).size<8)fixtures.add(File.createTempFile("fixture-",".png",directory))
            assertThrows(IllegalStateException::class.java) {ContributionExport.intent(context,stats,true,false,false)}
            fixtures.first().setLastModified(System.currentTimeMillis()-25*60*60*1000L)
            assertEquals("image/png",ContributionExport.intent(context,stats,false,false,true).type)
        }finally{fixtures.forEach {it.delete()}}
    }
}
