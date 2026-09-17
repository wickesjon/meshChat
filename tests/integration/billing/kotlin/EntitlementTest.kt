package org.meshchat.billing

import org.junit.Assert.*
import org.junit.Test
import org.meshchat.ui.Themes

class EntitlementTest {
    @Test fun unknownPendingAndOfflineCannotGrantButKeepValidatedCosmetics() {
        for(result in listOf(StoreResult.UNAVAILABLE,StoreResult.PENDING,StoreResult.DISABLED)) {
            assertFalse(EntitlementPolicy.active(false,result))
            assertTrue(EntitlementPolicy.active(true,result))
        }
        assertTrue(EntitlementPolicy.active(false,StoreResult.OWNED))
        assertFalse(EntitlementPolicy.active(true,StoreResult.NOT_OWNED))
        assertFalse(EntitlementPolicy.active(false,StoreResult.UNAVAILABLE))
    }
    @Test fun slotsApplyOnlyToNewPrivateSubscriptions() {
        assertTrue(EntitlementPolicy.canAddPrivate(false,4));assertFalse(EntitlementPolicy.canAddPrivate(false,5))
        assertTrue(EntitlementPolicy.canAddPrivate(true,29));assertFalse(EntitlementPolicy.canAddPrivate(true,30))
        assertFalse(EntitlementPolicy.canAddPrivate(false,30))
    }
    @Test fun everyThemeAndFallbackMeetsNormalTextContrast() {
        for(theme in Themes.all) {
            for(text in listOf(theme.text,theme.secondary,theme.accent,theme.error)) {
                assertTrue("${theme.id}: background",Themes.contrast(text,theme.background)>=4.5)
                assertTrue("${theme.id}: surface",Themes.contrast(text,theme.surface)>=4.5)
            }
            assertTrue(Themes.contrast(theme.onAccent,theme.accent)>=4.5)
            for(color in listOf(0xFF000000,0xFFFFFFFF,0xFF777777,0xFF00FF00,0xFFFF0000,0xFF0000FF)) {
                assertTrue(Themes.contrast(Themes.readableNickname(color,theme),theme.surface)>=4.5)
            }
        }
        assertEquals("afterhours",Themes.selected("violet",false).id)
        assertEquals("daylight",Themes.selected("daylight",false).id)
        assertEquals("violet",Themes.selected("violet",true).id)
    }
}
