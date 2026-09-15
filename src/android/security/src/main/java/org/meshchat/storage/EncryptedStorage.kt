package org.meshchat.storage

import android.content.Context
import android.database.Cursor
import android.system.Os
import android.system.OsConstants
import androidx.sqlite.db.SimpleSQLiteQuery
import java.io.File
import net.zetetic.database.sqlcipher.SQLiteDatabase
import org.meshchat.identity.*
import uniffi.meshchat_core.*

/** A connection never escapes an unlocked operation; no exception text is logged. */
internal class CipherConnection(private val database: SQLiteDatabase) : SqlDatabase, AutoCloseable {
    private fun arguments(values: List<SqlValue>): Array<Any> = values.map {
        when(it) { is SqlValue.Integer -> it.value; is SqlValue.Bytes -> it.value; is SqlValue.Text -> it.value }
    }.toTypedArray()
    override fun execute(sql: String, values: List<SqlValue>) {
        try { database.execSQL(sql, arguments(values)) }
        catch (_: Exception) { throw StorageException.Database() }
    }
    override fun query(sql: String, values: List<SqlValue>, limit: UInt): List<SqlRow> {
        if (limit > 100u) throw StorageException.InvalidInput()
        try {
            database.query(SimpleSQLiteQuery(sql, arguments(values))).use { cursor ->
                val rows = mutableListOf<SqlRow>()
                while(cursor.moveToNext()) {
                    if(rows.size >= limit.toInt() || cursor.columnCount > 12) throw StorageException.Database()
                    val cells = (0 until cursor.columnCount).map { column -> when(cursor.getType(column)) {
                        Cursor.FIELD_TYPE_INTEGER -> SqlValue.Integer(cursor.getLong(column))
                        Cursor.FIELD_TYPE_BLOB -> SqlValue.Bytes(cursor.getBlob(column).also { if(it.size>4096) throw StorageException.Database() })
                        Cursor.FIELD_TYPE_STRING -> SqlValue.Text(cursor.getString(column).also { if(it.length>4096) throw StorageException.Database() })
                        else -> throw StorageException.Database()
                    } }
                    rows.add(SqlRow(cells))
                }
                return rows
            }
        } catch (_: Exception) { throw StorageException.Database() }
    }
    override fun close() { database.close() }
    companion object {
        fun open(file: File, key: ByteArray, create: Boolean): CipherConnection {
            System.loadLibrary("sqlcipher")
            val flags = SQLiteDatabase.OPEN_READWRITE or (if(create) SQLiteDatabase.CREATE_IF_NECESSARY else 0)
            val db = try { SQLiteDatabase.openDatabase(file.path, key, null, flags, null, null) }
                catch (_: Exception) { throw StorageException.Database() }
            try {
                val result = CipherConnection(db)
                result.query("PRAGMA journal_mode=DELETE", emptyList(), 1u)
                result.execute("PRAGMA synchronous=FULL", emptyList())
                return result
            } catch(e: Exception) { db.close(); throw e }
        }
    }
}

/** Construct before IdentityProvider and pass it as the mandatory reset callback.
 * All operations share the identity lock; no DB/key handle survives an operation. */
class StorageVault internal constructor(
    private val folder: File,
    private val files: IdentityStorage,
    private val protection: IdentityProtection,
) : IdentityResetStore {
    companion object {
        fun android(context: Context): StorageVault {
            val app=context.applicationContext
            return StorageVault(File(app.noBackupFilesDir,"meshchat-storage-v1"), AndroidIdentityStorage(app,"meshchat-storage-v1"),
                AndroidIdentityProtection(app,"org.meshchat.storage.wrapping.v1"))
        }
    }
    private fun sync(directory:File) {val fd=Os.open(directory.path,OsConstants.O_RDONLY,0);try{Os.fsync(fd)}finally{Os.close(fd)}}
    private fun refuse(failure:IdentityFailure):Nothing = throw IdentityProviderException(failure)
    private val database get()=File(folder,"history.db")
    internal fun <T> access(generation:ByteArray,create:Boolean,now:Long?,work:(EncryptedStore)->T):T = IdentityProvider.withOperation {
        protection.requireUnlocked()
        if(generation.size!=16) refuse(IdentityFailure.INVALID_INPUT)
        if(create) {
            if(files.hasArtifacts()||protection.exists()) refuse(IdentityFailure.RECOVERY_REQUIRED)
            files.write(IdentityFile.JOURNAL,byteArrayOf(1))
            protection.create()
            val key=protection.random(64)
            try {val header=byteArrayOf(1)+generation; files.write(IdentityFile.ENVELOPE,header+protection.seal(key,header))}
            finally{key.fill(0)}
        } else {
            if(files.exists(IdentityFile.JOURNAL)) refuse(IdentityFailure.RECOVERY_REQUIRED)
            if(!files.exists(IdentityFile.ENVELOPE)||!database.isFile) refuse(if(files.hasArtifacts()||protection.exists()) IdentityFailure.RECOVERY_REQUIRED else IdentityFailure.MISSING)
        }
        if(!protection.exists()) refuse(IdentityFailure.INVALIDATED)
        val encoded=files.read(IdentityFile.ENVELOPE)
        if(encoded.size !in 18..1024||encoded[0]!=1.toByte()||!encoded.copyOfRange(1,17).contentEquals(generation)) refuse(IdentityFailure.INVALID_INPUT)
        val key=protection.open(encoded.copyOfRange(17,encoded.size),encoded.copyOfRange(0,17))
        try {
            if(key.size!=64) refuse(IdentityFailure.INVALID_INPUT)
            val result=CipherConnection.open(database,key,create).use { connection ->
                val core=EncryptedStore.open(connection,generation,create,now)
                try { work(core) } finally {core.close()}
            }
            protection.requireUnlocked()
            if(create){sync(folder);files.delete(IdentityFile.JOURNAL)}
            result
        } finally{key.fill(0)}
    }
    override fun clearIdentityState() = IdentityProvider.withOperation {
        protection.requireUnlocked()
        if(files.hasArtifacts()||protection.exists()) {
            files.write(IdentityFile.JOURNAL,byteArrayOf(2))
            protection.delete()
            for(name in listOf("history.db","history.db-wal","history.db-shm","history.db-journal")) {
                val file=File(folder,name);if(file.exists()&&!file.delete()) refuse(IdentityFailure.RECOVERY_REQUIRED)
            }
            files.delete(IdentityFile.ENVELOPE);files.delete(IdentityFile.JOURNAL)
            if(folder.exists()&&!folder.delete()) refuse(IdentityFailure.RECOVERY_REQUIRED)
            sync(checkNotNull(folder.parentFile))
        }
    }
}

class EncryptedStorage(private val identity:IdentityProvider,private val vault:StorageVault) {
    private fun <T> operation(create:Boolean=false,now:Long?=System.currentTimeMillis()/1000,work:(EncryptedStore)->T):T = IdentityProvider.withOperation {
        val generation=identity.load().identity.generation
        vault.access(generation,create,now,work)
    }
    fun create() {operation(create=true){}}
    fun reopen() {operation{}}
    fun putRecord(kind:RecordKind,key:ByteArray,value:ByteArray) {operation{it.putRecord(kind,key,value)}}
    fun getRecord(kind:RecordKind,key:ByteArray):ByteArray? = operation{it.getRecord(kind,key)}
    fun deleteRecord(kind:RecordKind,key:ByteArray) {operation{it.deleteRecord(kind,key)}}
    fun appendUnverified(item:HistoryItem) {operation{it.appendUnverified(item,System.currentTimeMillis()/1000)}}
    fun acceptAuthenticated(item:HistoryItem,subject:ByteArray,immutableBytes:ByteArray):AcceptResult = operation{it.acceptAuthenticated(item,subject,immutableBytes,System.currentTimeMillis()/1000)}
    fun history(conversation:ByteArray,direct:Boolean,limit:UInt):List<HistoryItem> = operation{it.history(conversation,direct,limit)}
    fun deleteHistory(conversation:ByteArray,direct:Boolean) {operation{it.deleteHistory(conversation,direct)}}
    fun ambiguousTarget(subject:ByteArray,messageId:ByteArray):Boolean = operation{it.ambiguousTarget(subject,messageId)}
    fun prune(){operation{it.prune(System.currentTimeMillis()/1000)}}
}
