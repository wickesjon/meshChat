package org.meshchat.storage

import android.app.Activity
import android.app.Instrumentation
import android.os.Bundle
import java.io.File
import java.security.KeyStore
import org.meshchat.identity.*
import uniffi.meshchat_core.*

/** Synthetic instrumentation only; runner script selects an isolated emulator. */
class StorageInstrumentation : Instrumentation() {
    private var phase=""
    private var stage="start"
    override fun onCreate(arguments:Bundle?) {super.onCreate(arguments);phase=arguments?.getString("phase")?:"";start()}
    private fun refused(work:()->Unit) {var failed=false;try{work()}catch(_:Exception){failed=true};check(failed)}
    private fun contains(bytes:ByteArray,marker:ByteArray):Boolean = marker.isNotEmpty() && bytes.asList().windowed(marker.size).any{it==marker.asList()}
    override fun onStart() {
        val result=Bundle()
        try {
            val context=targetContext
            val vault=StorageVault.android(context)
            val identity=IdentityProvider.android(context,vault)
            val store=EncryptedStorage(identity,vault)
            val folder=File(context.noBackupFilesDir,"meshchat-storage-v1")
            val marker="MC018 synthetic plaintext marker".toByteArray()
            val peer=ByteArray(64){(it+1).toByte()};val channel=byteArrayOf(1,2,3,4)
            val now=System.currentTimeMillis()/1000
            fun item(direct:Boolean,id:Byte)=HistoryItem(if(direct)peer else channel,direct,0u,1u,ByteArray(8){id},now,marker,if(direct)byteArrayOf(1,2,3)else byteArrayOf())
            val subject=byteArrayOf(2)+peer
            fun key():ByteArray {
                val encoded=File(folder,"identity.enc").readBytes()
                return AndroidIdentityProtection(context,"org.meshchat.storage.wrapping.v1").open(encoded.copyOfRange(17,encoded.size),encoded.copyOfRange(0,17))
            }
            when(phase) {
                "create" -> {
                    refused{store.reopen()};val info=identity.create();stage="create-store";store.create()
                    store.putRecord(RecordKind.SETTING,"own-public".toByteArray(),info.identity.signingKey)
                    store.putRecord(RecordKind.FRIEND,peer,marker)
                    store.putRecord(RecordKind.SUBSCRIPTION,channel,byteArrayOf(1))
                    store.putRecord(RecordKind.EVENT_ROOT,ByteArray(32){7},marker)
                    store.putRecord(RecordKind.STAFF_CREDENTIAL,ByteArray(64){8},marker)
                    store.appendUnverified(item(false,1));check(store.acceptAuthenticated(item(true,2),subject,byteArrayOf(3,4))==AcceptResult.ACCEPTED)
                    check(folder.canonicalPath.startsWith(context.noBackupFilesDir.canonicalPath+File.separator))
                    check(context.applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_ALLOW_BACKUP == 0)
                }
                "reopen" -> {
                    store.reopen();check(store.getRecord(RecordKind.SETTING,"own-public".toByteArray())!!.contentEquals(identity.load().identity.signingKey))
                    check(store.history(channel,false,100u).single().body.contentEquals(marker));check(store.history(peer,true,100u).single().body.contentEquals(marker))
                    check(store.getRecord(RecordKind.FRIEND,peer)!!.contentEquals(marker))
                    val passphrase=key()
                    try {
                        for(file in checkNotNull(folder.listFiles()).filter{it.isFile}) {val bytes=file.readBytes();check(!contains(bytes,marker));check(!contains(bytes,passphrase))}
                        val wrong=passphrase.copyOf().also{it[0]=(it[0].toInt() xor 1).toByte()}
                        try{refused{CipherConnection.open(File(folder,"history.db"),wrong,false).use{EncryptedStore.open(it,identity.load().identity.generation,false,now).close()}}}finally{wrong.fill(0)}
                    } finally{passphrase.fill(0)}
                    store.reopen();refused{store.create()}
                }
                "checks" -> {
                    refused{vault.clearIdentityState()}
                    check(store.getRecord(RecordKind.FRIEND,peer)!!.contentEquals(marker))
                    val generation=identity.load().identity.generation
                    store.deleteHistory(peer,true)
                    check(store.acceptAuthenticated(item(true,2),subject,byteArrayOf(3,4))==AcceptResult.REPLAY)
                    check(store.acceptAuthenticated(item(true,2),subject,byteArrayOf(9))==AcceptResult.CONFLICT)
                    check(store.ambiguousTarget(subject,ByteArray(8){2}));check(store.history(peer,true,100u).isEmpty())
                    val passphrase=key()
                    try {CipherConnection.open(File(folder,"history.db"),passphrase,false).use{connection ->
                        stage="journal-encryption"
                        connection.execute("BEGIN IMMEDIATE",emptyList())
                        try {
                            connection.execute("UPDATE records SET value=? WHERE kind=2",listOf(SqlValue.Bytes(marker+byteArrayOf(9))))
                            check(File(folder,"history.db-journal").isFile)
                            for(file in checkNotNull(folder.listFiles()).filter{it.isFile}) {val bytes=file.readBytes();check(!contains(bytes,marker));check(!contains(bytes,passphrase))}
                        } finally {connection.execute("ROLLBACK",emptyList())}
                        stage="atomic-effect"
                        val fault=object:SqlDatabase {
                            var enabled=true
                            override fun execute(sql:String,values:List<SqlValue>){if(enabled&&sql.startsWith("INSERT INTO history"))throw StorageException.Database();connection.execute(sql,values)}
                            override fun query(sql:String,values:List<SqlValue>,limit:UInt)=connection.query(sql,values,limit)
                        }
                        EncryptedStore.open(fault,generation,false,now).use{core->refused{core.acceptAuthenticated(item(true,3),subject,byteArrayOf(5),now)};fault.enabled=false;check(core.acceptAuthenticated(item(true,3),subject,byteArrayOf(5),now)==AcceptResult.ACCEPTED)}
                        stage="migration";connection.execute("DROP TABLE ledger",emptyList());connection.execute("DROP TABLE clock",emptyList());connection.execute("PRAGMA user_version=1",emptyList())
                        val migration=object:SqlDatabase {
                            override fun execute(sql:String,values:List<SqlValue>){if(sql.startsWith("CREATE TABLE clock"))throw StorageException.Database();connection.execute(sql,values)}
                            override fun query(sql:String,values:List<SqlValue>,limit:UInt)=connection.query(sql,values,limit)
                        }
                        refused{EncryptedStore.open(migration,generation,false,now).close()}
                        check((connection.query("PRAGMA user_version",emptyList(),1u).single().cells.single() as SqlValue.Integer).value==1L)
                        EncryptedStore.open(connection,generation,false,now).use{core->
                            check(core.getRecord(RecordKind.FRIEND,peer)!!.contentEquals(marker))
                            stage="pruning";core.prune(now+1000);refused{core.prune(now)};refused{core.prune(null)};core.prune(now+1000)
                            core.prune(now+172801);check(core.history(channel,false,100u).isEmpty());check(core.history(peer,true,100u).isEmpty())
                        }
                    }}finally{passphrase.fill(0)}
                }
                "key-loss" -> {
                    val original=File(folder,"history.db").readBytes()
                    KeyStore.getInstance("AndroidKeyStore").apply{load(null);deleteEntry("org.meshchat.storage.wrapping.v1")}
                    refused{store.reopen()};check(File(folder,"history.db").readBytes().contentEquals(original))
                }
                "reset" -> {
                    val previous=identity.load();val fresh=identity.reset();check(!previous.identity.signingKey.contentEquals(fresh.identity.signingKey));check(!folder.exists())
                    refused{store.reopen()};store.create();check(store.getRecord(RecordKind.FRIEND,peer)==null);check(store.history(peer,true,100u).isEmpty())
                    refused{identity.sign(previous.handle,byteArrayOf(1))}
                }
                else -> error("unknown phase")
            }
            result.putString("mc018","PASS $phase synthetic_functional_only");finish(Activity.RESULT_OK,result)
        } catch(error:Throwable) {result.putString("mc018","FAIL $phase $stage ${error.javaClass.simpleName} sanitized");finish(Activity.RESULT_CANCELED,result)}
    }
}
