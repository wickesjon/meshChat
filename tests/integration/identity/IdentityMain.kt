package org.meshchat.identity

import javax.crypto.Cipher
import javax.crypto.spec.GCMParameterSpec
import javax.crypto.spec.SecretKeySpec

private class Fault { var count=0; var fail=0; fun after() { count++; if(count==fail) throw IllegalStateException("synthetic failure") } }
private class MemoryStorage(private val fault: Fault) : IdentityStorage {
    val files=mutableMapOf<IdentityFile,ByteArray>(); var present=false
    override fun exists(file: IdentityFile)=files.containsKey(file)
    override fun read(file: IdentityFile)=checkNotNull(files[file]).copyOf()
    override fun write(file: IdentityFile,bytes:ByteArray) { present=true;files[file]=bytes.copyOf();fault.after() }
    override fun delete(file: IdentityFile) {files.remove(file);fault.after()}
    override fun hasArtifacts()=present
}
private class TestProtection(private val fault:Fault): IdentityProtection {
    var key:ByteArray?=null;var locked=false;var unavailable=false;var entropyFailure=false;var counter=0
    override fun requireUnlocked() {if(locked) throw IdentityProviderException(IdentityFailure.LOCKED);if(unavailable) throw IdentityProviderException(IdentityFailure.UNAVAILABLE)}
    override fun exists()=key!=null
    override fun create(){check(key==null);key=random(32);fault.after()}
    override fun delete(){key?.fill(0);key=null;fault.after()}
    override fun random(size:Int):ByteArray {if(entropyFailure)throw IdentityProviderException(IdentityFailure.UNAVAILABLE);counter++;return ByteArray(size){(it+counter).toByte()}}
    override fun seal(plain:ByteArray,aad:ByteArray):ByteArray {
        val c=Cipher.getInstance("AES/GCM/NoPadding");val iv=random(12)
        c.init(Cipher.ENCRYPT_MODE,SecretKeySpec(checkNotNull(key),"AES"),GCMParameterSpec(128,iv));c.updateAAD(aad)
        return iv+c.doFinal(plain)
    }
    override fun open(cipher:ByteArray,aad:ByteArray):ByteArray {
        val c=Cipher.getInstance("AES/GCM/NoPadding")
        c.init(Cipher.DECRYPT_MODE,SecretKeySpec(checkNotNull(key),"AES"),GCMParameterSpec(128,cipher.copyOfRange(0,12)));c.updateAAD(aad)
        return c.doFinal(cipher,12,cipher.size-12)
    }
    override fun capabilities()=IdentityCapabilities(WrappingProtection.SOFTWARE)
}
private class Pins(private val fault:Fault):IdentityResetStore {var present=true;var refuse=false;override fun clearIdentityState(){if(refuse)throw IllegalStateException("synthetic reset failure");present=false;fault.after()}}
private class Fixture {
    val fault=Fault();val storage=MemoryStorage(fault);val protection=TestProtection(fault);val pins=Pins(fault)
    fun provider()=IdentityProvider(storage,protection,pins)
}
private fun refused(expected:IdentityFailure,call:()->Unit) {
    try {call();error("expected refusal") } catch(e:IdentityProviderException){check(e.failure==expected){"wrong failure category: ${e.failure}"}}
}
fun main() {
    val f=Fixture();var provider=f.provider()
    refused(IdentityFailure.MISSING){provider.load()};check(f.fault.count==0)
    val first=provider.create();val original=f.storage.read(IdentityFile.ENVELOPE)
    refused(IdentityFailure.RECOVERY_REQUIRED){provider.create()}
    provider=f.provider();val reopened=provider.load()
    check(first.identity.signingKey.contentEquals(reopened.identity.signingKey));check(first.identity.agreementKey.contentEquals(reopened.identity.agreementKey))
    check(provider.sign(first.handle,byteArrayOf(1,2,3)).size==64)
    val peer=Fixture().provider().create()
    check(provider.agree(first.handle,peer.identity.agreementKey).size==32)
    refused(IdentityFailure.INVALID_INPUT){provider.agree(first.handle,ByteArray(32))}
    f.storage.files[IdentityFile.ENVELOPE]=original.copyOf().also { it[0]=2 }
    refused(IdentityFailure.INVALID_INPUT){provider.load()}
    f.storage.files[IdentityFile.ENVELOPE]=original.copyOf().also { it[17]=(it[17].toInt() xor 1).toByte() }
    refused(IdentityFailure.PROVIDER){provider.load()}
    f.storage.files[IdentityFile.ENVELOPE]=original.copyOf()
    f.protection.locked=true;refused(IdentityFailure.LOCKED){provider.sign(first.handle,byteArrayOf(1))};f.protection.locked=false
    f.protection.unavailable=true;refused(IdentityFailure.UNAVAILABLE){provider.load()};f.protection.unavailable=false
    f.protection.key=null;refused(IdentityFailure.INVALIDATED){provider.load()};check(f.storage.read(IdentityFile.ENVELOPE).contentEquals(original))
    val rotated=provider.reset();check(!f.pins.present)
    check(!rotated.identity.signingKey.contentEquals(first.identity.signingKey));check(!rotated.identity.agreementKey.contentEquals(first.identity.agreementKey))
    refused(IdentityFailure.STALE_HANDLE){provider.sign(first.handle,byteArrayOf(1))}
    f.pins.present=true;f.pins.refuse=true;refused(IdentityFailure.PROVIDER){provider.reset()}
    refused(IdentityFailure.RECOVERY_REQUIRED){provider.load()};check(f.protection.exists())
    f.pins.refuse=false;provider.reset();check(!f.pins.present)
    // Simulate a crash immediately after each durable mutation in reset, then
    // reconstruct the provider. Any successful load must be fully reset state.
    for(step in 1..7) {
        val x=Fixture();val old=x.provider().create();x.fault.count=0;x.fault.fail=step
        try{x.provider().reset()}catch(_:IdentityProviderException){}
        x.fault.fail=0
        try {val after=x.provider().load();check(!x.pins.present);check(!after.identity.signingKey.contentEquals(old.identity.signingKey))}
        catch(e:IdentityProviderException){check(e.failure==IdentityFailure.RECOVERY_REQUIRED)}
        val recovered=x.provider().reset();check(!x.pins.present);check(!recovered.identity.signingKey.contentEquals(old.identity.signingKey))
        refused(IdentityFailure.STALE_HANDLE){x.provider().sign(old.handle,byteArrayOf(1))}
    }
    for(step in 1..4) {
        val x=Fixture();x.fault.fail=step
        try{x.provider().create()}catch(_:IdentityProviderException){}
        x.fault.fail=0
        try{x.provider().load()}catch(e:IdentityProviderException){check(e.failure==IdentityFailure.RECOVERY_REQUIRED)}
        x.provider().reset();check(!x.pins.present)
    }
    val entropy=Fixture();entropy.protection.entropyFailure=true
    refused(IdentityFailure.UNAVAILABLE){entropy.provider().create()}
    refused(IdentityFailure.RECOVERY_REQUIRED){entropy.provider().load()}
    println("MC-017 Kotlin: provisioning/reopen/sign/agree/lock/key-loss/reset and 7 recovery faults PASS; injected wrapping/storage only")
}
