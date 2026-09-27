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

/** Opt-in real adapter/core teardown fixtures. Run each method in its own
 * process with the other endpoint stopped; never alter native time/credits. */
class GattReleaseConfirmationTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private fun field(owner:Any,name:String)=owner.javaClass.getDeclaredField(name).apply {isAccessible=true}
    @Test fun lastClientReleaseRefreshesAfterConfirmedDisconnect()=exercise("confirmed")
    @Test fun errorConsumesReleaseWithoutRefresh()=exercise("error")
    @Test fun unrelatedDisconnectCannotConsumeRelease()=exercise("unrelated")
    @Test fun newClientAttemptInvalidatesRelease()=exercise("new-client")
    @Test fun retiredReconnectInvalidatesRelease()=exercise("reconnected")
    @Test fun survivingSetupPreventsReleaseRefresh()=exercise("surviving")
    @Test fun oldEpochReleaseCannotRefreshNewServer()=exercise("old-epoch")

    private fun exercise(mode:String) {
        check(InstrumentationRegistry.getArguments().getString("physicalConnect")=="true")
        val activity=ui.activity
        assertFalse(activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        val model=(activity.application as MeshApplication).model
        val adapter=activity.getSystemService(BluetoothManager::class.java).adapter
        ui.runOnUiThread {activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);model.load()}
        try {
            ui.waitUntil(120000){model.screen.onboarded && !model.screen.locked}
            ui.runOnUiThread {model.startRadio()}
            ui.waitUntil(120000){model.screen.status=="Searching for nearby people…"}
            val radio=field(model,"radio").get(model) as AndroidGattRadio
            synchronized(checkNotNull(field(radio,"gate").get(radio))) {
                val driver=field(radio,"driver").get(radio) as GattDriver
                assertTrue("Other endpoint must be stopped",driver.connections().isEmpty())
                val server=checkNotNull(field(radio,"server").get(radio))
                val epoch=field(radio,"serverEpoch").getLong(radio)
                val callback=radio.javaClass.getDeclaredMethod("serverCallbacks",Long::class.javaPrimitiveType)
                    .apply {isAccessible=true}.invoke(radio,epoch) as BluetoothGattServerCallback
                val device=adapter.getRemoteDevice("02:00:00:00:30:01")
                val other=adapter.getRemoteDevice("02:00:00:00:30:02")
                val client=checkNotNull(driver.connect(device.address))
                callback.onConnectionStateChange(device,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                val peripheral=driver.connections().single {it.id!=client}.id
                driver.lost(peripheral)
                assertEquals(listOf(client),driver.connections().map {it.id})
                val retired=field(radio,"refusedAddresses").get(radio) as Set<*>
                assertTrue(device.address in retired)
                val survivor=if(mode=="surviving")checkNotNull(driver.connect(other.address)) else null
                driver.lost(client)
                assertSame("Client teardown alone cannot refresh an epoch",server,field(radio,"server").get(radio))
                assertEquals(epoch,field(radio,"serverEpoch").getLong(radio))
                when(mode) {
                    "error" -> callback.onConnectionStateChange(device,BluetoothGatt.GATT_FAILURE,BluetoothProfile.STATE_DISCONNECTED)
                    "unrelated" -> callback.onConnectionStateChange(other,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_DISCONNECTED)
                    "new-client" -> driver.lost(checkNotNull(driver.connect(other.address)))
                    "reconnected" -> callback.onConnectionStateChange(device,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                    "old-epoch" -> radio.javaClass.getDeclaredMethod("resetServer").apply {isAccessible=true}.invoke(radio)
                }
                if(mode!="old-epoch")assertEquals("Intervening callbacks alone cannot refresh",epoch,field(radio,"serverEpoch").getLong(radio))
                val beforeConfirm=field(radio,"serverEpoch").getLong(radio)
                if(mode=="old-epoch") {
                    assertTrue(beforeConfirm>epoch)
                    val fresh=radio.javaClass.getDeclaredMethod("serverCallbacks",Long::class.javaPrimitiveType)
                        .apply {isAccessible=true}.invoke(radio,beforeConfirm) as BluetoothGattServerCallback
                    fresh.onConnectionStateChange(device,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_DISCONNECTED)
                    assertEquals("A new epoch must not inherit a release target",beforeConfirm,field(radio,"serverEpoch").getLong(radio))
                }
                callback.onConnectionStateChange(device,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_DISCONNECTED)
                if(mode=="confirmed" || mode=="unrelated") {
                    assertTrue("Confirmed final client release must refresh the empty server",field(radio,"serverEpoch").getLong(radio)>epoch)
                    assertNotSame(server,field(radio,"server").get(radio))
                    assertTrue(retired.isEmpty())
                } else {
                    assertEquals("Invalidated/error/old release or surviving setup must not refresh",beforeConfirm,field(radio,"serverEpoch").getLong(radio))
                    if(survivor!=null) {
                        assertEquals(listOf(survivor),driver.connections().map {it.id})
                        driver.lost(survivor)
                    }
                }
                assertTrue(driver.connections().isEmpty())
                val finalEpoch=field(radio,"serverEpoch").getLong(radio)
                callback.onConnectionStateChange(device,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_DISCONNECTED)
                callback.onMtuChanged(device,517)
                callback.onNotificationSent(device,BluetoothGatt.GATT_SUCCESS)
                assertTrue("Late callbacks cannot admit another token",driver.connections().isEmpty())
                assertEquals("Confirmation is consumed once",finalEpoch,field(radio,"serverEpoch").getLong(radio))
            }
        } finally {
            ui.runOnUiThread {model.stop();activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}
            ui.waitUntil(30000){model.screen.status=="Nearby connection is off"}
        }
    }
}
