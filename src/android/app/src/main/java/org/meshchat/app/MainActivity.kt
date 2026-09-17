package org.meshchat.app

import android.Manifest
import android.content.Intent
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import org.meshchat.ui.MeshApplication
import org.meshchat.ui.MeshApp

class MainActivity : ComponentActivity() {
    private val model get() = (application as MeshApplication).model
    private val permissions = registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { model.load() }
    private val scanner = registerForActivityResult(ScanContract()) { result ->
        val value=result.contents
        if(value!=null)model.friendInput(value,scanned=true)
        else if(result.originalIntent?.getBooleanExtra(com.google.zxing.client.android.Intents.Scan.MISSING_CAMERA_PERMISSION,false)==true)model.scannerUnavailable()
    }
    private val camera = registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if(granted)scan() else model.scannerUnavailable()
    }
    private fun scan() {
        scanner.launch(ScanOptions().setDesiredBarcodeFormats(ScanOptions.QR_CODE)
            .setBeepEnabled(false).setBarcodeImageEnabled(false).setOrientationLocked(false)
            .setPrompt("Scan the public friend code on their screen"))
    }
    private fun incoming(intent: Intent?) {
        if(intent?.action==Intent.ACTION_VIEW)intent.dataString?.let { model.friendInput(it) }
    }
    override fun onNewIntent(intent: Intent) {super.onNewIntent(intent);setIntent(intent);incoming(intent)}
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if(savedInstanceState==null)incoming(intent)
        setContent { MeshApp(model, permissions = {
            val requested = mutableListOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION)
            if (Build.VERSION.SDK_INT >= 31) requested.addAll(listOf(Manifest.permission.BLUETOOTH_SCAN, Manifest.permission.BLUETOOTH_CONNECT, Manifest.permission.BLUETOOTH_ADVERTISE))
            if (Build.VERSION.SDK_INT >= 33) requested.add(Manifest.permission.POST_NOTIFICATIONS)
            permissions.launch(requested.toTypedArray())
        }, scan = { camera.launch(Manifest.permission.CAMERA) }) }
    }
    override fun onResume() { super.onResume(); model.load() }
}
