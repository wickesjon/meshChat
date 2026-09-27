package org.meshchat.ui

import android.app.KeyguardManager
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattServerCallback
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.os.SystemClock
import android.view.WindowManager
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity
import org.meshchat.transport.AndroidGattRadio
import org.meshchat.transport.GattDriver

/** Opt-in callback fixture on the real adapter/native admission. Run each method
 * in its own invocation, with the other endpoint off; never reset native budgets.
 * Synthetic callbacks are not evidence of actual MTU negotiation over the air. */
class GattMtuObservationTest {
    @get:Rule val ui = createAndroidComposeRule<MainActivity>()
    private fun field(owner: Any, name: String) = owner.javaClass.getDeclaredField(name).apply { isAccessible = true }

    @Test fun currentRequestedObservationReachesExistingPeripheral() = exercise("current")
    @Test fun unsolicitedObservationCannotSupplyPeripheralCapacity() = exercise("unsolicited")
    @Test fun failedObservationCannotSupplyPeripheralCapacity() = exercise("failed")
    @Test fun belowFloorObservationStillRefusesPeripheral() = exercise("small")
    @Test fun differentAddressCannotSupplyPeripheralCapacity() = exercise("address")
    @Test fun retiredClientCannotSupplyPeripheralCapacity() = exercise("client")
    @Test fun retiredPeripheralCannotReceiveObservation() = exercise("retired")
    @Test fun replacedServerCannotReceiveOldRequestObservation() = exercise("epoch")
    @Test fun secondRequestCannotRebindOldObservation() = exercise("rerequest")
    @Test fun laterPeripheralCannotReceiveEarlierObservation() = exercise("late")

    private fun exercise(mode: String) {
        check(InstrumentationRegistry.getArguments().getString("physicalConnect") == "true")
        val activity = ui.activity
        assertFalse(activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        val model = (activity.application as MeshApplication).model
        val adapter = activity.getSystemService(BluetoothManager::class.java).adapter
        ui.runOnUiThread { activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON); model.load() }
        try {
            ui.waitUntil(120000) { model.screen.onboarded && !model.screen.locked }
            ui.runOnUiThread { model.startRadio() }
            ui.waitUntil(120000) { model.screen.status == "Searching for nearby people…" }
            val radio = field(model, "radio").get(model) as AndroidGattRadio
            val gate = checkNotNull(field(radio, "gate").get(radio))
            val driver = field(radio, "driver").get(radio) as GattDriver
            val device = adapter.getRemoteDevice("02:00:00:00:28:01")
            fun serverCallback(): BluetoothGattServerCallback = radio.javaClass
                .getDeclaredMethod("serverCallbacks", java.lang.Long.TYPE).apply { isAccessible = true }
                .invoke(radio, field(radio, "serverEpoch").getLong(radio)) as BluetoothGattServerCallback
            var peripheral = 0L
            val central = synchronized(gate) {
                assertTrue("Other endpoint must be stopped", driver.connections().isEmpty())
                if (mode != "late") {
                    serverCallback().onConnectionStateChange(device, BluetoothGatt.GATT_SUCCESS, BluetoothProfile.STATE_CONNECTED)
                    peripheral = driver.connections().single().id
                }
                checkNotNull(driver.connect(if (mode == "address") "02:00:00:00:28:02" else device.address))
            }
            // Framework client registration is asynchronous even for a synthetic
            // peer; no real connection success or MTU callback is assumed here.
            SystemClock.sleep(500)
            synchronized(gate) {
                val client = checkNotNull((field(radio, "clients").get(radio) as Map<*, *>)[central])
                val gatt = field(client, "gatt").get(client) as BluetoothGatt
                val callback = radio.javaClass.getDeclaredMethod("clientCallbacks", java.lang.Long.TYPE)
                    .apply { isAccessible = true }.invoke(radio, central) as BluetoothGattCallback
                if (mode != "unsolicited") assertTrue("Framework must accept request dispatch", radio.requestMtu(central))
                when (mode) {
                    "late" -> {
                        serverCallback().onConnectionStateChange(device, BluetoothGatt.GATT_SUCCESS, BluetoothProfile.STATE_CONNECTED)
                        peripheral = driver.connections().single { it.id != central }.id
                    }
                    "client" -> driver.lost(central)
                    "retired" -> driver.lost(peripheral)
                    "epoch", "rerequest" -> {
                        radio.javaClass.getDeclaredMethod("resetServer").apply { isAccessible = true }.invoke(radio)
                        serverCallback().onConnectionStateChange(device, BluetoothGatt.GATT_SUCCESS, BluetoothProfile.STATE_CONNECTED)
                        peripheral = driver.connections().single { it.id != central }.id
                        if (mode == "rerequest") assertFalse("One request per client generation", radio.requestMtu(central))
                    }
                }
                callback.onMtuChanged(gatt, if (mode == "small") 23 else 517,
                    if (mode == "failed") BluetoothGatt.GATT_FAILURE else BluetoothGatt.GATT_SUCCESS)
                val peers = field(driver, "peers").get(driver) as Map<*, *>
                if (mode == "retired" || mode == "small") {
                    assertNull("Closed/below-floor peripheral cannot become ready", peers[peripheral])
                } else {
                    val peer = checkNotNull(peers[peripheral])
                    assertEquals(if (mode == "current") 512 else 0, field(peer, "capacity").getInt(peer))
                    assertNull("MTU does not bypass CCCD readiness", field(peer, "link").get(peer))
                }
            }
        } finally {
            ui.runOnUiThread { model.stop(); activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON) }
            ui.waitUntil(30000) { model.screen.status == "Nearby connection is off" }
        }
    }
}
