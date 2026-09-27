package org.meshchat.ui

import android.app.KeyguardManager
import android.os.Bundle
import android.os.SystemClock
import android.view.WindowManager
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity
import org.meshchat.storage.EncryptedStorage
import uniffi.meshchat_core.*
import java.nio.ByteBuffer
import java.util.concurrent.ThreadPoolExecutor
import java.util.concurrent.TimeUnit

/** Read-only equivalence on the existing synthetic profile; no resets or keys
 * leave the production storage adapter. No message/profile content is logged. */
class StorageSnapshotTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private fun channelBytes(value:ChannelMessage)=ByteBuffer.allocate(FfiConverterTypeChannelMessage.allocationSize(value).toInt())
        .also {FfiConverterTypeChannelMessage.write(value,it)}.array()
    private fun eventBytes(value:EventCard)=ByteBuffer.allocate(FfiConverterTypeEventCard.allocationSize(value).toInt())
        .also {FfiConverterTypeEventCard.write(value,it)}.array()

    @Test fun batchedSnapshotMatchesIndividualProtectedReads() {
        val instrumentation=InstrumentationRegistry.getInstrumentation()
        check(InstrumentationRegistry.getArguments().getString("physicalConnect")=="true")
        val activity=ui.activity
        assertFalse(activity.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        val model=(activity.application as MeshApplication).model
        fun field(name:String)=MeshModel::class.java.getDeclaredField(name).apply {isAccessible=true}.get(model)
        ui.runOnUiThread {activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);model.load();model.stop()}
        try {
            ui.waitUntil(120000) {model.screen.onboarded && !model.screen.locked && model.screen.status=="Nearby connection is off"}
            val profile=model.screen.nickname
            (field("queue") as ThreadPoolExecutor).submit {
                synchronized(checkNotNull(field("coreGate"))) {
                    check(field("radio")==null)
                    val storage=field("storage") as EncryptedStorage
                    val core=field("transport") as NativeTransport
                    val owner=field("owner") as NativeChannels
                    repeat(3) {
                        val at=SystemClock.elapsedRealtime()
                        val events=storage.events(core)
                        val rows=storage.messagingHistory(core,owner,"#general",profile)
                        val friends=storage.friendCards(core,at.toULong())
                        val individualMs=SystemClock.elapsedRealtime()-at
                        val start=SystemClock.elapsedRealtime()
                        val data=storage.channelScreen(core,owner,"#general",profile,at.toULong())
                        val batchMs=SystemClock.elapsedRealtime()-start
                        assertTrue("Existing history must exercise nonempty snapshot",rows.isNotEmpty())
                        assertEquals(rows.size,data.rows.size)
                        rows.zip(data.rows).forEach {(a,b)->assertTrue("History fields changed",channelBytes(a).contentEquals(channelBytes(b)))}
                        assertEquals(events.size,data.events.size)
                        events.zip(data.events).forEach {(a,b)->assertTrue("Event fields changed",eventBytes(a).contentEquals(eventBytes(b)))}
                        assertEquals(friends.size,data.friends.size)
                        friends.zip(data.friends).forEach {(a,b)->assertTrue("Friend fields changed",
                            a.keys.contentEquals(b.keys) && a.copy(keys=b.keys,handle=b.handle)==b)}
                        instrumentation.sendStatus(0,Bundle().apply {putString("stream",
                            "MC025 snapshot equivalent rows=${rows.size} events=${events.size} friends=${friends.size} individual_ms=$individualMs batch_ms=$batchMs\n")})
                    }
                    assertTrue(storage.channelScreen(core,owner,null,profile,SystemClock.elapsedRealtime().toULong()).rows.isEmpty())
                }
            }.get(90,TimeUnit.SECONDS)
        } finally {
            ui.runOnUiThread {activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)}
        }
    }
}
