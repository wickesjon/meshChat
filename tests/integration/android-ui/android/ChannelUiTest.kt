package org.meshchat.ui

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.activity.compose.setContent
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.test.platform.app.InstrumentationRegistry
import android.graphics.Bitmap
import android.os.SystemClock
import org.junit.Rule
import org.junit.Test
import org.junit.Assert.*
import org.meshchat.app.MainActivity
import org.meshchat.identity.IdentityProvider
import org.meshchat.storage.EncryptedStorage
import org.meshchat.storage.StorageVault
import uniffi.meshchat_core.*
import java.io.File

/** Synthetic app flows on a named emulator. Actual radios remain MC-025. */
class ChannelUiTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private fun shown(text: String) = ui.onAllNodesWithText(text).fetchSemanticsNodes().isNotEmpty()
    // CI may use software emulation; protected SQLCipher operations retain their
    // production KDF cost. This is a functional deadline, not a latency claim.
    private fun awaitState(condition: () -> Boolean) {
        ui.waitUntil(120000) {
            check(!(ui.activity.application as MeshApplication).model.screen.locked) {
                "Protected app entered its unavailable state"
            }
            condition()
        }
    }
    private fun waitFor(text: String) { awaitState { shown(text) } }
    private fun click(text: String) {
        val node=ui.onNodeWithText(text)
        try { node.assertIsDisplayed() } catch (_: AssertionError) { node.performScrollTo() }
        node.performClick()
    }
    private fun screenshot(name: String) {
        val bitmap=ui.onRoot().captureToImage().asAndroidBitmap()
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        File(context.filesDir,"mc028-$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG,100,it) }
    }
    @Test fun productionAppAcceptance() {
        val phase=InstrumentationRegistry.getArguments().getString("phase")
        try {
            when(phase) {
                "create" -> create()
                "reopen" -> reopen()
                "composer" -> composer()
                else -> error("Explicit isolated emulator fixture phase is required")
            }
        } catch (failure: Throwable) {
            runCatching { screenshot("failed-$phase") }
            throw failure
        }
    }
    private fun create() {
        waitFor("Nickname · 20 UTF-8 bytes")
        ui.onNodeWithText("Nickname · 20 UTF-8 bytes").performTextInput("Synthetic Alice")
        click("Continue");click("Continue");click("Create my profile")
        waitFor("Your channels")
        assertTrue(ui.onAllNodesWithContentDescription("open-lock").fetchSemanticsNodes().size >= 3)
        ui.onNodeWithText("#general").performClick()
        ui.onNode(hasSetTextAction()).performTextInput("🎉".repeat(71))
        ui.onNodeWithText("Send").assertIsNotEnabled()
        ui.onNode(hasSetTextAction()).performTextReplacement("Synthetic offline draft")
        ui.onNodeWithText("Send").assertIsEnabled().performClick()
        awaitState { ui.onAllNodesWithText("Not sent.",substring=true).fetchSemanticsNodes().isNotEmpty() }
        ui.runOnUiThread {
            val keyboard=ui.activity.getSystemService(android.content.Context.INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
            keyboard.hideSoftInputFromWindow(ui.activity.window.decorView.windowToken,0)
        }
        ui.waitForIdle()
        // The production protected store, generated channel codec and presentation
        // run here. Radio admission is separately exercised by the three-node test.
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        val vault=StorageVault.android(context);val identity=IdentityProvider.android(context,vault)
        val storage=EncryptedStorage(identity,vault);val info=identity.load()
        val now=SystemClock.elapsedRealtime().toULong()
        NativeChannels(info.identity,now).use { channels ->
            val bytes=channels.compose("#general","✓ Official Alice",0x23u,"<script>synthetic</script> https://example.org",1u,now)
            assertTrue(storage.channelAccept(channels,null,bytes,TransportIntake.UNVERIFIED,false,now))
            val message=storage.channelHistory(channels,"#general","Synthetic Alice").single()
            assertEquals("Alice",message.nickname)
            val reaction=channels.reaction("#general",message.id,3u,false,now)
            assertTrue(storage.channelAccept(channels,null,reaction,TransportIntake.UNVERIFIED,false,now))
        }
        waitFor("<script>synthetic</script> https://example.org")
        ui.onNodeWithText("Unverified").assertExists()
        screenshot("dark-chat")
        ui.onNodeWithText("Back").performClick()
        click("Join a word-triple channel");click("Join channel")
        awaitState { (ui.activity.application as MeshApplication).model.screen.selected?.private == true }
        ui.onNodeWithText("Back").performClick()
        ui.onNodeWithText("Settings").performClick()
        ui.onNode(isToggleable()).performScrollTo().performClick()
        click("Save")
        // The old channel list is already visible when the dialog closes; wait
        // for the asynchronous protected write and new UI snapshot instead.
        awaitState { (ui.activity.application as MeshApplication).model.screen.light }
        waitFor("Your channels");screenshot("light-channels")
        val snapshot=(ui.activity.application as MeshApplication).model.screen
        assertTrue(snapshot.light);assertEquals(4,snapshot.channels.size)
    }
    private fun reopen() {
        waitFor("Your channels")
        val snapshot=(ui.activity.application as MeshApplication).model.screen
        assertTrue(snapshot.light);assertEquals(4,snapshot.channels.size)
        ui.onNodeWithText("#general").performClick()
        waitFor("<script>synthetic</script> https://example.org")
        val rows=(ui.activity.application as MeshApplication).model.screen.rows
        assertEquals(1,rows.size);assertEquals(1u.toUShort(),rows.single().message.reactions[3])
        assertEquals("Alice",rows.single().message.nickname)
        screenshot("reopened-chat")
    }
    private fun composer() {
        waitFor("Your channels")
        IdentityKeySession.importUnlocked(ByteArray(64){126},ByteArray(16){126}).use { identity ->
            NativeChannels(identity.publicIdentity(),0u).use { channels ->
                repeat(5) { channels.compose("#general","Alice",1u,"budget",0u,0u) }
                val wait=mutableIntStateOf((channels.waitMs("#general",false,0u)/1000u).toInt())
                ui.runOnUiThread { ui.activity.setContent {
                    val density=LocalDensity.current.density
                    CompositionLocalProvider(LocalDensity provides Density(density,2f)) {
                        MaterialTheme { Surface { ChannelComposer(false,wait.intValue,0) {} } }
                    }
                } }
                ui.onNode(hasSetTextAction()).performTextInput("Synthetic draft")
                ui.onNodeWithText("Post again in 12s").assertIsDisplayed()
                ui.onNodeWithText("Send").assertIsNotEnabled().assertHeightIsAtLeast(48.dp)
                ui.runOnIdle { wait.intValue=(channels.waitMs("#general",false,12000u)/1000u).toInt() }
                ui.onNodeWithText("Send").assertIsEnabled()
                ui.onNode(hasSetTextAction()).performTextReplacement("🎉".repeat(70))
                ui.onNodeWithText("Send").assertIsEnabled()
                ui.onNodeWithText("0 bytes left").assertExists()
                ui.onNode(hasSetTextAction()).performTextInput("🎉")
                ui.onNodeWithText("Send").assertIsNotEnabled()
                screenshot("large-text-composer")
            }
        }
    }
}
