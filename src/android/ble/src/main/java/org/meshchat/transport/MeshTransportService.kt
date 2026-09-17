package org.meshchat.transport

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.bluetooth.BluetoothAdapter
import android.os.Build
import android.os.IBinder
import uniffi.meshchat_core.NativeTransport
import uniffi.meshchat_core.TransportEvent

/** Application integration supplies an already protected core; this service
 * never creates plaintext persistence or keeps an unlocked identity session.
 * The application starts it from a visible user action after permission grants.
 */
class MeshTransportService : Service() {
    private class Session(val core: NativeTransport, val event: (Long, TransportEvent) -> Unit, val ready: (AndroidGattRadio) -> Unit)
    companion object {
        private var session: Session? = null
        /** Transfers ownership of core only on success; no automatic restart. */
        @Synchronized fun start(context: Context, core: NativeTransport, event: (Long, TransportEvent) -> Unit, ready: (AndroidGattRadio) -> Unit): Boolean {
            if (session != null || !MeshGatt.allowed(context)) return false
            session = Session(core, event, ready)
            return try { context.startForegroundService(Intent(context, MeshTransportService::class.java)); true }
            catch (_: RuntimeException) { session = null; false }
        }
        @Synchronized private fun take(): Session? = session
        @Synchronized private fun release(value: Session?) { if (session === value) session = null }
    }
    private var active: Session? = null
    private var radio: AndroidGattRadio? = null
    private var registered = false
    private val lifecycle = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            radio?.stateChanged()
        }
    }
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (active != null) return START_NOT_STICKY
        val supplied = take()
        if (supplied == null || !MeshGatt.allowed(this)) { active = supplied; stopSelf(); return START_NOT_STICKY }
        active = supplied
        val notifications = getSystemService(NotificationManager::class.java)
        notifications.createNotificationChannel(NotificationChannel("mesh-transport", "Nearby mesh connection", NotificationManager.IMPORTANCE_LOW))
        startForeground(23, Notification.Builder(this, "mesh-transport")
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentTitle("MeshChat nearby connection")
            .setContentText("Nearby connections are active.")
            .setOngoing(true).build())
        val filter = IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED).apply { addAction(Intent.ACTION_SCREEN_OFF) }
        // Bluetooth broadcasts may originate from the privileged Bluetooth UID.
        // The receiver checks actual adapter state; extras never grant authority.
        if (Build.VERSION.SDK_INT >= 33) registerReceiver(lifecycle, filter, Context.RECEIVER_EXPORTED)
        else { @Suppress("UnspecifiedRegisterReceiverFlag") registerReceiver(lifecycle, filter) }
        registered = true
        val driver = AndroidGattRadio(applicationContext, supplied.core, supplied.event) { stopSelf() }
        radio = driver
        driver.start()
        supplied.ready(driver)
        return START_NOT_STICKY
    }
    override fun onDestroy() {
        radio?.stop(); radio = null
        if (registered) unregisterReceiver(lifecycle)
        registered = false
        active?.core?.close()
        release(active); active = null
        super.onDestroy()
    }
}
