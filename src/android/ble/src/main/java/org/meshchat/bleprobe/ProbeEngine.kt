package org.meshchat.bleprobe

import android.Manifest
import android.app.Application
import android.annotation.SuppressLint
import android.bluetooth.*
import android.bluetooth.le.*
import android.content.Context
import android.content.pm.PackageManager
import android.os.*
import java.util.ArrayDeque
import java.util.UUID

/** Test-only UUIDs, not the MC-006 production wire contract. */
object ProbeIds {
    val SERVICE: UUID = UUID.fromString("5f45c0de-71a5-4f81-9f52-52f1ee004001")
    val WRITE: UUID = UUID.fromString("5f45c0de-71a5-4f81-9f52-52f1ee004002")
    val NOTIFY: UUID = UUID.fromString("5f45c0de-71a5-4f81-9f52-52f1ee004003")
    val CCCD: UUID = UUID.fromString("00002902-0000-1000-8000-00805f9b34fb")
    fun permissions(): Array<String> = if (Build.VERSION.SDK_INT >= 31) {
        arrayOf(Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT,
            Manifest.permission.BLUETOOTH_ADVERTISE, Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION)
    } else arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION)
    fun allowed(context: Context): Boolean {
        val location = if (Build.VERSION.SDK_INT >= 31) Manifest.permission.ACCESS_COARSE_LOCATION else Manifest.permission.ACCESS_FINE_LOCATION
        return permissions().filter { it != Manifest.permission.ACCESS_FINE_LOCATION && it != Manifest.permission.ACCESS_COARSE_LOCATION }
            .all { context.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED } &&
            context.checkSelfPermission(location) == PackageManager.PERMISSION_GRANTED
    }
}

// Permission checks are centralized at every action/callback entry in safe().
// Revocation races are caught as SecurityException and close the probe. Android
// lint does not propagate this helper's permission proof into callback lambdas.
@SuppressLint("MissingPermission")
class ProbeEngine(private val context: Application) {
    private val handler = Handler(Looper.getMainLooper())
    private val adapter = context.getSystemService(BluetoothManager::class.java).adapter
    private val lines = ArrayDeque<String>()
    private var dropped = 0L
    private var closed = false
    private var client: BluetoothGatt? = null
    private var write: BluetoothGattCharacteristic? = null
    private var server: BluetoothGattServer? = null
    private var serverEpoch = 0L
    private var peer: BluetoothDevice? = null
    private var notify: BluetoothGattCharacteristic? = null
    private var subscribed = false
    private var advertisingWanted = false
    private var scanning = false
    private var scanEpoch = 0L
    private var scanListener: ScanCallback? = null
    private var clientReady = false
    private var clientBusy = false
    private var notifyBusy = false
    private var writeToken = 0L
    private var notifyToken = 0L
    private val writes = ArrayDeque<ByteArray>()
    private val notifications = ArrayDeque<ByteArray>()
    private var sequence = 0
    private var periodic = false
    var valueSize = 20
        private set

    init {
        record("start model=${Build.MODEL} os=${Build.VERSION.RELEASE} sdk=${Build.VERSION.SDK_INT}")
        record("location_grants fine=${context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION)} coarse=${context.checkSelfPermission(Manifest.permission.ACCESS_COARSE_LOCATION)}")
    }
    fun record(event: String) {
        if (lines.size == 4096) { lines.removeFirst(); dropped++ }
        lines.addLast("${SystemClock.elapsedRealtime()} $event")
    }
    fun report(): String = "MC004 synthetic BLE probe; dropped_trace_rows=$dropped\n" + lines.joinToString("\n")
    fun preview(): String = lines.toList().takeLast(45).joinToString("\n")
    fun setSize(size: Int) {
        if (size in 1..1024) { valueSize = size; record("value_size=$size") }
        else record("invalid_size allowed=1..1024")
    }
    private fun safe(action: () -> Unit) {
        if (closed) return
        if (!ProbeIds.allowed(context)) { record("permission_revoked"); stop(); return }
        try { action() } catch (_: SecurityException) { record("permission_race"); stop() }
    }
    private fun callback(action: () -> Unit) { handler.post { safe(action) } }
    private fun payload(): ByteArray {
        sequence++
        return ByteArray(valueSize) { index ->
            if (index < 4) (sequence ushr ((3 - index) * 8)).toByte() else index.toByte()
        }
    }
    private fun received(path: String, bytes: ByteArray) {
        if (bytes.size > 1024) { record("rx_rejected path=$path length=${bytes.size}"); return }
        // No raw values, addresses, names or message content are logged.
        val seq = if (bytes.size >= 4) bytes.take(4).fold(0L) { n, b -> (n shl 8) or (b.toLong() and 255) } else -1
        record("rx path=$path length=${bytes.size} fixture_seq=$seq")
    }

    fun scan() = safe {
        if (client != null) { record("scan_refused client_exists"); return@safe }
        val scanner = adapter?.bluetoothLeScanner
        if (scanner == null) { record("scan_unavailable"); return@safe }
        if (!scanning) {
            scanning = true
            val epoch = ++scanEpoch
            val listener = scanCallback(epoch)
            scanListener = listener
            scanner.startScan(listOf(ScanFilter.Builder().setServiceUuid(ParcelUuid(ProbeIds.SERVICE)).build()),
                ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build(), listener)
            record("scan_start timeout_ms=60000")
            handler.postDelayed({ safe { if (scanning && scanEpoch == epoch) { stopScan(); record("scan_timeout") } } }, 60_000)
        }
    }
    private fun stopScan() {
        if (scanning) { scanListener?.let { adapter?.bluetoothLeScanner?.stopScan(it) }; scanning = false; scanEpoch++ }
    }
    private fun scanCallback(epoch: Long) = object : ScanCallback() {
        override fun onScanFailed(errorCode: Int) = callback { if (epoch == scanEpoch) { scanning = false; record("scan_failed code=$errorCode") } }
        override fun onScanResult(callbackType: Int, result: ScanResult) = callback {
            if (!scanning || epoch != scanEpoch || client != null) return@callback
            stopScan()
            record("discovered rssi=${result.rssi}; connect_start")
            client = result.device.connectGatt(context, false, clientCallback, BluetoothDevice.TRANSPORT_LE)
            val current = client
            handler.postDelayed({ safe {
                if (client === current && !clientReady) { record("connection_setup_timeout"); disconnectClient() }
            } }, 30_000)
        }
    }
    private val clientCallback = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) = callback {
            if (gatt !== client) return@callback
            record("client_state=$newState status=$status")
            if (newState == BluetoothProfile.STATE_CONNECTED && status == BluetoothGatt.GATT_SUCCESS) {
                if (!gatt.discoverServices()) { record("discovery_request_refused"); disconnectClient() }
            } else if (newState == BluetoothProfile.STATE_DISCONNECTED || status != BluetoothGatt.GATT_SUCCESS) disconnectClient()
        }
        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) = callback {
            if (gatt !== client) return@callback
            if (status != BluetoothGatt.GATT_SUCCESS) { record("discovery_failed=$status"); disconnectClient(); return@callback }
            val service = gatt.getService(ProbeIds.SERVICE)
            write = service?.getCharacteristic(ProbeIds.WRITE)
            val rx = service?.getCharacteristic(ProbeIds.NOTIFY)
            val descriptor = rx?.getDescriptor(ProbeIds.CCCD)
            if (write == null || rx == null || descriptor == null || !gatt.setCharacteristicNotification(rx, true)) {
                record("service_contract_missing"); disconnectClient(); return@callback
            }
            if (!writeDescriptor(gatt, descriptor)) { record("subscribe_request_refused"); disconnectClient() }
        }
        override fun onDescriptorWrite(gatt: BluetoothGatt, descriptor: BluetoothGattDescriptor, status: Int) = callback {
            if (gatt !== client || descriptor.uuid != ProbeIds.CCCD) return@callback
            record("subscribe_status=$status")
            clientReady = status == BluetoothGatt.GATT_SUCCESS
            if (clientReady) { record("client_ready"); requestMtu(517) } else disconnectClient()
        }
        override fun onMtuChanged(gatt: BluetoothGatt, mtu: Int, status: Int) = callback {
            if (gatt === client) record("client_mtu=$mtu payload_hint=${(mtu - 3).coerceAtLeast(0)} status=$status")
        }
        override fun onCharacteristicWrite(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, status: Int) = callback {
            if (gatt !== client || characteristic.uuid != ProbeIds.WRITE || !clientBusy) return@callback
            record("write_callback status=$status")
            clientBusy = false
            drainWrites()
        }
        override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic, value: ByteArray) = callback {
            if (gatt === client && characteristic.uuid == ProbeIds.NOTIFY) received("notify", value)
        }
        @Deprecated("Legacy callback for API 29-32")
        override fun onCharacteristicChanged(gatt: BluetoothGatt, characteristic: BluetoothGattCharacteristic) {
            if (Build.VERSION.SDK_INT < 33) {
                @Suppress("DEPRECATION") val value = characteristic.value?.copyOf() ?: byteArrayOf()
                callback { if (gatt === client && characteristic.uuid == ProbeIds.NOTIFY) received("notify", value) }
            }
        }
    }
    @Suppress("DEPRECATION")
    private fun writeDescriptor(gatt: BluetoothGatt, descriptor: BluetoothGattDescriptor): Boolean =
        if (Build.VERSION.SDK_INT >= 33) gatt.writeDescriptor(descriptor, BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE) == BluetoothStatusCodes.SUCCESS
        else { descriptor.value = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE; gatt.writeDescriptor(descriptor) }

    fun requestMtu(mtu: Int) = safe {
        if (mtu != 23 && mtu != 517) return@safe
        record("request_mtu=$mtu accepted=${client?.requestMtu(mtu) ?: false}")
    }
    fun advertise() = safe { advertisingWanted = true; if (server == null) openServer() }
    private val advertiseCallback = object : AdvertiseCallback() {
        override fun onStartSuccess(settingsInEffect: AdvertiseSettings) = callback { record("advertising_started") }
        override fun onStartFailure(errorCode: Int) = callback { record("advertising_failed=$errorCode") }
    }
    private fun openServer() {
        val epoch = ++serverEpoch
        val service = BluetoothGattService(ProbeIds.SERVICE, BluetoothGattService.SERVICE_TYPE_PRIMARY)
        service.addCharacteristic(BluetoothGattCharacteristic(ProbeIds.WRITE,
            BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE, BluetoothGattCharacteristic.PERMISSION_WRITE))
        val characteristic = BluetoothGattCharacteristic(ProbeIds.NOTIFY, BluetoothGattCharacteristic.PROPERTY_NOTIFY, 0).apply {
            addDescriptor(BluetoothGattDescriptor(ProbeIds.CCCD, BluetoothGattDescriptor.PERMISSION_READ or BluetoothGattDescriptor.PERMISSION_WRITE))
        }
        notify = characteristic
        service.addCharacteristic(characteristic)
        server = context.getSystemService(BluetoothManager::class.java).openGattServer(context, serverCallback(epoch))
        if (server?.addService(service) != true) record("server_add_refused")
    }
    private fun serverCallback(epoch: Long) = object : BluetoothGattServerCallback() {
        private fun current(action: () -> Unit) = callback { if (epoch == serverEpoch) action() }
        override fun onServiceAdded(status: Int, service: BluetoothGattService) = current {
            record("service_added=$status")
            if (status == BluetoothGatt.GATT_SUCCESS && advertisingWanted) {
                val advertiser = adapter?.bluetoothLeAdvertiser
                if (advertiser == null) { record("advertising_unavailable"); return@current }
                advertiser.startAdvertising(AdvertiseSettings.Builder().setConnectable(true)
                    .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_BALANCED).build(),
                    AdvertiseData.Builder().addServiceUuid(ParcelUuid(ProbeIds.SERVICE))
                        .setIncludeDeviceName(false).setIncludeTxPowerLevel(false).build(), advertiseCallback)
            }
        }
        override fun onConnectionStateChange(device: BluetoothDevice, status: Int, newState: Int) = current {
            record("server_state=$newState status=$status epoch=$epoch")
            if (newState == BluetoothProfile.STATE_CONNECTED) {
                if (peer != null && peer != device) { server?.cancelConnection(device); record("server_peer_limit"); return@current }
                peer = device
            } else if (newState == BluetoothProfile.STATE_DISCONNECTED && peer == device) restartServer()
        }
        override fun onMtuChanged(device: BluetoothDevice, mtu: Int) = current {
            if (peer == device) record("server_mtu=$mtu notify_payload_hint=${(mtu - 3).coerceAtLeast(0)}")
        }
        override fun onCharacteristicWriteRequest(device: BluetoothDevice, requestId: Int, characteristic: BluetoothGattCharacteristic,
            preparedWrite: Boolean, responseNeeded: Boolean, offset: Int, value: ByteArray) = current {
            val valid = peer == device && characteristic.uuid == ProbeIds.WRITE && !preparedWrite && offset == 0 && value.size <= 1024
            if (valid) received("write", value) else record("server_write_rejected")
            if (responseNeeded) server?.sendResponse(device, requestId,
                if (valid) BluetoothGatt.GATT_SUCCESS else BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED, 0, null)
        }
        override fun onDescriptorWriteRequest(device: BluetoothDevice, requestId: Int, descriptor: BluetoothGattDescriptor,
            preparedWrite: Boolean, responseNeeded: Boolean, offset: Int, value: ByteArray) = current {
            val valid = peer == device && descriptor.uuid == ProbeIds.CCCD && !preparedWrite && offset == 0 &&
                (value.contentEquals(BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE) || value.contentEquals(BluetoothGattDescriptor.DISABLE_NOTIFICATION_VALUE))
            if (valid) { subscribed = value.contentEquals(BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE); record("server_subscribed=$subscribed") }
            if (responseNeeded) server?.sendResponse(device, requestId,
                if (valid) BluetoothGatt.GATT_SUCCESS else BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED, 0, null)
            if (!subscribed) { record("unsubscribe_dropped=${notifications.size}"); notifications.clear() }
        }
        override fun onDescriptorReadRequest(device: BluetoothDevice, requestId: Int, offset: Int, descriptor: BluetoothGattDescriptor) = current {
            val valid = peer == device && descriptor.uuid == ProbeIds.CCCD && offset == 0
            server?.sendResponse(device, requestId, if (valid) BluetoothGatt.GATT_SUCCESS else BluetoothGatt.GATT_REQUEST_NOT_SUPPORTED, 0,
                if (valid && subscribed) BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE else BluetoothGattDescriptor.DISABLE_NOTIFICATION_VALUE)
        }
        override fun onNotificationSent(device: BluetoothDevice, status: Int) = current {
            if (peer == device && notifyBusy) { record("notify_callback status=$status"); notifyBusy = false; drainNotifications() }
        }
    }
    private fun restartServer() {
        serverEpoch++
        adapter?.bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback)
        server?.close(); server = null; peer = null; subscribed = false; notifyBusy = false
        record("server_cleanup dropped=${notifications.size}"); notifications.clear()
        if (advertisingWanted) openServer()
    }
    fun burst(path: String, count: Int = 16) = safe {
        repeat(count.coerceIn(1, 16)) {
            val queue = if (path == "write") writes else notifications
            val ready = if (path == "write") clientReady else subscribed
            if (!ready) { record("send_refused path=$path not_ready"); return@repeat }
            if (queue.size >= 8) { record("queue_full path=$path"); return@repeat }
            queue.addLast(payload())
            if (path == "write") drainWrites() else drainNotifications()
        }
    }
    private fun accepted(path: String, operation: () -> Boolean): Boolean = try {
        operation()
    } catch (_: IllegalArgumentException) {
        record("api_argument_rejected path=$path")
        false
    }
    @Suppress("DEPRECATION")
    private fun drainWrites() {
        if (clientBusy || writes.isEmpty()) return
        val gatt = client ?: return
        val characteristic = write ?: return
        val value = writes.removeFirst()
        val accepted = accepted("write") { if (Build.VERSION.SDK_INT >= 33)
            gatt.writeCharacteristic(characteristic, value, BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE) == BluetoothStatusCodes.SUCCESS
        else { characteristic.writeType = BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE; characteristic.value = value; gatt.writeCharacteristic(characteristic) } }
        record("write_attempt length=${value.size} accepted=$accepted")
        clientBusy = accepted
        if (!accepted) { record("write_queue_dropped=${writes.size}"); writes.clear(); return }
        val token = ++writeToken
        handler.postDelayed({ safe { if (client === gatt && clientBusy && writeToken == token) {
            record("write_callback_timeout"); disconnectClient()
        } } }, 5_000)
    }
    @Suppress("DEPRECATION")
    private fun drainNotifications() {
        if (notifyBusy || notifications.isEmpty() || !subscribed) return
        val device = peer ?: return
        val characteristic = notify ?: return
        val value = notifications.removeFirst()
        val accepted = accepted("notify") { if (Build.VERSION.SDK_INT >= 33)
            server?.notifyCharacteristicChanged(device, characteristic, false, value) == BluetoothStatusCodes.SUCCESS
        else { characteristic.value = value; server?.notifyCharacteristicChanged(device, characteristic, false) == true } }
        record("notify_attempt length=${value.size} accepted=$accepted")
        notifyBusy = accepted
        if (!accepted) { record("notify_queue_dropped=${notifications.size}"); notifications.clear(); return }
        val epoch = serverEpoch
        val token = ++notifyToken
        handler.postDelayed({ safe { if (epoch == serverEpoch && notifyBusy && notifyToken == token) {
            record("notify_callback_timeout"); restartServer()
        } } }, 5_000)
    }
    fun disconnect() = safe { disconnectClient(); restartServer() }
    private fun disconnectClient() {
        val old = client
        client = null; clientReady = false; clientBusy = false; write = null
        record("client_cleanup dropped=${writes.size}"); writes.clear()
        try { old?.disconnect() } finally { old?.close() }
    }
    fun togglePeriodic() = safe {
        periodic = !periodic
        record("periodic=$periodic interval_ms=2000")
        handler.removeCallbacks(tick)
        if (periodic) handler.post(tick)
    }
    private val tick = object : Runnable {
        override fun run() = safe {
            if (!periodic) return@safe
            if (clientReady) burst("write", 1)
            if (subscribed) burst("notify", 1)
            handler.postDelayed(this, 2_000)
        }
    }
    fun stop() {
        if (closed) return
        record("stop"); closed = true; periodic = false
        handler.removeCallbacksAndMessages(null)
        // Permissions may have been revoked; close each independent resource.
        for (close in listOf<() -> Unit>({ stopScan() }, { adapter?.bluetoothLeAdvertiser?.stopAdvertising(advertiseCallback) },
            { disconnectClient() }, { server?.close() })) {
            try { close() } catch (_: SecurityException) { record("close_permission_denied") }
        }
        server = null; peer = null; writes.clear(); notifications.clear()
    }
}
