package org.meshchat.organizer
import java.io.File
import java.util.concurrent.TimeUnit
import uniffi.meshchat_core.*
// Synthetic plaintext SQL callback; not a storage-protection test.
class FixtureDatabase(root: File) : SqlDatabase, AutoCloseable {
    private val folder = File(root, ".work/mc041/kotlin").apply { mkdirs() }
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
