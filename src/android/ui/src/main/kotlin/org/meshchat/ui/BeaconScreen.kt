package org.meshchat.ui

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.onLongClick
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

private tailrec fun activity(context: Context): Activity? = when(context) {
    is Activity -> context
    is ContextWrapper -> activity(context.baseContext)
    else -> null
}

@Composable
internal fun BeaconScreen(s: MeshScreenState, exit: () -> Unit, connect: () -> Unit) {
    val view=LocalView.current
    DisposableEffect(view) {
        val window=activity(view.context)?.window
        val previous=window?.attributes?.screenBrightness
        val kept=view.keepScreenOn
        window?.attributes=window?.attributes?.apply { screenBrightness=0.06f }
        view.keepScreenOn=true
        onDispose {
            if(previous!=null)window.attributes=window.attributes.apply { screenBrightness=previous }
            view.keepScreenOn=kept
        }
    }
    var position by remember { mutableIntStateOf(0) }
    LaunchedEffect(Unit) { while(true) { delay(60_000);position=(position+1)%4 } }
    // Move all lit pixels, including the exit target, once a minute. Brightness
    // is window-local and restored on exit; no system setting is changed.
    MaterialTheme(colorScheme=meshColors(Themes.selected("afterhours",false))) {
        Column(Modifier.fillMaxSize().background(Color(0xff0e0e10)).padding(16.dp)
            .padding(start=if(position%2==0)0.dp else 12.dp,top=if(position<2)0.dp else 12.dp)
            .verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(16.dp)) {
            Text("Beacon Mode",style=MaterialTheme.typography.headlineMedium,color=MaterialTheme.colorScheme.onBackground)
            val b=s.beacon
            Text(if(b?.active==true) "Relaying nearby" else "Beacon is waiting for a nearby connection",color=MaterialTheme.colorScheme.onBackground)
            Text("${s.peers} connected · ${b?.cachedMessages ?: 0u} cached",color=MaterialTheme.colorScheme.onBackground)
            Text("${b?.relayFrameAttempts ?: 0uL} relay frame attempts · delivery unknown",color=MaterialTheme.colorScheme.onBackground)
            Text("Uptime ${(b?.uptimeMs ?: 0uL)/60_000uL} minutes",color=MaterialTheme.colorScheme.onBackground)
            Text(b?.let { "${it.batteryPercent}% battery · ${if(it.charging)"external power" else "unplugged"}" }
                ?: s.status,color=MaterialTheme.colorScheme.onBackground)
            Text(if(b?.infra==true)"Powered relay hint is on" else "Powered relay hint is off",color=MaterialTheme.colorScheme.onBackground)
            s.error?.let { Text(it,color=MaterialTheme.colorScheme.error) }
            if(b==null)TextButton(onClick=connect) { Text("Connect nearby") }
            Box(Modifier.fillMaxWidth().heightIn(min=64.dp).background(MaterialTheme.colorScheme.surface)
                .semantics(mergeDescendants=true) { onLongClick("Exit Beacon Mode") { exit();true } }
                .pointerInput(exit) { detectTapGestures(onPress={
                    coroutineScope {
                        val held=launch { delay(2_000);exit() }
                        try { tryAwaitRelease() } finally { held.cancel() }
                    }
                }) },contentAlignment=Alignment.Center) {
                Text("Hold 2 seconds to exit",color=MaterialTheme.colorScheme.onSurface)
            }
            Text("Screen-off relaying uses the Android connection service. Actual radio and battery results depend on this device.",color=MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}
