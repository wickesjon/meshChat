package org.meshchat.ui

import android.graphics.Bitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity
import java.io.File

class BeaconUiTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private val model get()=(ui.activity.application as MeshApplication).model
    private fun waitFor(check:()->Boolean) {ui.waitUntil(120000) {assertFalse(model.screen.locked);check()}}
    @Test fun protectedPreferenceRestartAndAccessibleExit() {
        waitFor {model.screen.onboarded}
        val phase=InstrumentationRegistry.getArguments().getString("phase") ?: "enable"
        if(phase=="enable") {
            ui.runOnUiThread {model.beacon(true,true)}
            waitFor {model.screen.beaconRequested && model.screen.autoBeacon}
        } else {
            assertTrue("Manual mode must reopen from encrypted settings",model.screen.beaconRequested)
            assertTrue(model.screen.autoBeacon)
        }
        // This emulator may have no Bluetooth radio. Activation must still
        // attempt the real service and report its actual platform limitation.
        waitFor {model.screen.status!="Nearby connection is off"}
        ui.onNodeWithText("Hold 2 seconds to exit").performScrollTo().assertIsDisplayed()
        ui.onNodeWithText("Hold 2 seconds to exit").performTouchInput {click()}
        assertTrue("A pocket tap must not exit",model.screen.beaconRequested)
        assertEquals(0.06f,ui.activity.window.attributes.screenBrightness)
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        File(context.filesDir,"mc033-beacon.png").outputStream().use {
            ui.onRoot().captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG,100,it)
        }
        if(phase=="reopen-exit") {
            ui.onNodeWithText("Hold 2 seconds to exit").performSemanticsAction(SemanticsActions.OnLongClick)
            waitFor {!model.screen.beaconRequested && !model.screen.autoBeacon}
            ui.waitForIdle()
            assertEquals(-1f,ui.activity.window.attributes.screenBrightness)
        }
    }
}
