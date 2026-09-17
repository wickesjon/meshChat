package org.meshchat.ui

import android.graphics.Bitmap
import android.os.SystemClock
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.meshchat.app.MainActivity
import org.meshchat.billing.StoreResult
import org.meshchat.identity.IdentityProvider
import org.meshchat.storage.EncryptedStorage
import org.meshchat.storage.StorageVault
import uniffi.meshchat_core.*
import java.io.File
import java.util.concurrent.atomic.AtomicBoolean

/** Test-only store results feed the same protected-cache callback as Play. There
 * is no production intent, preference flag or UI for granting test entitlement. */
class BillingUiTest {
    @get:Rule val ui=createAndroidComposeRule<MainActivity>()
    private val model get()=(ui.activity.application as MeshApplication).model
    private fun awaitState(check:()->Boolean) { ui.waitUntil(120000) {
        assertFalse("Protected app unexpectedly locked",model.screen.locked);check()
    } }
    private fun click(text:String) {
        val node=ui.onNodeWithText(text)
        try {node.assertIsDisplayed()} catch (_:AssertionError) {node.performScrollTo()}
        node.performClick()
    }
    private fun evidence(result:StoreResult) {
        val saved=AtomicBoolean(false)
        ui.runOnUiThread {model.storeResult(result,"Synthetic store: $result") { saved.set(it) }}
        awaitState {saved.get()}
    }
    private fun screenshot(name:String) {
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        File(context.filesDir,"mc032-$name.png").outputStream().use {
            ui.onRoot().captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG,100,it)
        }
    }
    @Test fun supporterAcceptance() {
        awaitState {model.screen.onboarded}
        when(InstrumentationRegistry.getArguments().getString("phase")) {
            "purchase" -> purchase()
            "reopen-refund" -> reopenRefund()
            else -> error("Explicit isolated emulator phase required")
        }
    }
    private fun purchase() {
        assertFalse(model.screen.supporter)
        val friends=model.screen.friends.map {it.keys.toList()}
        for(result in listOf(StoreResult.PENDING,StoreResult.UNAVAILABLE)) {
            evidence(result);assertFalse(model.screen.supporter)
        }
        ui.runOnUiThread {model.select(null)}
        awaitState {model.screen.selected==null && model.screen.direct==null}
        click("Settings")
        ui.onNodeWithText("Ember · Supporter").performScrollTo().assertIsNotEnabled()
        click("Cancel")
        val w=channelWords()
        val candidates=w.descriptors.flatMap { a -> w.genres.map { b -> "$a|$b|${w.locations.first()}" } }.take(35)
        fun fill(limit:Int) {
            for(name in candidates) {
                if(model.screen.channels.count {it.private}>=limit)break
                if(model.screen.channels.any {it.name==name})continue
                ui.runOnUiThread {model.join(name)}
                awaitState {model.screen.channels.any {it.name==name}}
            }
        }
        fill(5)
        val blocked=candidates.first { name -> model.screen.channels.none {it.name==name} }
        ui.runOnUiThread {model.join(blocked)}
        awaitState {model.screen.error?.startsWith("Private channel slots are full") == true}
        assertEquals(5,model.screen.channels.count {it.private})
        evidence(StoreResult.OWNED);assertTrue(model.screen.supporter)
        fill(30)
        val overflow=candidates.first { name -> model.screen.channels.none {it.name==name} }
        ui.runOnUiThread {model.join(overflow)}
        awaitState {model.screen.error?.startsWith("Private channel slots are full") == true}
        assertEquals(33,model.screen.channels.size)
        ui.runOnUiThread {model.select(null)}
        awaitState {model.screen.selected==null}
        click("Settings");click("Violet · Supporter")
        ui.onNodeWithText("Nickname color · six hex digits").performScrollTo().performTextInput("000000")
        click("Save")
        awaitState {model.screen.theme=="violet" && model.screen.nicknameRgb==0u}
        evidence(StoreResult.UNAVAILABLE);assertTrue(model.screen.supporter)
        assertEquals(friends,model.screen.friends.map {it.keys.toList()})
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        val vault=StorageVault.android(context);val identity=IdentityProvider.android(context,vault)
        val storage=EncryptedStorage(identity,vault);val now=SystemClock.elapsedRealtime().toULong()
        NativeChannels(identity.load().identity,now).use { n ->
            n.setCosmetics(true,0u)
            val bytes=n.compose("#general","Synthetic supporter",1u,"Synthetic cosmetic hint",1u,now)
            assertTrue(storage.channelAccept(n,null,bytes,TransportIntake.UNVERIFIED,false,now))
        }
        ui.runOnUiThread {model.select("#general")}
        awaitState {model.screen.rows.any {it.message.text=="Synthetic cosmetic hint"}}
        ui.onNode(hasScrollToNodeAction()).performScrollToNode(hasText("Synthetic cosmetic hint"))
        ui.onNodeWithText("Supporter flair · unverified").assertExists()
        assertFalse(model.screen.rows.last().message.signed)
        assertNull(model.screen.rows.last().message.verifiedPetname)
        screenshot("violet-flair")
    }
    private fun reopenRefund() {
        assertTrue(model.screen.supporter);assertEquals("violet",model.screen.theme)
        assertEquals(0u,model.screen.nicknameRgb);assertEquals(33,model.screen.channels.size)
        evidence(StoreResult.UNAVAILABLE);assertTrue(model.screen.supporter)
        evidence(StoreResult.NOT_OWNED);assertFalse(model.screen.supporter)
        assertEquals("afterhours",Themes.selected(model.screen.theme,false).id)
        assertEquals(33,model.screen.channels.size)
        val existing=model.screen.channels.first {it.private}.name
        ui.runOnUiThread {model.join(existing)}
        awaitState {model.screen.selected?.name==existing}
        ui.onNodeWithText("Send").assertExists()
        for(theme in Themes.all) {
            if(theme.paid)evidence(StoreResult.OWNED)
            ui.runOnUiThread {model.updateProfile(model.screen.nickname,model.screen.avatar,theme.light,theme.id)}
            awaitState {model.screen.theme==theme.id}
            screenshot("theme-${theme.id}")
        }
        evidence(StoreResult.NOT_OWNED)
        // Link confirmation uses the same cap; the proposal remains reviewable.
        ui.runOnUiThread {model.shareInput("meshfest://j/melodic-techno-valley")}
        awaitState {model.screen.channelProposal!=null}
        val name=checkNotNull(model.screen.channelProposal).name
        if(model.screen.channels.none {it.name==name}) {
            ui.runOnUiThread {model.confirmChannel(name)}
            awaitState {model.screen.error?.startsWith("Private channel slots are full") == true}
            assertNotNull(model.screen.channelProposal)
        }
        ui.runOnUiThread {model.cancelChannelProposal()}
    }
}
