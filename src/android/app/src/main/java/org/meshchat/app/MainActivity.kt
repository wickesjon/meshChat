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
import org.meshchat.billing.PlayEntitlements

class MainActivity : ComponentActivity() {
    private var privateScan=false
    private val model get() = (application as MeshApplication).model
    private val billing by lazy { PlayEntitlements(this,report=model::storeResult,priceChanged=model::storePrice) }
    private val permissions = registerForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { model.load() }
    private val scanner = registerForActivityResult(ScanContract()) { result ->
        val value=result.contents
        if(value!=null) {if(privateScan)model.staffInput(value) else model.shareInput(value,scanned=true)}
        else if(result.originalIntent?.getBooleanExtra(com.google.zxing.client.android.Intents.Scan.MISSING_CAMERA_PERMISSION,false)==true)model.scannerUnavailable()
    }
    private val camera = registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if(granted)scan() else model.scannerUnavailable()
    }
    private fun scan() {
        if(privateScan) {startActivity(Intent(this,StaffScanActivity::class.java));return}
        scanner.launch(ScanOptions().setDesiredBarcodeFormats(ScanOptions.QR_CODE)
            .setBeepEnabled(false).setBarcodeImageEnabled(false).setOrientationLocked(false)
            .setCaptureActivity(SecureScanActivity::class.java)
            .setPrompt(if(privateScan)"Scan a private staff provisioning code" else "Scan a channel, public friend or official event code"))
    }
    private fun incoming(intent: Intent?) {
        if(intent?.action==Intent.ACTION_VIEW)intent.dataString?.let { model.shareInput(it) };intent?.data=null
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
        }, scan = { privateScan=false;camera.launch(Manifest.permission.CAMERA) },
            scanStaff = { privateScan=true;camera.launch(Manifest.permission.CAMERA) },
            buySupporter = { billing.purchase(this) }, restoreSupporter = { billing.refresh() }) }
    }
    override fun dispatchTouchEvent(event:android.view.MotionEvent):Boolean {
        if(event.flags and (android.view.MotionEvent.FLAG_WINDOW_IS_OBSCURED or android.view.MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)!=0)return true
        return super.dispatchTouchEvent(event)
    }
    override fun onStop() {model.backgrounded();super.onStop()}
    override fun onResume() { super.onResume(); model.load();billing.refresh() }
    override fun onDestroy() { billing.close();super.onDestroy() }
}
