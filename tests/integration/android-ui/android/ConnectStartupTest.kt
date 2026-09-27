package org.meshchat.ui

import android.app.KeyguardManager
import android.os.SystemClock
import android.view.WindowManager
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity

/** Explicit physical preflight on an already onboarded synthetic profile.
 * Does not reset data, grant permissions, or substitute a radio/protection port.
 */
class ConnectStartupTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private val model get() = (ui.activity.application as MeshApplication).model

    private fun awaitState(condition: () -> Boolean) {
        ui.waitUntil(120000) {
            assertFalse("Connect must not make protected data unavailable", model.screen.locked)
            condition()
        }
    }

    @Test fun realRadioStartupAndProtectedReadsPreserveProfile() {
        check(InstrumentationRegistry.getArguments().getString("physicalConnect") == "true") {
            "Requires explicit physicalConnect=true and an existing synthetic test profile"
        }
        assertFalse(ui.activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        ui.runOnUiThread {
            ui.activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            model.load()
        }
        // Reopen may begin from a previous failed attempt. Require successful
        // protected loading before the regression's no-unavailable assertion.
        ui.waitUntil(120000) { model.screen.onboarded && !model.screen.locked }
        val before = model.screen
        try {
            repeat(3) {
                ui.runOnUiThread { model.startRadio() }
                awaitState { model.screen.status == "Searching for nearby people…" }
                // Exercise protected history/friend reads alongside real radio
                // ticks, including the asynchronous service startup handoff.
                ui.runOnUiThread { model.select("#general") }
                awaitState { model.screen.selected?.name == "#general" }
                val until = SystemClock.elapsedRealtime() + 10000
                while (SystemClock.elapsedRealtime() < until) {
                    ui.runOnUiThread { model.load() }
                    SystemClock.sleep(500)
                    assertFalse("Radio tick or refresh invalidated the profile", model.screen.locked)
                }
                assertEquals(before.nickname, model.screen.nickname)
                assertEquals(before.avatar, model.screen.avatar)
                assertEquals(before.channels.map { it.name }, model.screen.channels.map { it.name })
                before.channels.zip(model.screen.channels).forEach { (expected, actual) ->
                    assertArrayEquals(expected.id, actual.id)
                }
                ui.runOnUiThread { model.stop() }
                awaitState { model.screen.status == "Nearby connection is off" }
                ui.waitForIdle()
            }
        } finally {
            ui.runOnUiThread {
                model.stop()
                model.select(before.selected?.name)
                ui.activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }
    }
}
