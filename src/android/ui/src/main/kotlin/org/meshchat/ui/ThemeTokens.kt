package org.meshchat.ui

import kotlin.math.pow

/** Original palettes. Trust labels never use a sender-supplied color. */
data class ThemeTokens(val id:String,val title:String,val paid:Boolean,val light:Boolean,
    val background:Long,val surface:Long,val text:Long,val secondary:Long,val accent:Long,val onAccent:Long,val error:Long)

object Themes {
    val all=listOf(
        ThemeTokens("afterhours","Afterhours",false,false,0xFF16161A,0xFF1F1F25,0xFFF1EFE8,0xFFB4B2A9,0xFF5DCAA5,0xFF092D23,0xFFF09595),
        ThemeTokens("daylight","Daylight",false,true,0xFFFAF9F6,0xFFFFFFFF,0xFF22232A,0xFF55565D,0xFF176B50,0xFFFFFFFF,0xFF9B242C),
        ThemeTokens("ember","Ember",true,false,0xFF211A18,0xFF302521,0xFFF8EDDE,0xFFCCB7A5,0xFFEAB375,0xFF35210D,0xFFFFAAA2),
        ThemeTokens("lagoon","Lagoon",true,false,0xFF102127,0xFF1A3037,0xFFE3F2EF,0xFFAEC8CD,0xFF82D8CC,0xFF103C37,0xFFFFAAA2),
        ThemeTokens("violet","Violet",true,false,0xFF1E192A,0xFF2B243B,0xFFF2EBFC,0xFFC3B5D8,0xFFD0B2F3,0xFF352044,0xFFFFAAA2),
    )
    fun selected(id:String,active:Boolean):ThemeTokens=all.firstOrNull {it.id==id && (!it.paid || active)} ?: all.first()
    private fun luminance(rgb:Long):Double {
        fun component(shift:Int):Double {
            val c=((rgb shr shift) and 255).toDouble()/255
            return if(c<=0.04045)c/12.92 else ((c+0.055)/1.055).pow(2.4)
        }
        return component(16)*0.2126+component(8)*0.7152+component(0)*0.0722
    }
    fun contrast(a:Long,b:Long):Double {
        val x=luminance(a);val y=luminance(b)
        return (maxOf(x,y)+0.05)/(minOf(x,y)+0.05)
    }
    fun readableNickname(rgb:Long,tokens:ThemeTokens):Long = if(
        contrast(rgb,tokens.surface)>=4.5 && contrast(rgb,tokens.background)>=4.5)rgb else tokens.text
}
