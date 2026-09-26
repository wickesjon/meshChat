package org.meshchat.ui

import android.app.KeyguardManager
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattServerCallback
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.view.WindowManager
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity
import org.meshchat.transport.AndroidGattRadio
import org.meshchat.transport.GattDriver

/** Real Android adapter, with explicitly injected connection-refusal callbacks.
 * No keys, history, permissions or platform lock checks change; native admission
 * uses its real budgets without resets or refunds.
 */
class GattServerRejectionTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private val model get() = (ui.activity.application as MeshApplication).model
    private fun field(owner: Any, name: String) = owner.javaClass.getDeclaredField(name).apply { isAccessible = true }

    @Test fun refusedConnectionsRetireAddressesWithoutReplacingAnEmptyServer() {
        check(InstrumentationRegistry.getArguments().getString("physicalConnect") == "true")
        assertFalse(ui.activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        ui.runOnUiThread {
            ui.activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            model.load()
        }
        try {
            ui.waitUntil(120000) { model.screen.onboarded && !model.screen.locked }
            ui.runOnUiThread { model.startRadio() }
            ui.waitUntil(120000) {
                assertFalse(model.screen.locked)
                model.screen.status == "Searching for nearby people…"
            }
            val radio = field(model, "radio").get(model) as AndroidGattRadio
            val gate = checkNotNull(field(radio, "gate").get(radio))
            // Compose's activity accessor waits for the UI thread. Resolve it
            // before holding the gate also used by the main-thread radio tick.
            val adapter = ui.activity.getSystemService(BluetoothManager::class.java).adapter
            synchronized(gate) {
                val driver = field(radio, "driver").get(radio) as GattDriver
                assertTrue("Run with the other test endpoint stopped", driver.connections().isEmpty())
                val server = field(radio, "server").get(radio)
                assertNotNull(server)
                val epoch = field(radio, "serverEpoch").getLong(radio)
                val method = radio.javaClass.getDeclaredMethod("serverCallbacks", java.lang.Long.TYPE).apply { isAccessible = true }
                val callback = method.invoke(radio, epoch) as BluetoothGattServerCallback
                val limit = field(radio, "linkLimit")
                val originalLimit = limit.getInt(radio)
                val peers = (1..7).map { adapter.getRemoteDevice("02:00:00:00:25:%02X".format(it)) }
                try {
                    // Force the real selection path to refuse before any native
                    // reservation. The original bug replaces the empty server.
                    limit.setInt(radio, 0)
                    for (peer in peers) {
                        callback.onConnectionStateChange(peer, BluetoothGatt.GATT_SUCCESS, BluetoothProfile.STATE_CONNECTED)
                        assertEquals("Refusal must not create a server restart loop", epoch, field(radio, "serverEpoch").getLong(radio))
                        assertSame(server, field(radio, "server").get(radio))
                    }
                    val retired = field(radio, "refusedAddresses").get(radio) as Set<*>
                    assertEquals("Refused address memory remains bounded", 6, retired.size)
                    limit.setInt(radio, originalLimit)
                    // Retirement is not cleared by disconnect, late MTU, or
                    // repeated CONNECTED events within the same callback epoch.
                    for (peer: BluetoothDevice in peers) {
                        callback.onConnectionStateChange(peer, BluetoothGatt.GATT_SUCCESS, BluetoothProfile.STATE_DISCONNECTED)
                        callback.onMtuChanged(peer, 517)
                        repeat(3) { callback.onConnectionStateChange(peer, BluetoothGatt.GATT_SUCCESS, BluetoothProfile.STATE_CONNECTED) }
                    }
                    assertEquals(epoch, field(radio, "serverEpoch").getLong(radio))
                    assertTrue(driver.connections().isEmpty())
                    assertEquals(6, retired.size)
                    val retry = radio.javaClass.getDeclaredMethod("retryRefusedServer").apply { isAccessible = true }
                    retry.invoke(radio)
                    assertEquals("Retry must wait for its deadline", epoch, field(radio, "serverEpoch").getLong(radio))
                    // Expire only the adapter's retry deadline; the native clock
                    // and admission accounting stay real and monotonic.
                    field(radio, "refusedRetryAt").setLong(radio, 0L)
                    val setup = checkNotNull(driver.incoming("02:00:00:00:26:01"))
                    retry.invoke(radio)
                    assertEquals("Even a setup link defers server replacement", epoch, field(radio, "serverEpoch").getLong(radio))
                    driver.lost(setup)
                    retry.invoke(radio)
                    val freshEpoch = field(radio, "serverEpoch").getLong(radio)
                    assertTrue("An isolated node can retry in a fresh epoch", freshEpoch > epoch)
                    assertNotNull(field(radio, "server").get(radio))
                    assertNotSame(server, field(radio, "server").get(radio))
                    assertTrue(retired.isEmpty())
                    callback.onConnectionStateChange(peers.first(), BluetoothGatt.GATT_SUCCESS, BluetoothProfile.STATE_CONNECTED)
                    callback.onMtuChanged(peers.first(), 517)
                    assertTrue("Retired server callbacks cannot admit work", driver.connections().isEmpty())
                    assertEquals(freshEpoch, field(radio, "serverEpoch").getLong(radio))
                } finally { limit.setInt(radio, originalLimit) }
            }
        } finally {
            ui.runOnUiThread {
                model.stop()
                ui.activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }
    }
}
