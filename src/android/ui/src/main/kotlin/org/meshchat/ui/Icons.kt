package org.meshchat.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

internal val animals = listOf("mask","cat","dog","lizard","raccoon","turtle","fish","bird","whale")
internal val palette = listOf(0xFF5DCAA5,0xFFF49B83,0xFFBDA6EF,0xFFE8A0CE,0xFF8DBBEF,0xFFA3CB8C,0xFFEFBB67,0xFF91CCCA,
    0xFF9FE1CB,0xFFF7BEAA,0xFFD3C5F5,0xFFF1C1DF,0xFFB9D4F6,0xFFC5DEB3,0xFFF5D79E,0xFFBCE2E0).map { Color(it) }
internal val reactions = listOf("fire","heart","laugh","thumb","party","surprise","melt","speaker")
@Composable
internal fun nicknameColor(sender: ByteArray): Color {
    val base=palette[sender.fold(0){sum,b -> (sum*31+(b.toInt() and 255)) and 15}]
    return if(MaterialTheme.colorScheme.background.luminance()>.5f) Color(base.red*.45f,base.green*.45f,base.blue*.45f) else base
}
/** Bundled vector paths; never platform emoji or attacker-provided images. */
@Composable
fun MeshIcon(name: String, tint: Color = MaterialTheme.colorScheme.primary, modifier: Modifier = Modifier) {
    Canvas(modifier.size(28.dp).semantics { contentDescription = name }) {
        val stroke=Stroke(width=1.7f)
        scale(size.width/24f,size.height/24f,pivot=Offset.Zero) {
            val p=Path()
            when(name) {
                "wave" -> { p.moveTo(2f,13f);p.cubicTo(6f,3f,9f,22f,13f,12f);p.cubicTo(17f,2f,19f,17f,22f,9f) }
                "lightning-bolt" -> { p.moveTo(14f,2f);p.lineTo(4f,14f);p.lineTo(11f,14f);p.lineTo(10f,22f);p.lineTo(20f,9f);p.lineTo(13f,9f);p.close() }
                "fire" -> { p.moveTo(12f,2f);p.cubicTo(17f,8f,23f,13f,18f,19f);p.cubicTo(9f,27f,1f,16f,8f,9f);p.cubicTo(7f,14f,15f,10f,12f,2f);p.close() }
                "heart" -> { p.moveTo(12f,21f);p.cubicTo(-7f,8f,7f,-1f,12f,7f);p.cubicTo(18f,-1f,31f,8f,12f,21f);p.close() }
                "moon" -> { p.moveTo(17f,3f);p.cubicTo(1f,-1f,1f,26f,21f,18f);p.cubicTo(9f,20f,7f,7f,17f,3f);p.close() }
                "star" -> { p.moveTo(12f,2f);p.lineTo(16f,9f);p.lineTo(22f,12f);p.lineTo(16f,15f);p.lineTo(12f,22f);p.lineTo(8f,15f);p.lineTo(2f,12f);p.lineTo(8f,9f);p.close() }
                "crystal" -> { p.moveTo(12f,2f);p.lineTo(20f,8f);p.lineTo(17f,20f);p.lineTo(7f,22f);p.lineTo(4f,9f);p.close();p.moveTo(12f,2f);p.lineTo(10f,10f);p.lineTo(7f,22f);p.moveTo(10f,10f);p.lineTo(20f,8f) }
                "sun" -> { drawCircle(tint,5f,Offset(12f,12f),style=stroke);for(i in 0..7){val a=i*Math.PI/4;p.moveTo(12f+8f*kotlin.math.cos(a).toFloat(),12f+8f*kotlin.math.sin(a).toFloat());p.lineTo(12f+11f*kotlin.math.cos(a).toFloat(),12f+11f*kotlin.math.sin(a).toFloat())} }
                "spiral" -> { p.moveTo(12f,12f);for(i in 1..120){val a=i*.12f;val r=i*.075f;p.lineTo(12f+r*kotlin.math.cos(a),12f+r*kotlin.math.sin(a))} }
                "disco-ball" -> { drawCircle(tint,9f,Offset(12f,13f),style=stroke);p.moveTo(12f,0f);p.lineTo(12f,4f);for(x in listOf(8f,12f,16f)){p.moveTo(x,6f);p.lineTo(x,20f)};for(y in listOf(9f,13f,17f)){p.moveTo(5f,y);p.lineTo(19f,y)} }
                "open-lock","closed-lock" -> { p.moveTo(7f,11f);p.lineTo(7f,7f);p.cubicTo(7f,0f,18f,0f,18f,7f);if(name=="closed-lock")p.lineTo(18f,11f);p.moveTo(4f,11f);p.lineTo(21f,11f);p.lineTo(21f,22f);p.lineTo(4f,22f);p.close() }
                "mushroom" -> { p.moveTo(2f,14f);p.cubicTo(3f,-1f,22f,-1f,23f,14f);p.close();p.moveTo(10f,14f);p.lineTo(9f,22f);p.lineTo(16f,22f);p.lineTo(15f,14f) }
                "cactus" -> { p.moveTo(10f,22f);p.lineTo(10f,4f);p.cubicTo(10f,0f,15f,0f,15f,4f);p.lineTo(15f,22f);p.moveTo(10f,13f);p.lineTo(5f,13f);p.lineTo(5f,7f);p.moveTo(15f,16f);p.lineTo(20f,16f);p.lineTo(20f,10f) }
                "speaker" -> { p.moveTo(3f,9f);p.lineTo(8f,9f);p.lineTo(14f,4f);p.lineTo(14f,21f);p.lineTo(8f,16f);p.lineTo(3f,16f);p.close();p.moveTo(18f,6f);p.cubicTo(25f,9f,25f,16f,18f,19f) }
                "mask" -> { p.moveTo(2f,7f);p.quadraticTo(12f,3f,22f,7f);p.quadraticTo(21f,21f,12f,15f);p.quadraticTo(3f,21f,2f,7f);p.close();drawCircle(tint,1f,Offset(7f,11f));drawCircle(tint,1f,Offset(17f,11f)) }
                "cat","dog","raccoon" -> {
                    drawCircle(tint,8f,Offset(12f,13f),style=stroke)
                    if(name=="cat"||name=="raccoon"){p.moveTo(5f,9f);p.lineTo(3f,2f);p.lineTo(10f,6f);p.moveTo(14f,6f);p.lineTo(21f,2f);p.lineTo(19f,9f)}
                    if(name=="dog"){p.moveTo(5f,6f);p.quadraticTo(-3f,14f,5f,17f);p.moveTo(19f,6f);p.quadraticTo(28f,14f,19f,17f)}
                    if(name=="raccoon"){p.moveTo(5f,10f);p.lineTo(19f,10f);p.lineTo(16f,15f);p.lineTo(8f,15f);p.close()}
                    drawCircle(tint,1f,Offset(9f,12f));drawCircle(tint,1f,Offset(15f,12f));p.moveTo(9f,17f);p.quadraticTo(12f,19f,15f,17f)
                }
                "lizard" -> { p.moveTo(12f,3f);p.cubicTo(3f,6f,18f,14f,10f,20f);p.quadraticTo(6f,24f,4f,19f);p.moveTo(10f,8f);p.lineTo(4f,8f);p.lineTo(2f,5f);p.moveTo(12f,11f);p.lineTo(20f,8f);p.lineTo(22f,5f);p.moveTo(11f,15f);p.lineTo(18f,17f);p.lineTo(19f,21f);drawCircle(tint,2f,Offset(12f,4f)) }
                "turtle" -> { drawOval(tint,Offset(5f,7f),androidx.compose.ui.geometry.Size(14f,13f),style=stroke);drawCircle(tint,3f,Offset(12f,4f),style=stroke);p.moveTo(6f,9f);p.lineTo(2f,6f);p.moveTo(18f,9f);p.lineTo(22f,6f);p.moveTo(7f,18f);p.lineTo(3f,22f);p.moveTo(17f,18f);p.lineTo(21f,22f);p.moveTo(12f,7f);p.lineTo(8f,13f);p.lineTo(12f,19f);p.lineTo(16f,13f);p.close() }
                "fish" -> { p.moveTo(2f,12f);p.quadraticTo(9f,1f,17f,9f);p.lineTo(23f,5f);p.lineTo(23f,19f);p.lineTo(17f,15f);p.quadraticTo(9f,23f,2f,12f);p.close();drawCircle(tint,1f,Offset(7f,11f)) }
                "bird" -> { p.moveTo(4f,20f);p.lineTo(4f,10f);p.cubicTo(4f,1f,17f,0f,18f,9f);p.lineTo(23f,12f);p.lineTo(18f,14f);p.quadraticTo(16f,23f,4f,20f);p.moveTo(6f,14f);p.quadraticTo(11f,21f,15f,13f);drawCircle(tint,1f,Offset(14f,8f)) }
                "whale" -> { p.moveTo(2f,12f);p.cubicTo(2f,1f,17f,5f,17f,14f);p.lineTo(22f,9f);p.lineTo(22f,17f);p.cubicTo(10f,25f,2f,21f,2f,12f);p.close();p.moveTo(8f,5f);p.lineTo(8f,1f);p.moveTo(8f,3f);p.lineTo(12f,1f);drawCircle(tint,1f,Offset(6f,12f)) }
                "paw" -> { drawOval(tint,Offset(6f,12f),androidx.compose.ui.geometry.Size(12f,9f),style=stroke);for(i in 0..3)drawCircle(tint,2f,Offset(4f+i*5f,if(i==0||i==3)9f else 5f),style=stroke) }
                "thumb" -> { p.moveTo(5f,10f);p.lineTo(10f,10f);p.lineTo(13f,2f);p.lineTo(16f,3f);p.lineTo(15f,10f);p.lineTo(22f,10f);p.lineTo(20f,21f);p.lineTo(5f,21f);p.close() }
                "party" -> { p.moveTo(2f,22f);p.lineTo(7f,7f);p.lineTo(17f,17f);p.close();p.moveTo(10f,4f);p.lineTo(13f,1f);p.moveTo(17f,9f);p.lineTo(22f,5f);p.moveTo(20f,14f);p.lineTo(24f,13f) }
                "comet" -> { drawCircle(tint,5f,Offset(7f,17f),style=stroke);p.moveTo(3f,12f);p.lineTo(21f,2f);p.lineTo(12f,21f);p.moveTo(11f,13f);p.lineTo(21f,3f) }
                "surprise" -> { drawCircle(tint,9f,Offset(12f,12f),style=stroke);drawCircle(tint,1f,Offset(8f,9f));drawCircle(tint,1f,Offset(16f,9f));drawCircle(tint,3f,Offset(12f,16f),style=stroke) }
                "laugh" -> { drawCircle(tint,9f,Offset(12f,12f),style=stroke);p.moveTo(6f,10f);p.lineTo(8f,8f);p.lineTo(10f,10f);p.moveTo(14f,10f);p.lineTo(16f,8f);p.lineTo(18f,10f);p.moveTo(6f,14f);p.lineTo(18f,14f);p.quadraticTo(12f,25f,6f,14f) }
                "melt" -> { p.moveTo(3f,15f);p.cubicTo(-1f,0f,24f,0f,21f,15f);p.cubicTo(28f,19f,19f,24f,13f,20f);p.cubicTo(5f,26f,-3f,20f,3f,15f);drawCircle(tint,1f,Offset(8f,9f));drawCircle(tint,1f,Offset(16f,11f));p.moveTo(8f,15f);p.quadraticTo(13f,20f,17f,14f) }
                else -> { drawCircle(tint,8f,Offset(12f,12f),style=stroke);drawCircle(tint,1f,Offset(9f,10f));drawCircle(tint,1f,Offset(15f,10f));p.moveTo(8f,15f);p.quadraticTo(12f,20f,16f,15f) }
            }
            drawPath(p,tint,style=stroke)
        }
    }
}
@Composable
internal fun Avatar(value: UByte,modifier: Modifier = Modifier) {
    val raw=value.toInt();val animal=animals.getOrElse(raw and 15){"paw"}
    val base=if(raw==0)Color(0xFFB4B2A9) else palette[raw shr 4]
    val tint=if(MaterialTheme.colorScheme.background.luminance()>.5f)Color(base.red*.45f,base.green*.45f,base.blue*.45f) else base
    Box(modifier.size(44.dp).background(tint.copy(alpha=.12f),CircleShape),contentAlignment=Alignment.Center){MeshIcon(animal,tint)}
}
