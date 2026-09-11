package org.meshchat.app

import android.app.Activity
import android.os.Bundle
import android.widget.TextView
import android.os.SystemClock
import java.security.SecureRandom
import uniffi.meshchat_core.Core
import uniffi.meshchat_core.DriverEvent
import uniffi.meshchat_core.Limits
import uniffi.meshchat_core.PowerState

class MainActivity : Activity() {
    private var core: Core? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        var nonce = 0uL
        val random = SecureRandom()
        while (nonce == 0uL) nonce = random.nextLong().toULong()
        core = Core(Limits(1u, 64u), nonce, SystemClock.elapsedRealtime().toULong())
        core?.handleEvent(DriverEvent.PowerChanged(PowerState.FOREGROUND))
        setContentView(TextView(this).apply { setText(R.string.app_name) })
    }

    override fun onDestroy() {
        core?.close()
        core = null
        super.onDestroy()
    }
}
