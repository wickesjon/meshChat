package org.meshchat.ffi

import java.io.File
import java.util.concurrent.TimeUnit
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WireVectorsTest {
    @Test fun generatedCryptoBindingsPassPublicVectors() {
        val root = generateSequence(File(requireNotNull(System.getProperty("user.dir"))).canonicalFile) { it.parentFile }
            .first { File(it, "tests/integration/crypto/run_native.py").isFile }
        val log = File(root, ".work/mc022/kotlin-vector-run.log")
        requireNotNull(log.parentFile).mkdirs()
        val python = if (System.getProperty("os.name").orEmpty().startsWith("Windows")) "python" else "python3"
        val process = ProcessBuilder(python, "-B", "tests/integration/crypto/run_native.py", "kotlin")
            .directory(root).redirectErrorStream(true).redirectOutput(log).start()
        val completed = process.waitFor(15, TimeUnit.MINUTES)
        if (!completed) process.destroyForcibly()
        assertTrue("Crypto vector runner timed out; see $log", completed)
        assertEquals("Crypto vector runner failed; see $log\n${log.readText().takeLast(12000)}", 0, process.exitValue())
    }
}
