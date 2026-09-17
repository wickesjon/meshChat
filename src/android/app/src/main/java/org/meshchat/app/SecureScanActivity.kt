package org.meshchat.app

import android.os.Bundle
import android.view.MotionEvent
import android.view.WindowManager
import com.journeyapps.barcodescanner.CaptureActivity

/** No barcode image, recents thumbnail or ordinary screenshot for provisioning. */
class SecureScanActivity : CaptureActivity() {
    override fun onCreate(savedInstanceState:Bundle?) {
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        super.onCreate(savedInstanceState)
    }
    override fun dispatchTouchEvent(event:MotionEvent):Boolean {
        if(event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)!=0)return true
        return super.dispatchTouchEvent(event)
    }
}
