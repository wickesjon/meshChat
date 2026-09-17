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
        IdentityProvider.requireResetInProgress()
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

class EncryptedStorage(private val identity:IdentityProvider,private val vault:StorageVault,private val staff:StaffKeyVault?=null) {
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
    /** Only public identity/friend state escapes construction; the database and
     * wrapping key are closed/cleared before returning to the feature owner. */
    fun transport(instance: ULong, now: ULong): NativeTransport = operation { store ->
        NativeTransport(store, identity.load().identity, instance, now)
    }
    fun channelAccept(owner: NativeChannels, link: LinkHandle?, bytes: ByteArray, intake: TransportIntake,
        own: Boolean, now: ULong): Boolean = operation { store ->
        owner.accept(store, ChannelReceipt(link, bytes, intake, own, System.currentTimeMillis()/1000, now))
    }
    fun channelHistory(owner: NativeChannels, name: String, nickname: String): List<ChannelMessage> =
        operation { owner.history(it, name, nickname) }

    fun friendCards(core: NativeTransport, now: ULong): List<FriendCard> = operation { core.friendCards(it, now) }
    fun friendCode(nickname: String): FriendProposal = friendCode(identity.load().identity, nickname)
    fun confirmFriend(core: NativeTransport, uri: String, petname: String, previous: FriendHandle?, now: ULong): TransportEffects =
        operation { store -> identity.messaging { core.confirmFriend(store, it, uri, petname, previous, now) } }
    fun changeFriend(core: NativeTransport, friend: FriendHandle, replace: Boolean, now: ULong): TransportEffects =
        operation { core.changeFriend(it, friend, replace, now) }
    fun authenticate(core: NativeTransport, link: LinkHandle, bytes: ByteArray, now: ULong): AuthenticationResult =
        operation { store -> identity.messaging { core.authenticateMessage(store, it, link, bytes, now, System.currentTimeMillis()/1000) } }
    fun retryMessages(core: NativeTransport, now: ULong): Boolean =
        operation { store -> identity.messaging { core.retryMessages(store, it, now, System.currentTimeMillis()/1000) } }
    fun sendDirect(core: NativeTransport, friend: FriendHandle, content: DirectContent, cookie: ULong, now: ULong): MessageSubmission =
        operation { store -> identity.messaging { core.sendDirect(store, it, friend, content, cookie, now, System.currentTimeMillis()/1000) } }
    fun sendSigned(core: NativeTransport, bytes: ByteArray, cookie: ULong, now: ULong): MessageSubmission =
        operation { store -> identity.messaging { core.sendSigned(store, it, bytes, cookie, now, System.currentTimeMillis()/1000) } }
    fun directHistory(core: NativeTransport, keys: ByteArray, now: ULong): List<DirectMessage> =
        operation { core.directHistory(it, keys, now, System.currentTimeMillis()/1000) }
    fun messagingHistory(core: NativeTransport, owner: NativeChannels, name: String, nickname: String): List<ChannelMessage> =
        operation { core.messagingChannelHistory(it, owner, name, nickname) }
    fun proof(core: NativeTransport, link: LinkHandle, now: ULong): TransportEffects =
        IdentityProvider.withOperation { identity.messaging { core.prepareProof(link, it, now) } }
    /** Keep reset serialization through the actual native submission. The
     * database/session are closed before returning; callbacks never hold keys. */
    fun messageEgress(core: NativeTransport, send: TransportSend, submit: () -> Boolean): Boolean = IdentityProvider.withOperation {
        if (core.messageNeedsAuthorization(send.link, send.token)) {
            val organizer=operation {
                val guarded=core.authorizeOrganizerEgress(it,send.link,send.token,System.currentTimeMillis()/1000)
                if(!guarded)core.authorizeMessageEgress(it, send.link, send.token)
                guarded
            }
            if(organizer)return@withOperation staff?.submitForeground(submit) ?: false
        }
        submit()
    }

    fun events(core: NativeTransport): List<EventCard> = operation { core.eventCards(it,System.currentTimeMillis()/1000) }
    fun adoptEvent(core: NativeTransport,uri:String,now:ULong) = operation { core.confirmEvent(it,uri,now,System.currentTimeMillis()/1000) }
    fun removeEvent(core:NativeTransport,key:ByteArray) = operation { core.removeEvent(it,key,System.currentTimeMillis()/1000) }
    fun eventMessages(core:NativeTransport,now:ULong):List<EventMessage> = operation { core.eventMessages(it,System.currentTimeMillis()/1000,now) }
    fun importStaff(core:NativeTransport,candidate:ByteArray,now:ULong) = operation { store ->
        checkNotNull(staff).importConfirmed(identity.load().identity.generation,candidate) { uri ->
            core.importStaffKey(store,uri,now,System.currentTimeMillis()/1000).use { it.invalidate() }
        }
    }
    fun staffCard():StaffCard? {
        val owner=staff ?: return null
        if(!owner.present())return null
        return IdentityProvider.withOperation { owner.access(identity.load().identity.generation,::staffProposal) }
    }
    fun forgetStaff(core:NativeTransport) = IdentityProvider.withOperation {
        core.forgetStaffOperations();checkNotNull(staff).forget()
    }
    fun organizerTick(core:NativeTransport,now:ULong):TransportEffects = operation {
        val wall=System.currentTimeMillis()/1000
        val card=try {if(staff?.signingAllowed()==true)staffCard() else null} catch (_:IdentityProviderException) {null}
        core.organizerTick(it,card?.takeIf {c->wall in c.notBefore.toLong()..c.notAfter.toLong()}?.credential,now,wall)
    }
    fun postEvent(core:NativeTransport,nickname:String,text:String,avatar:UByte,pin:UInt?,cookie:ULong,now:ULong):MessageSubmission = operation { store ->
        checkNotNull(staff).access(identity.load().identity.generation) { uri ->
            core.importStaffKey(store,uri,now,System.currentTimeMillis()/1000).use { session ->
                try { core.postEvent(store,session,nickname,text,avatar,pin,cookie,now,System.currentTimeMillis()/1000) }
                finally {session.invalidate()}
            }
        }
    }

    /** Bounded encrypted navigation index for retained old-identity histories.
     * Persist before pin mutation; a failed mutation can leave a harmless duplicate.
     * Eight slots hold eight canonical public-key codes each, below 2 KiB/record. */
    fun archives(): List<FriendProposal> = operation { store -> (0 until 8).flatMap { slot ->
        store.getRecord(RecordKind.SETTING, "ui-old-friends-$slot".toByteArray())?.toString(Charsets.UTF_8)
            ?.split('\n')?.filter { it.isNotEmpty() }?.map(::friendProposal) ?: emptyList()
    } }
    fun archiveFriend(friend: FriendCard): Boolean = operation { store ->
        val slots=(0 until 8).map { slot ->
            store.getRecord(RecordKind.SETTING, "ui-old-friends-$slot".toByteArray())?.toString(Charsets.UTF_8)
                ?.split('\n')?.filter { it.isNotEmpty() }?.toMutableList() ?: mutableListOf()
        }
        if (slots.flatten().any { friendProposal(it).keys.contentEquals(friend.keys) }) true else {
            val slot=slots.indexOfFirst { it.size<8 }
            if(slot<0) false else {
                val metadata=PublicIdentity(byteArrayOf(),friend.keys.copyOfRange(0,32),friend.keys.copyOfRange(32,64),byteArrayOf(),byteArrayOf())
                slots[slot].add(friendCode(metadata,friend.petname).uri)
                store.putRecord(RecordKind.SETTING,"ui-old-friends-$slot".toByteArray(),slots[slot].joinToString("\n").toByteArray());true
            }
        }
    }
    fun deleteArchive(keys: ByteArray) = operation { store ->
        store.deleteHistory(keys,true)
        for(slot in 0 until 8) {
            val name="ui-old-friends-$slot".toByteArray()
            val old=store.getRecord(RecordKind.SETTING,name)?.toString(Charsets.UTF_8) ?: continue
            val kept=old.split('\n').filter { it.isNotEmpty() && !friendProposal(it).keys.contentEquals(keys) }
            store.putRecord(RecordKind.SETTING,name,kept.joinToString("\n").toByteArray())
        }
    }
}
