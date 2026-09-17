package org.meshchat.transport

import android.Manifest
import android.annotation.SuppressLint
import android.app.KeyguardManager
import android.app.ActivityManager
import android.content.Intent
import android.content.IntentFilter
import android.os.BatteryManager
import android.location.LocationManager
import android.bluetooth.*
import android.bluetooth.le.*
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.ParcelUuid
import android.os.SystemClock
import java.util.UUID
import uniffi.meshchat_core.*

object MeshGatt {
    val SERVICE: UUID = UUID.fromString("c11b1d76-75da-4ae0-b0fe-cb273609c526")
    val TX: UUID = UUID.fromString("78db2e71-ff31-46e0-8a8e-371f8199bcdc")
    val RX: UUID = UUID.fromString("87bbc0ab-e60a-4802-b45f-445ed492cf30")
    val INFO: UUID = UUID.fromString("ac933506-2294-4d92-8a0c-58d9f23acfb3")
    val CCCD: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
    fun allowed(context: Context): Boolean {
        val permissions = if (Build.VERSION.SDK_INT >= 31) listOf(
            Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT,
            Manifest.permission.BLUETOOTH_ADVERTISE,
        ) else listOf(Manifest.permission.ACCESS_FINE_LOCATION)
        return permissions.all { context.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }
    }
}

/** Serialized native adapter. No message parsing or authenticated UI decisions.
 * Permission checks guard every entry; revocation races fail closed. MissingPermission
 * is suppressed only because Android lint cannot follow this shared guard.
 */
@SuppressLint("MissingPermission")
class AndroidGattRadio(
    private val context: Context,
    core: NativeTransport,
    event: (Long, TransportEvent) -> Unit,
    private val status: (RadioState) -> Unit = {},
    private val stopped: () -> Unit = {},
) : GattRadio {
    private class Client(val gatt: BluetoothGatt) {
        var tx: BluetoothGattCharacteristic? = null
        var rx: BluetoothGattCharacteristic? = null
    }
    private val gate = Any()
    private val handler = Handler(Looper.getMainLooper())
    private val manager = context.getSystemService(BluetoothManager::class.java)
    private val adapter = manager.adapter
    private val clock = { SystemClock.elapsedRealtime().toULong() }
    private val selection = ConnectionPolicy(clock)
    private val scanning = ScanPolicy(clock)
    private val driver = GattDriver(core, this, clock, event)
    private var setting: TransportPowerSetting? = TransportPowerSetting.AUTO
    private var nextPolicy = 0uL
    private var scanMode = ScanMode.OFF
    private var state = RadioState.STOPPED
    private var foreground = true
    private var saver = false
    private var linkLimit = 6
    private var powerStatus: PowerStatus? = null
    fun currentPower(): PowerStatus? = synchronized(gate) { powerStatus }
    data class PowerStatus(val saver: Boolean, val linkLimit: Int, val batteryTier: Int, val announceMs: ULong)
    private fun report(value: RadioState) { if (state != value) { state = value; status(value) } }
    fun currentState(): RadioState = synchronized(gate) { state }
    fun powerSetting(value: TransportPowerSetting) = guarded { setting = value; nextPolicy = 0uL; policy() }
    private val clients = mutableMapOf<Long, Client>()
    private val peripherals = mutableMapOf<Long, BluetoothDevice>()
    private var server: BluetoothGattServer? = null
    private var notify: BluetoothGattCharacteristic? = null
    private var serverEpoch = 0L
    private var advertiser: AdvertiseCallback? = null
    private var scanner: ScanCallback? = null
    private var running = false
    private val subscriptions = mutableSetOf<Long>()
    // Refused connections have no driver token, but Android can still deliver
    // their MTU callbacks. Never reuse such an address within this server epoch.
    private val refusedAddresses = mutableSetOf<String>()
    private val notifications = NotificationQueue(::submitNotification) { id, success -> driver.completed(id, success) }
    private val timer = object : Runnable {
        override fun run() {
            guarded { driver.tick(); if (clock() >= nextPolicy) policy() }
            synchronized(gate) { if (running) handler.postDelayed(this, 100) }
        }
    }

    fun start() = synchronized(gate) {
        if (running) return@synchronized
        running = true
        guarded { openServer(); if (running) { policy(); if (running) handler.post(timer) } }
    }
    fun send(id: Long, bytes: ByteArray, traffic: TransportTraffic, cookie: ULong): Boolean {
        var accepted = false
        guarded { accepted = driver.enqueue(id, bytes, traffic, cookie) }
        return accepted
    }
    fun proof(id: Long, provider: IdentityKeySession): Boolean {
        var accepted = false
        try { guarded { accepted = driver.proof(id, provider) }; return accepted } finally { provider.invalidate() }
    }
    fun stateChanged() = guarded { /* guard tears down on real permission/radio/lock loss */ }

    private fun guarded(work: () -> Unit): Unit = synchronized(gate) {
        if (!running) return@synchronized
        try {
            val unavailable = when {
                !MeshGatt.allowed(context) -> RadioState.PERMISSION_REQUIRED
                adapter?.isEnabled != true -> RadioState.RADIO_DISABLED
                context.getSystemService(KeyguardManager::class.java).isDeviceLocked -> RadioState.LOCKED
                else -> null
            }
            if (unavailable != null) { stop(unavailable); return@synchronized }
            work()
        } catch (_: SecurityException) { stop(RadioState.PERMISSION_REQUIRED) } catch (_: IllegalStateException) { stop(RadioState.STOPPED) }
    }
    private fun policy() {
        nextPolicy = clock() + 1_000uL
        val memory = ActivityManager.RunningAppProcessInfo()
        ActivityManager.getMyMemoryState(memory)
        foreground = memory.importance <= ActivityManager.RunningAppProcessInfo.IMPORTANCE_FOREGROUND
        val battery = context.registerReceiver(null, IntentFilter(Intent.ACTION_BATTERY_CHANGED))
            ?: run { stop(RadioState.STOPPED); return }
        val level = battery.getIntExtra(BatteryManager.EXTRA_LEVEL, -1)
        val scale = battery.getIntExtra(BatteryManager.EXTRA_SCALE, -1)
        if (level < 0 || scale <= 0 || level > scale) { stop(RadioState.STOPPED); return }
        val visible = selection.visible(driver.connections())
        val result = driver.power(setting, (level.toLong() * 100 / scale).toInt(),
            (battery.getIntExtra(BatteryManager.EXTRA_PLUGGED, 0)) != 0, visible)
            ?: run { stop(RadioState.STOPPED); return }
        setting = null
        saver = result.saver
        linkLimit = result.linkLimit.toInt()
        powerStatus = PowerStatus(saver, linkLimit, result.batteryTier.toInt(), result.announceMs)
        val proximity = context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED
        if (!proximity) selection.clearRssi()
        val restriction = discoveryRestriction(Build.VERSION.SDK_INT, foreground,
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED,
            context.getSystemService(LocationManager::class.java).isLocationEnabled,
            context.checkSelfPermission(Manifest.permission.ACCESS_BACKGROUND_LOCATION) == PackageManager.PERMISSION_GRANTED)
        val mode = scanning.mode(saver, visible, foreground, restriction == null)
        changeScan(mode)
        report(if (restriction != null) restriction
            else if (scanning.retrying || (mode != ScanMode.OFF && scanner == null)) RadioState.DISCOVERY_PAUSED else RadioState.ACTIVE)
        val plan = selection.plan(driver.connections(), linkLimit)
        plan.close.forEach { driver.lost(it) }
        plan.connect?.let { address ->
            // Teardown may replace the whole server, so recheck current slots.
            if (selection.allowed(address, driver.connections(), linkLimit)) {
                selection.attempted(address)
                driver.connect(address)
            }
        }
    }
    private fun changeScan(mode: ScanMode) {
        if (mode == scanMode && (mode == ScanMode.OFF || scanner != null)) return
        scanner?.let { adapter?.bluetoothLeScanner?.stopScan(it) }
        scanner = null
        scanMode = mode
        if (mode == ScanMode.OFF) return
        val native = adapter?.bluetoothLeScanner ?: run { scanning.failed(); report(RadioState.DISCOVERY_PAUSED); return }
        val listener = object : ScanCallback() {
            override fun onScanResult(callbackType: Int, result: ScanResult) = guarded {
                if (scanner === this) {
                    val proximity = context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION) == PackageManager.PERMISSION_GRANTED
                    scanning.discovered(); scanning.received()
                    selection.discovered(result.device.address, if (proximity) result.rssi else null)
                }
            }
            override fun onScanFailed(errorCode: Int) = guarded {
                if (scanner === this) { scanner = null; scanning.failed(); report(RadioState.DISCOVERY_PAUSED) }
            }
        }
        scanner = listener
        native.startScan(listOf(ScanFilter.Builder().setServiceUuid(ParcelUuid(MeshGatt.SERVICE)).build()),
            ScanSettings.Builder().setScanMode(if (mode == ScanMode.BURST) ScanSettings.SCAN_MODE_LOW_LATENCY else ScanSettings.SCAN_MODE_BALANCED).build(), listener)
    }
    override fun connect(id: Long, address: String): Boolean {
        val device = adapter?.getRemoteDevice(address) ?: return false
        val gatt = device.connectGatt(context, false, clientCallbacks(id), BluetoothDevice.TRANSPORT_LE) ?: return false
        clients[id] = Client(gatt)
        return true
    }
    private fun clientCallbacks(id: Long) = object : BluetoothGattCallback() {
        private fun current(gatt: BluetoothGatt, work: (Client) -> Unit) = guarded {
            clients[id]?.takeIf { it.gatt === gatt }?.let(work)
        }
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) = current(gatt) {
            if (status != BluetoothGatt.GATT_SUCCESS || newState == BluetoothProfile.STATE_DISCONNECTED) driver.lost(id)
            else if (newState == BluetoothProfile.STATE_CONNECTED) driver.connected(id, true)
        }
        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) = current(gatt) { client ->
            val service = gatt.getService(MeshGatt.SERVICE)
            client.tx = service?.getCharacteristic(MeshGatt.TX)
            client.rx = service?.getCharacteristic(MeshGatt.RX)
            val tx = client.tx
            val rx = client.rx
            val valid = status == BluetoothGatt.GATT_SUCCESS && tx != null && rx != null &&
                tx.properties and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0 &&
                rx.properties and BluetoothGattCharacteristic.PROPERTY_NOTIFY != 0 &&
                rx.getDescriptor(MeshGatt.CCCD) != null && service.getCharacteristic(MeshGatt.INFO) != null
            driver.services(id, valid)
        }
        override fun onMtuChanged(gatt: BluetoothGatt, mtu: Int, status: Int) = current(gatt) {
            driver.mtu(id, mtu, status == BluetoothGatt.GATT_SUCCESS)
        }
        override fun onDescriptorWrite(gatt: BluetoothGatt, descriptor: BluetoothGattDescriptor, status: Int) = current(gatt) {
            if (descriptor.uuid == MeshGatt.CCCD && descriptor.characteristic === it.rx)
                driver.subscribed(id, status == BluetoothGatt.GATT_SUCCESS)
        }
        override fun onCharacteristicWrite(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, status: Int) = current(gatt) {
            if (characteristic === it.tx) driver.completed(id, status == BluetoothGatt.GATT_SUCCESS)
        }
        override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, value: ByteArray) = current(gatt) {
            if (characteristic === it.rx) driver.value(id, value)
        }
        @Deprecated("Legacy callback required on API 29-32")
        override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic) {
            if (Build.VERSION.SDK_INT < 33) current(gatt) {
                @Suppress("DEPRECATION") val value = characteristic.value ?: byteArrayOf()
                if (characteristic === it.rx) driver.value(id, value)
            }
        }
    }
    override fun discover(id: Long): Boolean = clients[id]?.gatt?.discoverServices() == true
    override fun requestMtu(id: Long): Boolean = clients[id]?.gatt?.requestMtu(517) == true
    @Suppress("DEPRECATION")
    override fun subscribe(id: Long): Boolean {
        val client = clients[id] ?: return false
        val rx = client.rx ?: return false
        val descriptor = rx.getDescriptor(MeshGatt.CCCD) ?: return false
        if (!client.gatt.setCharacteristicNotification(rx, true)) return false
        return if (Build.VERSION.SDK_INT >= 33) client.gatt.writeDescriptor(descriptor, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE) == BluetoothStatusCodes.SUCCESS
        else { descriptor.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE; client.gatt.writeDescriptor(descriptor) }
    }
    private fun openServer() {
        val epoch = ++serverEpoch
        val service = BluetoothGattService(MeshGatt.SERVICE, BluetoothGattService.SERVICE_TYPE_PRIMARY)
        service.addCharacteristic(BluetoothGattCharacteristic(MeshGatt.TX, BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE, BluetoothGattCharacteristic.PERMISSION_WRITE))
        val rx = BluetoothGattCharacteristic(MeshGatt.RX, BluetoothGattCharacteristic.PROPERTY_NOTIFY, 0).apply {
            addDescriptor(BluetoothGattDescriptor(MeshGatt.CCCD, BluetoothGattDescriptor.PERMISSION_READ or BluetoothGattDescriptor.PERMISSION_WRITE))
        }
        notify = rx
        service.addCharacteristic(rx)
        service.addCharacteristic(BluetoothGattCharacteristic(MeshGatt.INFO, BluetoothGattCharacteristic.PROPERTY_READ, BluetoothGattCharacteristic.PERMISSION_READ))
        server = manager.openGattServer(context, serverCallbacks(epoch))
        if (server?.addService(service) != true) stop()
    }
    private fun serverCallbacks(epoch: Long) = object : BluetoothGattServerCallback() {
        private fun current(work: () -> Unit) = guarded { if (epoch == serverEpoch && server != null) work() }
        private fun id(device: BluetoothDevice): Long? = peripherals.entries.firstOrNull { it.value == device }?.key
        override fun onServiceAdded(status: Int, service: BluetoothGattService) = current {
            if (status != BluetoothGatt.GATT_SUCCESS || service.uuid != MeshGatt.SERVICE) { stop(); return@current }
            val native = adapter?.bluetoothLeAdvertiser ?: run { stop(RadioState.DISCOVERY_PAUSED); return@current }
            val listener = object : AdvertiseCallback() {
                override fun onStartFailure(errorCode: Int) = current { if (advertiser === this) stop(RadioState.DISCOVERY_PAUSED) }
            }
            advertiser = listener
            native.startAdvertising(AdvertiseSettings.Builder().setConnectable(true).setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_BALANCED).build(),
                AdvertiseData.Builder().addServiceUuid(ParcelUuid(MeshGatt.SERVICE)).setIncludeDeviceName(false).setIncludeTxPowerLevel(false).build(), listener)
        }
        override fun onConnectionStateChange(device: BluetoothDevice, status: Int, newState: Int) = current {
            val existing = id(device)
            if (status != BluetoothGatt.GATT_SUCCESS || newState == BluetoothProfile.STATE_DISCONNECTED) {
                if (existing != null) driver.lost(existing)
            } else if (newState == BluetoothProfile.STATE_CONNECTED) {
                if (existing != null) { resetServer(); return@current }
                if (device.address in refusedAddresses || refusedAddresses.size >= 6) {
                    server?.cancelConnection(device); return@current
                }
                val handle = if (selection.allowed(device.address, driver.connections(), linkLimit, inbound = true)) {
                    selection.attempted(device.address); driver.incoming(device.address)
                } else null
                if (handle == null) {
                    if (peripherals.isEmpty()) resetServer()
                    else { refusedAddresses.add(device.address); server?.cancelConnection(device) }
                }
                else { peripherals[handle] = device; driver.connected(handle, true) }
            }
        }
        override fun onMtuChanged(device: BluetoothDevice, mtu: Int) = current { id(device)?.let { driver.mtu(it, mtu, true) } }
        override fun onCharacteristicWriteRequest(device: BluetoothDevice, requestId: Int, characteristic: BluetoothGattCharacteristic,
            preparedWrite: Boolean, responseNeeded: Boolean, offset: Int, value: ByteArray) = current {
            val handle = id(device)
            val valid = handle != null && characteristic.uuid == MeshGatt.TX && !preparedWrite && offset == 0 && !responseNeeded
            if (responseNeeded) server?.sendResponse(device, requestId, BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED, offset, null)
            if (valid) driver.value(requireNotNull(handle), value)
        }
        override fun onCharacteristicReadRequest(device: BluetoothDevice, requestId: Int, offset: Int, characteristic: BluetoothGattCharacteristic) = current {
            val info = byteArrayOf(1, 2, 0)
            val valid = characteristic.uuid == MeshGatt.INFO && offset in 0..info.size
            server?.sendResponse(device, requestId, if (valid) BluetoothGatt.GATT_SUCCESS else BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED,
                offset, if (valid) info.copyOfRange(offset, info.size) else null)
        }
        override fun onDescriptorWriteRequest(device: BluetoothDevice, requestId: Int, descriptor: BluetoothGattDescriptor,
            preparedWrite: Boolean, responseNeeded: Boolean, offset: Int, value: ByteArray) = current {
            val handle = id(device)
            val enabled = value.contentEquals(BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE)
            val valid = handle != null && descriptor.characteristic === notify && descriptor.uuid == MeshGatt.CCCD &&
                !preparedWrite && offset == 0 && (enabled || value.contentEquals(BluetoothGattDescriptor.DISABLE_NOTIFICATION_VALUE))
            val respond = {
                responseNeeded && server?.sendResponse(device, requestId,
                    if (valid) BluetoothGatt.GATT_SUCCESS else BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED, offset, null) == true
            }
            if (handle == null) respond()
            else driver.subscriptionRequest(handle, valid && enabled, respond) { accepted ->
                if (accepted) subscriptions.add(handle) else subscriptions.remove(handle)
            }
        }
        override fun onDescriptorReadRequest(device: BluetoothDevice, requestId: Int, offset: Int, descriptor: BluetoothGattDescriptor) = current {
            val handle = id(device)
            val valid = handle != null && descriptor.characteristic === notify && descriptor.uuid == MeshGatt.CCCD && offset in 0..2
            val bytes = if (handle in subscriptions) BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE else BluetoothGattDescriptor.DISABLE_NOTIFICATION_VALUE
            server?.sendResponse(device, requestId, if (valid) BluetoothGatt.GATT_SUCCESS else BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED,
                offset, if (valid) bytes.copyOfRange(offset, 2) else null)
        }
        override fun onExecuteWrite(device: BluetoothDevice, requestId: Int, execute: Boolean) = current {
            server?.sendResponse(device, requestId, BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED, 0, null)
        }
        override fun onNotificationSent(device: BluetoothDevice, status: Int) = current {
            id(device)?.let { notifications.sent(it, status == BluetoothGatt.GATT_SUCCESS) }
        }
    }
    @Suppress("DEPRECATION")
    override fun send(id: Long, path: SendPath, bytes: ByteArray): Boolean {
        if (bytes.isEmpty() || bytes.size > 512) return false
        return when (path) {
            SendPath.WRITE -> {
                val client = clients[id] ?: return false
                val tx = client.tx ?: return false
                if (Build.VERSION.SDK_INT >= 33) client.gatt.writeCharacteristic(tx, bytes, BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE) == BluetoothStatusCodes.SUCCESS
                else { tx.writeType = BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE; tx.value = bytes; client.gatt.writeCharacteristic(tx) }
            }
            SendPath.NOTIFY -> {
                id in subscriptions && notifications.offer(id, bytes)
            }
        }
    }
    @Suppress("DEPRECATION")
    private fun submitNotification(id: Long, bytes: ByteArray): Boolean {
        // One shared characteristic: even different devices wait for the last
        // notification callback before its value changes on legacy Android.
        val device = peripherals[id] ?: return false
        val rx = notify ?: return false
        return if (Build.VERSION.SDK_INT >= 33) server?.notifyCharacteristicChanged(device, rx, false, bytes) == BluetoothStatusCodes.SUCCESS
        else { rx.value = bytes; server?.notifyCharacteristicChanged(device, rx, false) == true }
    }
    override fun close(id: Long) {
        val address = clients[id]?.gatt?.device?.address ?: peripherals[id]?.address
        address?.let { selection.disconnected(it) }
        clients.remove(id)?.gatt?.let { gatt -> cleanup { gatt.disconnect() }; cleanup { gatt.close() } }
        if (peripherals.containsKey(id)) resetServer()
    }
    private fun resetServer() {
        // Android server completions contain an address, no connection token.
        // Replace the entire server on teardown so stale callbacks carry an old
        // epoch, including when the same address reconnects immediately.
        serverEpoch++
        advertiser?.let { listener -> cleanup { adapter?.bluetoothLeAdvertiser?.stopAdvertising(listener) } }; advertiser = null
        val previous = server
        server = null
        notify = null
        val ids = peripherals.keys.toList()
        peripherals.clear()
        subscriptions.clear()
        refusedAddresses.clear()
        notifications.clear()
        cleanup { previous?.close() }
        ids.forEach { driver.lost(it) }
        if (running) openServer()
    }
    private inline fun cleanup(work: () -> Unit) {
        try { work() } catch (_: SecurityException) { } catch (_: IllegalStateException) { }
    }
    fun stop(reason: RadioState = RadioState.STOPPED): Unit = synchronized(gate) {
        if (!running) return@synchronized
        running = false
        report(reason)
        handler.removeCallbacks(timer)
        // Each native resource is attempted even if permission vanished midway.
        try { scanner?.let { adapter?.bluetoothLeScanner?.stopScan(it) } } catch (_: SecurityException) { }
        scanner = null
        try { driver.stop() } catch (_: SecurityException) { }
        for (id in clients.keys.toList()) try { close(id) } catch (_: SecurityException) { }
        try { resetServer() } catch (_: SecurityException) { serverEpoch++; server?.close(); server = null; peripherals.clear() }
        stopped()
    }
}
