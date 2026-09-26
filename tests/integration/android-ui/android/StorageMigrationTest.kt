package org.meshchat.storage

import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.security.SecureRandom
import java.util.UUID
import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec
import org.junit.Assert.*
import org.junit.Test
import org.meshchat.identity.*
import uniffi.meshchat_core.*

/** Only fresh synthetic stores, real pinned SQLCipher and production AtomicFile.
 * Fault-injected wrapping is not real-device Keystore/lock certification. */
class StorageMigrationTest {
    @Test fun platformWrappingKeyLossAndIdentityReset() {
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        val name="mc025-reset-${UUID.randomUUID()}"
        val identityName="$name-identity"
        val folder=File(context.noBackupFilesDir,name).canonicalFile
        val identityFolder=File(context.noBackupFilesDir,identityName).canonicalFile
        check(listOf(folder,identityFolder).all {it.parentFile==context.noBackupFilesDir.canonicalFile && !it.exists()})
        val protection=AndroidIdentityProtection(context,name)
        val identityProtection=AndroidIdentityProtection(context,identityName)
        val vault=StorageVault(folder,AndroidIdentityStorage(context,name),protection)
        val identity=IdentityProvider(AndroidIdentityStorage(context,identityName),identityProtection,vault)
        val store=EncryptedStorage(identity,vault)
        try {
            val before=identity.create();store.create()
            store.putRecord(RecordKind.SETTING,byteArrayOf(1),byteArrayOf(2))
            store.reopen()
            val db=File(folder,"history.db").readBytes()
            protection.delete()
            assertThrows(IdentityProviderException::class.java) {store.reopen()}
            assertArrayEquals(db,File(folder,"history.db").readBytes())
            val after=identity.reset()
            assertFalse(before.identity.generation.contentEquals(after.identity.generation))
            assertFalse(folder.exists())
            store.create();assertNull(store.getRecord(RecordKind.SETTING,byteArrayOf(1)))
            assertThrows(IdentityProviderException::class.java) {identity.sign(before.handle,byteArrayOf(1))}
        } finally {
            protection.delete();identityProtection.delete()
            for(owned in listOf(folder,identityFolder)) if(owned.exists())check(owned.deleteRecursively())
        }
    }
    private class Protection:IdentityProtection {
        var key:ByteArray?=ByteArray(32).also {SecureRandom().nextBytes(it)}
        var locked=false;var failSeal=false;var lockOnSeal=false
        val plains=mutableListOf<ByteArray>()
        override fun requireUnlocked(){if(locked)throw IdentityProviderException(IdentityFailure.LOCKED)}
        override fun exists()=key!=null
        override fun create(){key=random(32)}
        override fun delete(){key?.fill(0);key=null}
        override fun random(size:Int)=ByteArray(size).also {SecureRandom().nextBytes(it)}
        override fun seal(plain:ByteArray,aad:ByteArray):ByteArray {
            requireUnlocked();plains.add(plain)
            if(failSeal)error("synthetic seal failure")
            val iv=random(12);val c=Cipher.getInstance("AES/GCM/NoPadding")
            c.init(Cipher.ENCRYPT_MODE,SecretKeySpec(checkNotNull(key),"AES"),GCMParameterSpec(128,iv));c.updateAAD(aad)
            return (iv+c.doFinal(plain)).also {if(lockOnSeal)locked=true}
        }
        override fun open(cipher:ByteArray,aad:ByteArray):ByteArray {
            requireUnlocked();val c=Cipher.getInstance("AES/GCM/NoPadding")
            c.init(Cipher.DECRYPT_MODE,SecretKeySpec(checkNotNull(key),"AES"),GCMParameterSpec(128,cipher.copyOfRange(0,12)));c.updateAAD(aad)
            return c.doFinal(cipher,12,cipher.size-12).also {plains.add(it)}
        }
        override fun capabilities()=IdentityCapabilities(WrappingProtection.SOFTWARE)
    }
    private class Files(private val delegate:IdentityStorage):IdentityStorage by delegate {
        var fault=0
        override fun write(file:IdentityFile,bytes:ByteArray) {
            if(fault==1)error("synthetic failure before commit")
            delegate.write(file,bytes)
            if(fault==2)error("synthetic failure after commit")
        }
    }
    @Test fun legacyMigrationAndAtomicCommitFailuresPreserveHistory() {
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        for(fault in 0..4) {
            val name="mc025-migration-${UUID.randomUUID()}"
            val folder=File(context.noBackupFilesDir,name).canonicalFile
            check(folder.parentFile==context.noBackupFilesDir.canonicalFile && !folder.exists())
            val files=Files(AndroidIdentityStorage(context,name));val protection=Protection()
            val generation=ByteArray(16){7};val pass=ByteArray(64){(it*5).toByte()}
            val record="migration".toByteArray();val marker="synthetic preserved history".toByteArray()
            try {
                val header=byteArrayOf(1)+generation
                files.write(IdentityFile.ENVELOPE,header+protection.seal(pass,header))
                val db=File(folder,"history.db")
                CipherConnection.open(db,pass,true).use {connection ->
                    EncryptedStore.open(connection,generation,true,null).use {it.putRecord(RecordKind.SETTING,record,marker)}
                }
                val before=db.readBytes();val oldEnvelope=files.read(IdentityFile.ENVELOPE)
                pass.fill(0)
                val vault=StorageVault(folder,files,protection)
                assertThrows(IdentityProviderException::class.java) {vault.access(ByteArray(16){8},false,null){}}
                files.fault=if(fault<=2)fault else 0
                protection.failSeal=fault==3;protection.lockOnSeal=fault==4
                if(fault!=0) {
                    assertThrows(Exception::class.java) {vault.access(generation,false,null){}}
                    assertEquals(if(fault==2)2 else 1,files.read(IdentityFile.ENVELOPE)[0].toInt())
                    if(fault!=2)assertArrayEquals(oldEnvelope,files.read(IdentityFile.ENVELOPE))
                    assertTrue(protection.plains.all {p->p.all {it==0.toByte()}})
                    files.fault=0;protection.failSeal=false;protection.lockOnSeal=false;protection.locked=false
                }
                repeat(2) {vault.access(generation,false,null){assertArrayEquals(marker,it.getRecord(RecordKind.SETTING,record))}}
                assertEquals(2,files.read(IdentityFile.ENVELOPE)[0].toInt())
                assertArrayEquals("Migration must not rewrite database",before,db.readBytes())
                assertTrue(protection.plains.all {p->p.all {it==0.toByte()}})
                protection.locked=true
                assertThrows(IdentityProviderException::class.java) {vault.access(generation,false,null){}}
                protection.locked=false;protection.delete()
                assertThrows(IdentityProviderException::class.java) {vault.access(generation,false,null){}}
                assertArrayEquals(before,db.readBytes())
            } finally {pass.fill(0);protection.delete();check(folder.deleteRecursively())}
        }
    }

    @Test fun newRawKeyStoreReopensAndRejectsUnknownEnvelope() {
        val context=InstrumentationRegistry.getInstrumentation().targetContext
        val name="mc025-fresh-${UUID.randomUUID()}"
        val folder=File(context.noBackupFilesDir,name).canonicalFile
        check(folder.parentFile==context.noBackupFilesDir.canonicalFile && !folder.exists())
        val files=Files(AndroidIdentityStorage(context,name));val protection=Protection().also {it.delete()}
        val vault=StorageVault(folder,files,protection);val generation=ByteArray(16){4}
        try {
            vault.access(generation,true,null) {it.putRecord(RecordKind.SETTING,byteArrayOf(1),byteArrayOf(2))}
            vault.access(generation,false,null) {assertArrayEquals(byteArrayOf(2),it.getRecord(RecordKind.SETTING,byteArrayOf(1)))}
            assertEquals(2,files.read(IdentityFile.ENVELOPE)[0].toInt())
            val encoded=files.read(IdentityFile.ENVELOPE)
            val material=protection.open(encoded.copyOfRange(17,encoded.size),encoded.copyOfRange(0,17))
            try {
                val wrong=StorageKeys.rawInput(material).also {it[2]=if(it[2]==48.toByte())49 else 48}
                try {assertThrows(StorageException::class.java) {CipherConnection.open(File(folder,"history.db"),wrong,false).use {}}}
                finally {wrong.fill(0)}
            } finally {material.fill(0)}
            vault.access(generation,false,null) {assertArrayEquals(byteArrayOf(2),it.getRecord(RecordKind.SETTING,byteArrayOf(1)))}
            val bad=files.read(IdentityFile.ENVELOPE).also {it[0]=3}
            files.write(IdentityFile.ENVELOPE,bad)
            assertThrows(IdentityProviderException::class.java) {vault.access(generation,false,null){}}
            assertThrows(IdentityProviderException::class.java) {vault.clearIdentityState()}
            assertTrue(protection.plains.all {p->p.all {it==0.toByte()}})
        } finally {protection.delete();check(folder.deleteRecursively())}
    }
}
