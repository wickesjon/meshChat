package org.meshchat.wirechecks

import java.io.File
import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_wire_checks.*

class WireVectorsTest {
    private fun vectors(name: String): Map<String, ByteArray> =
        File(System.getProperty("meshchat.root"), "tests/vectors/crypto/$name-v1.tsv")
            .readLines().associate { line ->
                val (key, hex) = line.split('\t')
                key to hex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
            }

    @Test fun publicReferenceBytesAndCorruptionCrossGeneratedBindings() {
        val friend = vectors("friend")
        val dm = vectors("dm")
        val organizer = vectors("organizer")
        val expected = listOf("friend:8", "dm:17", "organizer:8")
        assertEquals(expected, checkWireVectors(friend, dm, organizer))
        // Each family must execute verification, not return a fixed success marker.
        for ((family, key) in listOf(friend to "peer_chat", dm to "chat_2_1", organizer to "chat_a")) {
            val bytes = family.getValue(key)
            val last = bytes.lastIndex
            bytes[last] = (bytes[last].toInt() xor 1).toByte()
            assertThrows(FixtureException.Mismatch::class.java) {
                checkWireVectors(friend, dm, organizer)
            }
            bytes[last] = (bytes[last].toInt() xor 1).toByte()
        }
        assertEquals(expected, checkWireVectors(friend, dm, organizer))
    }
}
