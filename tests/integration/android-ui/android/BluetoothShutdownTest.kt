package org.meshchat.ui

import android.app.KeyguardManager
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothAdapter
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.view.WindowManager
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity

/** Explicit physical radio-toggle regression, preserving the existing profile.
 * Uses real Bluetooth/lock/storage checks and restores Bluetooth on exit.
 */
class BluetoothShutdownTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private val model get() = (ui.activity.application as MeshApplication).model

    private fun bluetooth(enabled: Boolean) {
        val output = InstrumentationRegistry.getInstrumentation().uiAutomation
            .executeShellCommand("svc bluetooth ${if (enabled) "enable" else "disable"}")
        ParcelFileDescriptor.AutoCloseInputStream(output).use { it.readBytes() }
    }

    @Test fun bluetoothOffIsNotAProtectedDataFailure() {
        val args = InstrumentationRegistry.getArguments()
        check(args.getString("physicalConnect") == "true" && args.getString("physicalBluetooth") == "true")
        assertFalse(ui.activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        val adapter = ui.activity.getSystemService(BluetoothManager::class.java).adapter
        assertTrue("Test requires Bluetooth initially enabled", adapter.isEnabled)
        ui.runOnUiThread {
            ui.activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            model.load()
        }
        try {
            ui.waitUntil(120000) { model.screen.onboarded && !model.screen.locked }
            val before = model.screen
            ui.runOnUiThread { model.startRadio(); model.select("#general") }
            ui.waitUntil(120000) { model.screen.status == "Searching for nearby people…" && model.screen.selected?.name == "#general" }
            bluetooth(false)
            ui.waitUntil(30000) {
                assertFalse("Bluetooth loss must not report protected data locked", model.screen.locked)
                // isEnabled is already false in TURNING_OFF; enabling there
                // races the platform shutdown and may be ignored.
                adapter.state == BluetoothAdapter.STATE_OFF && model.screen.status == "Turn on Bluetooth to connect"
            }
            InstrumentationRegistry.getInstrumentation().sendStatus(0,Bundle().apply {
                putString("stream","MC025 Bluetooth fully off before re-enable; state=${adapter.state}\n")
            })
            assertEquals(before.nickname, model.screen.nickname)
            assertEquals(before.avatar, model.screen.avatar)
            assertEquals(before.channels.map { it.name }, model.screen.channels.map { it.name })
            bluetooth(true)
            ui.waitUntil(30000) { adapter.isEnabled }
            ui.runOnUiThread { model.startRadio() }
            ui.waitUntil(120000) {
                assertFalse(model.screen.locked)
                model.screen.status == "Searching for nearby people…"
            }
        } finally {
            bluetooth(true)
            ui.runOnUiThread {
                model.stop()
                ui.activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }
    }
}
