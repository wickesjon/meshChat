package org.meshchat.bleprobe

import android.Manifest
import android.app.*
import android.content.*
import android.content.pm.PackageManager
import android.os.*
import android.text.InputType
import android.widget.*

object ProbeSession {
    var engine: ProbeEngine? = null
    var lastReport = "Probe not started. Grant permissions, then start the service."
}

class ProbeService : Service() {
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (!ProbeIds.allowed(this)) { stopSelf(); return START_NOT_STICKY }
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel("probe", "BLE bench session", NotificationManager.IMPORTANCE_LOW))
        val open = PendingIntent.getActivity(this, 0, Intent(this, ProbeActivity::class.java), PendingIntent.FLAG_IMMUTABLE)
        startForeground(4, Notification.Builder(this, "probe")
            .setSmallIcon(android.R.drawable.stat_sys_data_bluetooth)
            .setContentTitle("MeshChat BLE bench is running")
            .setContentText("Open the probe to stop the session.")
            .setContentIntent(open).setOngoing(true).build())
        if (ProbeSession.engine == null) ProbeSession.engine = ProbeEngine(application)
        return START_NOT_STICKY
    }
    override fun onDestroy() {
        ProbeSession.engine?.let { it.stop(); ProbeSession.lastReport = it.report() }
        ProbeSession.engine = null
        super.onDestroy()
    }
}

class ProbeActivity : Activity() {
    private val handler = Handler(Looper.getMainLooper())
    private lateinit var output: TextView
    private lateinit var size: EditText
    private val refresh = object : Runnable {
        override fun run() {
            output.text = ProbeSession.engine?.preview() ?: ProbeSession.lastReport
            handler.postDelayed(this, 500)
        }
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val layout = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(20, 20, 20, 20) }
        fun button(label: String, action: () -> Unit) {
            layout.addView(Button(this).apply { text = label; setOnClickListener { action() } })
        }
        layout.addView(TextView(this).apply {
            setText(R.string.bench_intro)
        })
        button("Grant permissions") { requestPermissions(ProbeIds.permissions(), 1) }
        button("Start foreground service") {
            if (ProbeIds.allowed(this)) startForegroundService(Intent(this, ProbeService::class.java))
            else output.setText(R.string.grant_first)
        }
        if (Build.VERSION.SDK_INT >= 33) button("Allow notification visibility") {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 2)
        }
        if (Build.VERSION.SDK_INT <= 30) button("Allow background location (Android 10/11)") {
            if (checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) == PackageManager.PERMISSION_GRANTED)
                requestPermissions(arrayOf(Manifest.permission.ACCESS_BACKGROUND_LOCATION), 3)
        }
        button("Scan and connect (central)") { ProbeSession.engine?.scan() }
        button("Advertise (peripheral)") { ProbeSession.engine?.advertise() }
        size = EditText(this).apply {
            hint = "Synthetic value size: 1–1024 bytes"
            inputType = InputType.TYPE_CLASS_NUMBER
            setText(R.string.default_size)
        }
        layout.addView(size)
        fun send(path: String, count: Int) {
            val value = size.text.toString().toIntOrNull()
            if (value == null || value !in 1..1024) { size.error = "Use 1–1024"; return }
            ProbeSession.engine?.let { it.setSize(value); it.burst(path, count) }
        }
        button("One write") { send("write", 1) }
        button("One notification") { send("notify", 1) }
        button("Burst 16 writes (queue limit 8)") { send("write", 16) }
        button("Burst 16 notifications (queue limit 8)") { send("notify", 16) }
        button("Toggle traffic every 2 seconds") {
            val value = size.text.toString().toIntOrNull()
            if (value != null && value in 1..1024) ProbeSession.engine?.let { it.setSize(value); it.togglePeriodic() }
        }
        button("Request MTU 23") { ProbeSession.engine?.requestMtu(23) }
        button("Request MTU 517") { ProbeSession.engine?.requestMtu(517) }
        button("Disconnect both links; keep advertising") { ProbeSession.engine?.disconnect() }
        button("Copy sanitized report") {
            getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("MC004 bench",
                ProbeSession.engine?.report() ?: ProbeSession.lastReport))
        }
        button("Stop service") { stopService(Intent(this, ProbeService::class.java)) }
        output = TextView(this).apply { setTextIsSelectable(true) }
        layout.addView(output)
        setContentView(ScrollView(this).apply { addView(layout) })
    }
    override fun onResume() {
        super.onResume()
        ProbeSession.engine?.record("activity_foreground")
        handler.post(refresh)
    }
    override fun onPause() {
        ProbeSession.engine?.record("activity_background; suspension_unconfirmed")
        handler.removeCallbacks(refresh)
        super.onPause()
    }
}
