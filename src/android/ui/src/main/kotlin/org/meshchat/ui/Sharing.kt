package org.meshchat.ui

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.os.PersistableBundle
import androidx.core.content.FileProvider
import com.google.zxing.BarcodeFormat
import com.google.zxing.EncodeHintType
import com.journeyapps.barcodescanner.BarcodeEncoder
import uniffi.meshchat_core.*
import java.io.File

/** Public sharing only. No event/staff provisioning or private seed export. */
internal object Sharing {
    const val HTTPS_NOTICE = "HTTPS app opening is not verified yet. Installation needs internet and a published store listing. In-app QR scanning works offline."
    fun channelUri(name: String): String {
        val channel=channelInfo(name)
        require(channel.private)
        return "meshfest://j/"+channel.name.replace('|','-')
    }
    private fun canonical(uri: String): String = try {
        channelUri(channelLink(uri).name)
    } catch (_: ChannelException.Invalid) {
        friendProposal(uri) // Validate the whole URI before extracting either path form.
        "meshfest://friend/"+uri.substringAfter("://").split('/').takeLast(2).joinToString("/")
    }
    fun https(uri: String): String = "https://meshfest.app/"+canonical(uri).removePrefix("meshfest://")
    fun qr(uri: String): Bitmap = BarcodeEncoder().encodeBitmap(canonical(uri),BarcodeFormat.QR_CODE,640,640,
        mapOf(EncodeHintType.MARGIN to 4))
    fun clip(uri: String): ClipData = ClipData.newPlainText("MeshChat shared link",https(uri)).apply {
        description.extras=PersistableBundle().apply {
            // Android's documented compatibility key; ignored by older clipboard UIs.
            putBoolean("android.content.extra.IS_SENSITIVE",true)
        }
    }
    fun linkIntent(uri: String): Intent = Intent(Intent.ACTION_SEND).setType("text/plain")
        .putExtra(Intent.EXTRA_TEXT,https(uri)+"\n\n"+HTTPS_NOTICE)
    fun qrIntent(context: Context, uri: String): Intent {
        val code=canonical(uri)
        val directory=File(context.cacheDir,"share-qr").apply {mkdirs()}
        // Limit temporary exported images. Never include them in backup or logs.
        directory.listFiles()?.filter {System.currentTimeMillis()-it.lastModified()>24*60*60*1000L}?.forEach {it.delete()}
        check((directory.listFiles()?.size ?: 0)<32) {"QR export cache is full"}
        val bitmap=qr(code)
        val file=File.createTempFile("code-",".png",directory)
        try {
            file.outputStream().use {check(bitmap.compress(Bitmap.CompressFormat.PNG,100,it))}
        } catch (failure: Exception) {file.delete();throw failure} finally {bitmap.recycle()}
        val content=FileProvider.getUriForFile(context,context.packageName+".share",file)
        return Intent(Intent.ACTION_SEND).setType("image/png").putExtra(Intent.EXTRA_STREAM,content).apply {
            clipData=ClipData.newUri(context.contentResolver,"MeshChat QR",content)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
    }
}
