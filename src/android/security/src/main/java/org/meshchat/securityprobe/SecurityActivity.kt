package org.meshchat.securityprobe

import android.annotation.SuppressLint
import android.app.Activity
import android.app.KeyguardManager
import android.content.ClipData
import android.content.ClipboardManager
import android.os.Build
import android.os.Bundle
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyInfo
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import java.io.File
import java.security.KeyPairGenerator
import java.security.KeyFactory
import java.security.KeyStore
import java.security.Signature
import java.security.spec.ECGenParameterSpec
import java.security.spec.NamedParameterSpec
import java.security.spec.PKCS8EncodedKeySpec
import java.security.spec.X509EncodedKeySpec
import java.util.concurrent.ScheduledThreadPoolExecutor
import java.util.concurrent.TimeUnit
import javax.crypto.Cipher
import javax.crypto.KeyAgreement
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.SecretKeyFactory
import javax.crypto.spec.GCMParameterSpec
import net.zetetic.database.sqlcipher.SQLiteDatabase
import uniffi.meshchat_core.probeRandomMaterial
import uniffi.meshchat_core.probeSigningPublic
import uniffi.meshchat_core.probeSign
import uniffi.meshchat_core.probeVerify
import uniffi.meshchat_core.probeAgreementPublic
import uniffi.meshchat_core.probeAgreementMatches

/** Synthetic bench only. No real identities or messages. */
class SecurityActivity : Activity() {
    private val worker = ScheduledThreadPoolExecutor(1).apply { setExecuteExistingDelayedTasksAfterShutdownPolicy(false) }
    private val rows = ArrayDeque<String>()
    private lateinit var output: TextView
    private var busy = false
    private var held: SQLiteDatabase? = null // Worker owns DB/key work.
    private val alias = "mc005-wrapping-v1"
    private val store: KeyStore get() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private val folder: File get() = File(noBackupFilesDir, "mc005")
    private val envelope: File get() = File(folder, "material.enc")
    private val database: File get() = File(folder, "fixture.db")

    @SuppressLint("SetTextI18n") // Fixed English labels in a temporary bench.
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        System.loadLibrary("sqlcipher")
        val layout = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(24, 24, 24, 24) }
        output = TextView(this)
        layout.addView(TextView(this).apply { text = "MC-005 synthetic security bench. Copy report before restart. Reset deletes only this fixture." })
        fun action(label: String, operation: () -> Unit) {
            layout.addView(Button(this).apply { text = label; setOnClickListener { execute(label, operation) } })
        }
        action("Create fresh protected fixture") { create() }
        action("Reopen + curves + wrong-key rejection") { verify() }
        action("Open and hold DB across lock") { check(held == null); held = open(false); event("db_held_open") }
        action("Read held DB") { checkRow(checkNotNull(held)); event("held_read_ok") }
        action("Close held DB") { held?.close(); held = null; event("held_closed") }
        action("Schedule lock reads in 15 seconds") {
            worker.schedule({
                try { open(false).use { checkRow(it) }; event("scheduled_reopen_ok") }
                catch (e: Exception) { event("scheduled_reopen_failed=${e.javaClass.simpleName}") }
                held?.let {
                    try { checkRow(it); event("scheduled_held_read_ok") }
                    catch (e: Exception) { event("scheduled_held_read_failed=${e.javaClass.simpleName}") }
                }
            }, 15, TimeUnit.SECONDS)
            event("lock_read_scheduled; lock_screen_now; timing_not_guaranteed")
        }
        action("Probe platform curve availability") { platformCurves() }
        action("Invalidate wrapping key only") { held?.close(); held = null; store.deleteEntry(alias); event("wrapping_key_deleted; ciphertext_retained") }
        action("Reset synthetic fixture") { reset() }
        layout.addView(Button(this).apply { text = "Copy sanitized report"; setOnClickListener {
            getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("MC005", rows.joinToString("\n")))
        } })
        layout.addView(output); setContentView(ScrollView(this).apply { addView(layout) })
        event("start api=${Build.VERSION.SDK_INT}; process_restart_requires_manual_reopen; reports_not_persisted")
    }
    private fun execute(name: String, operation: () -> Unit) {
        if (busy) { event("busy_refused"); return }
        busy = true
        worker.execute {
            try { operation(); event("operation_ok=$name") }
            catch (e: Exception) { event("operation_failed=$name type=${e.javaClass.simpleName}; no_error_message_logged") }
            finally { runOnUiThread { busy = false } }
        }
    }
    private fun event(value: String) = runOnUiThread {
        if (rows.size == 512) rows.removeFirst()
        val locked = getSystemService(KeyguardManager::class.java).isDeviceLocked
        rows.addLast("${android.os.SystemClock.elapsedRealtime()} locked=$locked $value")
        output.text = rows.joinToString("\n")
    }
    private fun wrappingKey(): SecretKey = (store.getKey(alias, null) as? SecretKey)
        ?: error("missing wrapping key; explicit reset required")
    private fun material(): ByteArray {
        check(envelope.isFile && envelope.length() in 29..256)
        val encoded = AtomicFile(envelope).readFully()
        check(encoded[0] == 1.toByte())
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, wrappingKey(), GCMParameterSpec(128, encoded.copyOfRange(1, 13)))
        cipher.updateAAD("mc005-material-v1".toByteArray(Charsets.UTF_8))
        return cipher.doFinal(encoded, 13, encoded.size - 13).also { check(it.size == 96) }
    }
    @Suppress("DEPRECATION") // API 29/30 expose hardware status via isInsideSecureHardware.
    private fun create() {
        check(!folder.exists() && !store.containsAlias(alias)) { "explicit reset required" }
        check(folder.mkdirs())
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
            .setKeySize(256).setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).setUnlockedDeviceRequired(true).build())
        val key = generator.generateKey()
        val info = SecretKeyFactory.getInstance(key.algorithm, "AndroidKeyStore").getKeySpec(key, KeyInfo::class.java) as KeyInfo
        event("wrapping_nonexportable=${key.encoded == null}; hardware=${info.isInsideSecureHardware}; unlocked_required=true")
        val bytes = probeRandomMaterial()
        try {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, key)
            cipher.updateAAD("mc005-material-v1".toByteArray(Charsets.UTF_8))
            val atomic = AtomicFile(envelope); val stream = atomic.startWrite()
            try { stream.write(byteArrayOf(1) + cipher.iv + cipher.doFinal(bytes)); atomic.finishWrite(stream) }
            catch (e: Exception) { atomic.failWrite(stream); throw e }
            val dbKey = bytes.copyOfRange(64, 96)
            try { SQLiteDatabase.openOrCreateDatabase(database, dbKey, null, null, null).use { db ->
                cipherPresent(db); db.execSQL("CREATE TABLE fixture (value INTEGER NOT NULL)")
                db.execSQL("INSERT INTO fixture VALUES (5)"); checkRow(db)
            } } finally { dbKey.fill(0) }
            event("created; no_backup_directory=true; backup_manifest_excluded=true; actual_restore_untested")
        } finally { bytes.fill(0) }
    }
    private fun open(wrong: Boolean): SQLiteDatabase {
        check(database.isFile)
        val bytes = material()
        val key = bytes.copyOfRange(64, 96); bytes.fill(0)
        if (wrong) key[0] = (key[0].toInt() xor 1).toByte()
        try { return SQLiteDatabase.openDatabase(database.absolutePath, key, null, SQLiteDatabase.OPEN_READWRITE, null, null) }
        finally { key.fill(0) }
    }
    private fun cipherPresent(db: SQLiteDatabase) {
        db.rawQuery("PRAGMA cipher_version", null).use { cursor ->
            check(cursor.moveToFirst() && cursor.getString(0).isNotBlank()); event("cipher_version=${cursor.getString(0)}")
        }
    }
    private fun checkRow(db: SQLiteDatabase) {
        db.rawQuery("SELECT value FROM fixture", null).use { check(it.moveToFirst() && it.getInt(0) == 5 && !it.moveToNext()) }
    }
    private fun verify() {
        check(held == null) { "close held database first" }
        open(false).use { cipherPresent(it); checkRow(it) }; event("reopen_ok")
        val bytes = material()
        val signing = bytes.copyOfRange(0, 32); val agreement = bytes.copyOfRange(32, 64); bytes.fill(0)
        try {
            val msg = "MC005 synthetic challenge".toByteArray(Charsets.UTF_8)
            val public = probeSigningPublic(signing); val signature = probeSign(signing, msg)
            check(probeVerify(public, msg, signature)); signature[0] = (signature[0].toInt() xor 1).toByte()
            check(!probeVerify(public, msg, signature)); check(probeAgreementPublic(agreement).size == 32)
            event("rust_ed25519_positive_negative_ok; rust_x25519_public_ok")
            nativeCurves(signing, agreement, msg)
        } finally { signing.fill(0); agreement.fill(0) }
        var rejected = false
        try { open(true).use { checkRow(it) } } catch (_: android.database.sqlite.SQLiteException) { rejected = true }
        check(rejected) { "wrong key unexpectedly accepted" }; event("wrong_key_rejected")
        open(false).use { checkRow(it) }; event("correct_key_still_opens")
    }
    private fun platformCurves() {
        for (algorithm in listOf("Ed25519", "XDH")) {
            val testAlias = "mc005-capability-$algorithm"
            try {
                val purpose = if (algorithm == "Ed25519") KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY
                    else if (Build.VERSION.SDK_INT >= 31) KeyProperties.PURPOSE_AGREE_KEY else { event("platform_X25519=api_unavailable"); continue }
                val generator = KeyPairGenerator.getInstance(algorithm, "AndroidKeyStore")
                val builder = KeyGenParameterSpec.Builder(testAlias, purpose)
                builder.setAlgorithmParameterSpec(ECGenParameterSpec(if (algorithm == "Ed25519") "ed25519" else "x25519"))
                if (algorithm == "Ed25519") builder.setDigests(KeyProperties.DIGEST_NONE)
                generator.initialize(builder.build()); val pair = generator.generateKeyPair()
                val info = KeyFactory.getInstance(algorithm, "AndroidKeyStore").getKeySpec(pair.private, KeyInfo::class.java)
                @Suppress("DEPRECATION")
                val hardware = info.isInsideSecureHardware
                if (algorithm == "Ed25519") {
                    val msg = "MC005 synthetic challenge".toByteArray(Charsets.UTF_8)
                    val signer = Signature.getInstance("Ed25519"); signer.initSign(pair.private); signer.update(msg)
                    check(probeVerify(rawPublic(pair.public.encoded, 112), msg, signer.sign()))
                } else {
                    val bytes = probeRandomMaterial(); val peerSeed = bytes.copyOfRange(32, 64); bytes.fill(0)
                    try {
                        val peer = KeyFactory.getInstance("XDH").generatePublic(X509EncodedKeySpec(spki(110) + probeAgreementPublic(peerSeed)))
                        val agreement = KeyAgreement.getInstance("XDH"); agreement.init(pair.private); agreement.doPhase(peer, true)
                        val shared = agreement.generateSecret()
                        try { check(probeAgreementMatches(peerSeed, rawPublic(pair.public.encoded, 110), shared)) }
                        finally { shared.fill(0) }
                    } finally { peerSeed.fill(0) }
                }
                event("platform_$algorithm operation_interop_ok nonexportable=${pair.private.encoded == null} hardware=$hardware")
            } catch (e: Exception) { event("platform_$algorithm unavailable_or_failed=${e.javaClass.simpleName}") }
            finally { store.deleteEntry(testAlias) }
        }
    }
    // RFC 8410 fixed 32-byte public-key encodings; reject other algorithms/encodings.
    private fun spki(oid: Int) = byteArrayOf(48, 42, 48, 5, 6, 3, 43, 101, oid.toByte(), 3, 33, 0)
    private fun rawPublic(encoded: ByteArray, oid: Int): ByteArray {
        check(encoded.size == 44 && encoded.copyOfRange(0, 12).contentEquals(spki(oid)))
        return encoded.copyOfRange(12, 44)
    }
    private fun nativeCurves(signing: ByteArray, agreement: ByteArray, msg: ByteArray) {
        try {
            val encoded = byteArrayOf(48, 46, 2, 1, 0, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32) + signing
            val key = try { KeyFactory.getInstance("Ed25519").generatePrivate(PKCS8EncodedKeySpec(encoded)) }
                finally { encoded.fill(0) }
            val signer = Signature.getInstance("Ed25519"); signer.initSign(key); signer.update(msg)
            check(probeVerify(probeSigningPublic(signing), msg, signer.sign()))
            val public = KeyFactory.getInstance("Ed25519").generatePublic(X509EncodedKeySpec(spki(112) + probeSigningPublic(signing)))
            signer.initVerify(public); signer.update(msg); check(signer.verify(probeSign(signing, msg)))
            event("native_software_Ed25519_interop_ok; provider=${signer.provider.name}; process_memory_keys=true")
        } catch (e: Exception) { event("native_software_Ed25519_unavailable_or_failed=${e.javaClass.simpleName}") }
        if (Build.VERSION.SDK_INT < 33) { event("native_software_X25519=api_unavailable"); return }
        try {
            val generator = KeyPairGenerator.getInstance("XDH"); generator.initialize(NamedParameterSpec.X25519)
            val pair = generator.generateKeyPair()
            val public = KeyFactory.getInstance("XDH").generatePublic(X509EncodedKeySpec(spki(110) + probeAgreementPublic(agreement)))
            val native = KeyAgreement.getInstance("XDH"); native.init(pair.private); native.doPhase(public, true)
            val shared = native.generateSecret()
            try { check(probeAgreementMatches(agreement, rawPublic(pair.public.encoded, 110), shared)) }
            finally { shared.fill(0) }
            event("native_software_X25519_interop_ok; provider=${native.provider.name}; process_memory_keys=true")
        } catch (e: Exception) { event("native_software_X25519_unavailable_or_failed=${e.javaClass.simpleName}") }
    }
    private fun reset() {
        held?.close(); held = null; store.deleteEntry(alias)
        // Fixed app-private fixture directory; no externally supplied deletion path.
        if (folder.exists()) check(folder.deleteRecursively())
        event("synthetic_fixture_reset")
    }
    override fun onPause() { super.onPause(); event("activity_paused; held_database_may_retain_key; suspension_unconfirmed") }
    override fun onDestroy() { worker.execute { held?.close(); held = null }; worker.shutdown(); super.onDestroy() }
}
