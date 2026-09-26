package org.meshchat.ui

import android.app.KeyguardManager
import android.os.Bundle
import android.os.SystemClock
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import org.meshchat.storage.CipherConnection
import uniffi.meshchat_core.*
import java.io.File
import java.security.SecureRandom

/** Isolated SQLCipher cost comparison, not production key-lifecycle evidence.
 * Never reads the user's database, key envelope or profile. */
class StorageKeyCostTest {
    @Test fun compareSyntheticPassphraseAndRawKeyOpens() {
        check(InstrumentationRegistry.getArguments().getString("physicalMeasure")=="true")
        val instrumentation=InstrumentationRegistry.getInstrumentation()
        val context=instrumentation.targetContext
        assertFalse(context.getSystemService(KeyguardManager::class.java).isDeviceLocked)
        val folder=File(context.cacheDir,"mc025-key-cost").canonicalFile
        check(folder.parentFile==context.cacheDir.canonicalFile)
        check(!folder.exists()) {"Scratch directory exists; preserve it for investigation"}
        check(folder.mkdir())
        val random=SecureRandom()
        val pass=ByteArray(64).also {random.nextBytes(it)}
        val raw=ByteArray(32).also {random.nextBytes(it)}
        // The documented blob literal is passed as bytes; never log key data.
        val hex="0123456789abcdef".toByteArray()
        val rawInput=ByteArray(67).also {out ->
            out[0]='x'.code.toByte();out[1]=39;out[66]=39
            raw.forEachIndexed {i,b -> val n=b.toInt() and 255;out[2+i*2]=hex[n ushr 4];out[3+i*2]=hex[n and 15]}
        }
        val generation=ByteArray(16).also {random.nextBytes(it)}
        val marker="MC025 isolated key-cost record".toByteArray()
        val record="cost".toByteArray()
        try {
            for((label,key) in listOf("passphrase" to pass,"raw" to rawInput)) {
                val file=File(folder,"$label.db")
                CipherConnection.open(file,key,true).use {db ->
                    EncryptedStore.open(db,generation,true,System.currentTimeMillis()/1000).use {
                        it.putRecord(RecordKind.SETTING,record,marker)
                    }
                }
                repeat(5) {index ->
                    val start=SystemClock.elapsedRealtime()
                    CipherConnection.open(file,key,false).use {db ->
                        EncryptedStore.open(db,generation,false,System.currentTimeMillis()/1000).use {
                            assertTrue(marker.contentEquals(it.getRecord(RecordKind.SETTING,record)))
                        }
                    }
                    val elapsed=SystemClock.elapsedRealtime()-start
                    instrumentation.sendStatus(0,Bundle().apply {putString("stream","MC025KEY mode=$label index=$index open_read_close_ms=$elapsed\n")})
                }
                val wrong=key.copyOf().also {it[if(label=="raw")2 else 0]=if(label=="raw") {
                    if(it[2]=='0'.code.toByte()) '1'.code.toByte() else '0'.code.toByte()
                } else (it[0].toInt() xor 1).toByte()}
                try {assertThrows(StorageException::class.java) {CipherConnection.open(file,wrong,false).use { }}}
                finally {wrong.fill(0)}
                assertFalse("Plaintext marker present",file.readBytes().asList().windowed(marker.size).any {it==marker.asList()})
            }
        } finally {
            pass.fill(0);raw.fill(0);rawInput.fill(0)
            // Only this invocation's fresh scratch directory is removed.
            check(folder.deleteRecursively())
        }
    }
}
