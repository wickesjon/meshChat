package org.meshchat.app

import android.app.Activity
import android.os.Bundle
import android.view.MotionEvent
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.TextView
import com.journeyapps.barcodescanner.DecoratedBarcodeView
import org.meshchat.ui.MeshApplication

/** Private scan results stay in the process and never enter a result Intent,
 * ActivityResultRegistry saved state, an image file or the clipboard. */
class StaffScanActivity : Activity() {
    private lateinit var scanner:DecoratedBarcodeView
    private var scanned=false
    override fun onCreate(savedInstanceState:Bundle?) {
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        super.onCreate(savedInstanceState)
        scanner=DecoratedBarcodeView(this)
        scanner.setStatusText("Scan the private staff code on the organizer's screen")
        val layout=LinearLayout(this).apply {orientation=LinearLayout.VERTICAL}
        layout.addView(TextView(this).apply {setText(R.string.private_staff_scan);setPadding(20,24,20,24)})
        layout.addView(scanner,LinearLayout.LayoutParams(-1,0,1f));setContentView(layout)
        scanner.decodeSingle { result ->
            if(!scanned) {scanned=true;scanner.pause();(application as MeshApplication).model.staffInput(result.text);finish()}
        }
    }
    override fun onResume() {super.onResume();if(!scanned)scanner.resume()}
    override fun onPause() {scanner.pause();super.onPause()}
    override fun onSaveInstanceState(outState:Bundle) {super.onSaveInstanceState(outState);outState.clear()}
    override fun dispatchTouchEvent(event:MotionEvent):Boolean {
        if(event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)!=0)return true
        return super.dispatchTouchEvent(event)
    }
}
