package org.meshchat.organizer

import java.io.File
import com.google.zxing.*
import com.google.zxing.common.HybridBinarizer
import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_core.*

class OrganizerImportTest {
    private val root = generateSequence(File(System.getProperty("user.dir"))) { it.parentFile }
        .first { File(it,"AGENTS.md").isFile }
    private val fixtures = File(root,".work/mc041/fixtures.tsv").readLines().associate {
        val pair=it.split('\t',limit=2);pair[0] to pair[1]
    }
    private fun imported(event:String, staff:String, now:Long):Boolean = FixtureDatabase(root).use { database ->
        IdentityKeySession.importUnlocked(ByteArray(64){41},ByteArray(16){41}).use { identity ->
            val own=identity.publicIdentity()
            EncryptedStore.open(database,own.generation,true,now).use { store ->
                try {probeOrganizerImport(store,own,event,staff,now)} catch(_:ProbeException) {false}
            }
        }
    }
    @Test fun toolGeneratedQrAndCanonicalImports() {
        val event=fixtures.getValue("event")
        for (name in listOf("staff_a","staff_b")) {
            val staff=fixtures.getValue(name)
            assertTrue(imported(event,staff,200000))
            assertFalse(imported(event,staff,201001))
            assertFalse(imported(event,staff,199989))
            assertFalse(imported(fixtures.getValue("other"),staff,200000))
            val wrongSeed=staff.substringBeforeLast('/')+"/"+"A".repeat(43)
            assertFalse(imported(event,wrongSeed,200000))
        }
        assertFalse(imported(event,"meshfest://staff/bad/bad",200000))
        // Decode the actual tool's matrix with the mobile scanner library.
        val rows=fixtures.getValue("matrix").split('/')
        val side=(rows.size+8)*4
        val pixels=IntArray(side*side) { i ->
            val x=i%side/4-4;val y=i/side/4-4
            if(x in rows.indices && y in rows.indices && rows[y][x]=='1')0xff000000.toInt() else 0xffffffff.toInt()
        }
        val bitmap=BinaryBitmap(HybridBinarizer(RGBLuminanceSource(side,side,pixels)))
        assertEquals(fixtures.getValue("staff_a"),MultiFormatReader().decode(bitmap).text)
    }
}
