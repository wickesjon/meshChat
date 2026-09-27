package org.meshchat.ui

import android.app.KeyguardManager
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

/** Opt-in native callback regression, isolated synthetic addresses; no profile reset. */
class GattPeripheralRetirementTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private fun field(owner: Any,name: String)=owner.javaClass.getDeclaredField(name).apply {isAccessible=true}
    @Test fun idlePeripheralCloseKeepsServerAndRetiresOldCallbacks() = exercise(null)
    @Test fun activeNotificationRequiresFreshServerEpoch() = exercise("notification")
    @Test fun fullRetirementSetRequiresFreshServerEpoch() = exercise("capacity")
    @Test fun lastPeripheralDisconnectRefreshesEmptyServer() = exercise("disconnected")
    @Test fun errorDisconnectKeepsRetirementDeadline() = exercise("error")
    @Test fun peripheralDisconnectPreservesCentralSetup() = exercise("central")

    @Suppress("UNCHECKED_CAST")
    private fun exercise(fallback: String?) {
        check(InstrumentationRegistry.getArguments().getString("physicalConnect")=="true")
        assertFalse(ui.activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        val model=(ui.activity.application as MeshApplication).model
        ui.runOnUiThread {ui.activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);model.load()}
        try {
            ui.waitUntil(120000) {model.screen.onboarded && !model.screen.locked}
            ui.runOnUiThread {model.startRadio()}
            ui.waitUntil(120000) {model.screen.status=="Searching for nearby people…"}
            val radio=field(model,"radio").get(model) as AndroidGattRadio
            val adapter=ui.activity.getSystemService(BluetoothManager::class.java).adapter
            synchronized(checkNotNull(field(radio,"gate").get(radio))) {
                val driver=field(radio,"driver").get(radio) as GattDriver
                assertTrue("Requires no real peer during isolated callback regression",driver.connections().isEmpty())
                val server=field(radio,"server").get(radio)
                assertNotNull(server)
                val epoch=field(radio,"serverEpoch").getLong(radio)
                val callback=radio.javaClass.getDeclaredMethod("serverCallbacks",Long::class.javaPrimitiveType)
                    .apply {isAccessible=true}.invoke(radio,epoch) as BluetoothGattServerCallback
                val first=adapter.getRemoteDevice("02:00:00:00:25:31")
                val second=adapter.getRemoteDevice("02:00:00:00:25:32")
                callback.onConnectionStateChange(first,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                callback.onConnectionStateChange(second,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                val peers=driver.connections();assertEquals(2,peers.size)
                val closed=peers.single {it.address==first.address}.id
                val retained=peers.single {it.address==second.address}.id
                // Inject only the callback-ownership/capacity edge, not a
                // successful native notification or changed admission budget.
                if (fallback=="notification") {
                    val queue=checkNotNull(field(radio,"notifications").get(radio))
                    field(queue,"active").set(queue,closed)
                }
                if (fallback=="capacity") {
                    val retired=field(radio,"refusedAddresses").get(radio) as MutableSet<String>
                    for (index in 1..6) retired.add("02:00:00:00:27:%02X".format(index))
                }
                if (fallback=="disconnected" || fallback=="error" || fallback=="central") {
                    callback.onConnectionStateChange(first,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_DISCONNECTED)
                } else driver.lost(closed)
                if (fallback=="notification" || fallback=="capacity") {
                    assertNotSame(server,field(radio,"server").get(radio))
                    assertTrue(field(radio,"serverEpoch").getLong(radio)>epoch)
                    assertTrue(driver.connections().isEmpty())
                    callback.onConnectionStateChange(first,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                    callback.onMtuChanged(second,517)
                    callback.onNotificationSent(second,BluetoothGatt.GATT_SUCCESS)
                    assertTrue("Old server callbacks cannot admit or advance a new epoch",driver.connections().isEmpty())
                    return
                }
                assertSame("Closing idle peripheral must not replace other peers' services",server,field(radio,"server").get(radio))
                assertEquals(epoch,field(radio,"serverEpoch").getLong(radio))
                assertEquals(listOf(retained),driver.connections().map {it.id})
                val retired=field(radio,"refusedAddresses").get(radio) as Set<*>
                assertTrue(first.address in retired)
                callback.onMtuChanged(first,517)
                callback.onNotificationSent(first,BluetoothGatt.GATT_SUCCESS)
                callback.onConnectionStateChange(first,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                assertEquals("Retired address cannot receive a new token in this epoch",listOf(retained),driver.connections().map {it.id})
                if (fallback=="disconnected" || fallback=="error" || fallback=="central") {
                    val central=if (fallback=="central") checkNotNull(driver.connect("02:00:00:00:25:33")) else null
                    callback.onConnectionStateChange(second,if (fallback=="error") BluetoothGatt.GATT_FAILURE else BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_DISCONNECTED)
                    if (fallback!="disconnected") {
                        assertSame("Error or surviving central setup must preserve epoch",server,field(radio,"server").get(radio))
                        assertEquals(epoch,field(radio,"serverEpoch").getLong(radio))
                        if (central!=null) {
                            assertEquals(listOf(central),driver.connections().map {it.id})
                            driver.lost(central)
                        }
                        callback.onConnectionStateChange(second,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_DISCONNECTED)
                        assertEquals("Unmatched later disconnect cannot refresh retirement",epoch,field(radio,"serverEpoch").getLong(radio))
                        assertTrue(driver.connections().isEmpty())
                        return
                    }
                    assertTrue("Confirmed last-peer disconnect must refresh the empty server",field(radio,"serverEpoch").getLong(radio)>epoch)
                    assertNotSame(server,field(radio,"server").get(radio))
                    assertTrue(driver.connections().isEmpty())
                    val freshEpoch=field(radio,"serverEpoch").getLong(radio)
                    callback.onConnectionStateChange(second,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                    callback.onMtuChanged(second,517)
                    callback.onConnectionStateChange(second,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_DISCONNECTED)
                    assertTrue("Old callbacks cannot create connections or refresh the new epoch",driver.connections().isEmpty())
                    assertEquals(freshEpoch,field(radio,"serverEpoch").getLong(radio))
                } else driver.lost(retained)
            }
        } finally {
            ui.runOnUiThread {model.stop();ui.activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}
            ui.waitUntil(30000){model.screen.status=="Nearby connection is off"}
        }
    }
}
