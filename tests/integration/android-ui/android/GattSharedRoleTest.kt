package org.meshchat.ui

import android.app.KeyguardManager
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattServer
import android.bluetooth.BluetoothGattServerCallback
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.os.ParcelFileDescriptor
import android.os.Process
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

/** Opt-in real adapter/API sequencing checks. Invoke each method separately so
 * fixtures do not share the native process admission budget. No data reset. */
class GattSharedRoleTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private fun field(owner:Any,name:String)=owner.javaClass.getDeclaredField(name).apply {isAccessible=true}
    private fun cancellations(address:String):Int {
        SystemClock.sleep(100)
        val descriptor=InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(
            "logcat -d --pid=${Process.myPid()} -v brief BluetoothGattServer:D *:S")
        // A global logcat tail can omit this process's entries under system
        // traffic. Filter by process first, then bound the consumed records.
        val lines=ParcelFileDescriptor.AutoCloseInputStream(descriptor).bufferedReader().use {it.lineSequence().take(257).toList()}
        assertTrue("Framework diagnostic exceeded its record bound",lines.size<=256)
        // Samsung redacts the address prefix. This isolated fixture uses only
        // synthetic devices with distinct suffixes; never emit those log lines.
        return lines.count {it.substringAfter("cancelConnection() - device: ","").trimEnd().takeLast(5)==address.takeLast(5)}
    }
    @Test fun retiredPeripheralWaitsForSameDeviceClient()=exercise("retired")
    @Test fun refusedPeripheralWaitsForSameDeviceClient()=exercise("refused")
    @Test fun unrelatedClientDoesNotDeferCancellation()=exercise("unrelated")
    @Test fun fullRetirementDoesNotCreateUntrackedDeferral()=exercise("full")

    @Suppress("UNCHECKED_CAST")
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
                assertTrue("Other physical endpoint must be stopped",driver.connections().isEmpty())
                val server=field(radio,"server").get(radio) as BluetoothGattServer
                val epoch=field(radio,"serverEpoch").getLong(radio)
                val callback=radio.javaClass.getDeclaredMethod("serverCallbacks",Long::class.javaPrimitiveType)
                    .apply {isAccessible=true}.invoke(radio,epoch) as BluetoothGattServerCallback
                val marker=adapter.getRemoteDevice("02:00:00:00:28:01")
                server.cancelConnection(marker)
                assertEquals("Framework API logging must be available for this diagnostic",1,cancellations(marker.address))
                val device=adapter.getRemoteDevice("02:00:00:00:28:02")
                val address=if(mode=="unrelated") "02:00:00:00:28:03" else device.address
                val client=checkNotNull(driver.connect(address))
                val retired=field(radio,"refusedAddresses").get(radio) as MutableSet<String>
                if(mode=="full") for(index in 1..6)retired.add("02:00:00:00:29:%02X".format(index))
                val limit=field(radio,"linkLimit")
                val oldLimit=limit.getInt(radio)
                try {
                    if(mode=="refused")limit.setInt(radio,0)
                    callback.onConnectionStateChange(device,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                } finally {limit.setInt(radio,oldLimit)}
                if(mode=="retired" || mode=="unrelated") {
                    val peripheral=driver.connections().single {it.id!=client}.id
                    driver.lost(peripheral)
                }
                assertEquals(listOf(client),driver.connections().map {it.id})
                if(mode=="unrelated" || mode=="full") {
                    assertEquals("Unrelated/full-table cancellation must remain immediate",1,cancellations(device.address))
                    if(mode=="full")assertFalse(device.address in retired)
                } else {
                    assertTrue(device.address in retired)
                    assertEquals("Retiring one role must not cancel the same-device client",0,cancellations(device.address))
                    repeat(3){callback.onConnectionStateChange(device,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)}
                    callback.onMtuChanged(device,517)
                    callback.onNotificationSent(device,BluetoothGatt.GATT_SUCCESS)
                    assertEquals(listOf(client),driver.connections().map {it.id})
                    assertEquals("Retired callback cannot cancel the surviving client",0,cancellations(device.address))
                }
                assertSame(server,field(radio,"server").get(radio))
                assertEquals(epoch,field(radio,"serverEpoch").getLong(radio))
                driver.lost(client)
                assertTrue(driver.connections().isEmpty())
                if(mode=="retired" || mode=="refused") {
                    assertEquals("Last client releases the deferred server connection",1,cancellations(device.address))
                    callback.onConnectionStateChange(device,BluetoothGatt.GATT_SUCCESS,BluetoothProfile.STATE_CONNECTED)
                    assertTrue(driver.connections().isEmpty())
                    assertEquals("Without a client, retired reconnects are cancelled",2,cancellations(device.address))
                }
            }
        } finally {
            ui.runOnUiThread {model.stop();activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}
            ui.waitUntil(30000){model.screen.status=="Nearby connection is off"}
        }
    }
}
