package org.meshchat.ffi

import java.io.File
import java.util.concurrent.TimeUnit
import org.junit.Assert.*
import org.junit.Test
import uniffi.meshchat_core.*

/** Synthetic plaintext SQL double; this is binding/transport evidence only. */
private class TransportDatabase(root: File) : SqlDatabase, AutoCloseable {
    private val folder = File(root, ".work/mc023/kotlin").apply { mkdirs() }
    private val file = File.createTempFile("transport-", ".sqlite", folder).apply { delete() }
    private val process = ProcessBuilder(
        if (System.getProperty("os.name").orEmpty().startsWith("Windows")) "python" else "python3",
        "-B", File(root, "tests/integration/friends/sqlite_worker.py").absolutePath,
        file.absolutePath
    ).redirectError(ProcessBuilder.Redirect.INHERIT).start()
    private val input = process.outputStream.bufferedWriter()
    private val output = process.inputStream.bufferedReader()
    private fun hex(bytes: ByteArray) = bytes.joinToString("") { "%02x".format(it.toInt() and 255) }
    private fun unhex(value: String) = value.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    @Synchronized private fun call(mode: String, sql: String, values: List<SqlValue>, limit: UInt): List<SqlRow> {
        val args = values.joinToString(" ") { value -> when (value) {
            is SqlValue.Integer -> "i${value.value}"
            is SqlValue.Bytes -> "b${hex(value.value)}"
            is SqlValue.Text -> "t${hex(value.value.toByteArray(Charsets.UTF_8))}"
        } }
        input.write("$mode $limit ${hex(sql.toByteArray(Charsets.UTF_8))} $args\n")
        input.flush()
        val response = output.readLine() ?: throw StorageException.Database()
        if (!response.startsWith("ok ")) throw StorageException.Database()
        return List(response.removePrefix("ok ").toInt()) {
            SqlRow((output.readLine() ?: throw StorageException.Database()).split(" ").map { value -> when (value[0]) {
                'i' -> SqlValue.Integer(value.substring(1).toLong())
                'b' -> SqlValue.Bytes(unhex(value.substring(1)))
                else -> SqlValue.Text(unhex(value.substring(1)).toString(Charsets.UTF_8))
            } })
        }
    }
    override fun execute(sql: String, values: List<SqlValue>) { call("x", sql, values, 0u) }
    override fun query(sql: String, values: List<SqlValue>, limit: UInt) = call("q", sql, values, limit)
    override fun close() {
        input.close()
        if (!process.waitFor(5, TimeUnit.SECONDS)) process.destroyForcibly()
        output.close()
    }
}

class NativeTransportTest {
    private fun root(): File = generateSequence(File(requireNotNull(System.getProperty("user.dir")))) { it.parentFile }
        .first { File(it, "AGENTS.md").isFile }
    private fun <T> owner(seed: Int, action: (NativeTransport, IdentityKeySession) -> T): T {
        TransportDatabase(root()).use { database ->
            IdentityKeySession.importUnlocked(ByteArray(64) { seed.toByte() }, ByteArray(16) { seed.toByte() }).use { identity ->
                EncryptedStore.open(database, identity.publicIdentity().generation, true, 200000L).use { store ->
                    NativeTransport(store, identity.publicIdentity(), seed.toULong(), 0uL).use { core ->
                        try { return action(core, identity) } finally { identity.invalidate() }
                    }
                }
            }
        }
    }
    @Test fun generatedBindingRunsRealHelloAndProofInBothRoles() {
        owner(31) { a, ap -> owner(32) { b, bp ->
            val al = a.nativeReady(a.admitConnection(ByteArray(16) { 1 }, 0uL), TransportRole.CENTRAL, 512u, 146u, 0uL).link
            val bl = b.nativeReady(b.admitConnection(ByteArray(16) { 2 }, 0uL), TransportRole.PERIPHERAL, 182u, 512u, 0uL).link
            val ah = a.tick(0uL).sends.single()
            val bh = b.tick(0uL).sends.single()
            assertEquals(59, ah.bytes.size)
            assertEquals(SendPath.WRITE, ah.path)
            assertEquals(SendPath.NOTIFY, bh.path)
            b.receive(bl, 59uL, ah.bytes, 0uL)
            a.receive(al, 59uL, bh.bytes, 0uL)
            val ae = a.complete(al, ah.token, true, 0uL).events.single() as TransportEvent.Admitted
            val be = b.complete(bl, bh.token, true, 0uL).events.single() as TransportEvent.Admitted
            assertEquals(512u.toUShort(), ae.transmitBytes)
            assertEquals(146u.toUShort(), ae.receiveBytes)
            assertEquals(146u.toUShort(), be.transmitBytes)
            a.prepareProof(al, ap, 0uL)
            b.prepareProof(bl, bp, 0uL)
            val af = a.tick(1000uL).sends.single()
            val bf = b.tick(1000uL).sends.single()
            assertEquals(71, af.bytes.size)
            assertTrue(b.receive(bl, 71uL, af.bytes, 1000uL).events.isEmpty())
            assertTrue(a.receive(al, 71uL, bf.bytes, 1000uL).events.isEmpty())
            a.complete(al, af.token, true, 1000uL)
            b.complete(bl, bf.token, true, 1000uL)
            a.disconnect(al, 1001uL)
            assertThrows(TransportException.Stale::class.java) { a.complete(al, af.token, true, 1001uL) }
        } }
    }
    @Test fun typedCapacityAndCallbackTimeoutRefusals() {
        owner(33) { core, _ ->
            val bad = core.admitConnection(ByteArray(16) { 3 }, 0uL)
            assertThrows(TransportException.Invalid::class.java) { core.nativeReady(bad, TransportRole.CENTRAL, 145u, 512u, 0uL) }
            val good = core.admitConnection(ByteArray(16) { 4 }, 0uL)
            val link = core.nativeReady(good, TransportRole.CENTRAL, 146u, 146u, 0uL).link
            val send = core.tick(0uL).sends.single()
            assertTrue(core.tick(5000uL).events.single() is TransportEvent.Closed)
            assertThrows(TransportException.Stale::class.java) { core.complete(link, send.token, true, 5000uL) }
        }
    }
}
