import Foundation
import CryptoKit

@MainActor private final class Fault { var count = 0; var fail = 0; func after() throws { count += 1; if count == fail { throw IdentityFailure.provider } } }
@MainActor private final class MemoryStorage: IdentityStorage {
    let fault: Fault; var files: [IdentityFile: Data] = [:]; var present = false
    init(_ fault: Fault) { self.fault = fault }
    func exists(_ file: IdentityFile) -> Bool { files[file] != nil }
    func read(_ file: IdentityFile) throws -> Data { guard let data = files[file] else { throw IdentityFailure.missing }; return data }
    func write(_ file: IdentityFile, _ bytes: Data) throws { present = true; files[file] = bytes; try fault.after() }
    func delete(_ file: IdentityFile) throws { files.removeValue(forKey: file); try fault.after() }
    func hasArtifacts() -> Bool { present }
}
@MainActor private final class TestProtection: IdentityProtection {
    let fault: Fault; var key: SymmetricKey?; var locked = false; var unavailable = false; var entropyFailure = false; var counter = 0
    init(_ fault: Fault) { self.fault = fault }
    func requireUnlocked() throws { if locked { throw IdentityFailure.locked }; if unavailable { throw IdentityFailure.unavailable } }
    func exists() throws -> Bool { key != nil }
    func create() throws { precondition(key == nil); key = SymmetricKey(data: try random(32)); try fault.after() }
    func delete() throws { key = nil; try fault.after() }
    func random(_ size: Int) throws -> Data {
        if entropyFailure { throw IdentityFailure.unavailable }; counter += 1
        return Data((0..<size).map { UInt8(truncatingIfNeeded: $0 + counter) })
    }
    func seal(_ plain: Data, aad: Data) throws -> Data {
        guard let key else { throw IdentityFailure.invalidated }
        return try AES.GCM.seal(plain, using: key, nonce: AES.GCM.Nonce(data: random(12)), authenticating: aad).combined!
    }
    func open(_ cipher: Data, aad: Data) throws -> Data {
        guard let key else { throw IdentityFailure.invalidated }
        return try AES.GCM.open(AES.GCM.SealedBox(combined: cipher), using: key, authenticating: aad)
    }
    func capabilities() throws -> IdentityCapabilities { IdentityCapabilities(wrapping: .software) }
}
@MainActor private final class Pins: IdentityResetStore {
    let fault: Fault; var present = true; var refuse = false
    init(_ fault: Fault) { self.fault = fault }
    func clearIdentityState() throws { if refuse { throw IdentityFailure.provider }; present = false; try fault.after() }
}
@MainActor private final class Fixture {
    let fault: Fault; let storage: MemoryStorage; let protection: TestProtection; let pins: Pins
    init() { let fault = Fault(); self.fault = fault; storage = MemoryStorage(fault); protection = TestProtection(fault); pins = Pins(fault) }
    func provider() -> IdentityProvider { IdentityProvider(storage: storage, protection: protection, state: pins) }
}
@MainActor private func refused<T>(_ expected: IdentityFailure, _ work: () throws -> T) {
    do { _ = try work(); fatalError("expected refusal") }
    catch let error as IdentityFailure { precondition(error == expected) }
    catch { fatalError("unexpected failure type") }
}
@main private struct IdentityChecks {
    @MainActor static func main() throws {
        let f = Fixture(); var provider = f.provider()
        refused(.missing) { try provider.load() }; precondition(f.fault.count == 0)
        let first = try provider.create(); let original = try f.storage.read(.envelope)
        refused(.recoveryRequired) { try provider.create() }
        provider = f.provider(); let reopened = try provider.load()
        precondition(first.identity.signingKey == reopened.identity.signingKey && first.identity.agreementKey == reopened.identity.agreementKey)
        let transcript = Data([1, 2, 3]); let signature = try provider.sign(first.handle, transcript: transcript)
        let verifier = try Curve25519.Signing.PublicKey(rawRepresentation: first.identity.signingKey)
        precondition(verifier.isValidSignature(signature, for: transcript))
        let other = Fixture(); other.protection.counter = 33; let peer = try other.provider().create()
        let shared = try provider.agree(first.handle, peer: peer.identity.agreementKey)
        let peerShared = try other.provider().agree(peer.handle, peer: first.identity.agreementKey)
        precondition(shared == peerShared)
        refused(.invalidInput) { try provider.agree(first.handle, peer: Data(repeating: 0, count: 32)) }
        var malformed = original; malformed[0] = 2; f.storage.files[.envelope] = malformed
        refused(.invalidInput) { try provider.load() }
        malformed = original; malformed[17] ^= 1; f.storage.files[.envelope] = malformed
        refused(.provider) { try provider.load() }
        f.storage.files[.envelope] = original
        f.protection.locked = true; refused(.locked) { try provider.sign(first.handle, transcript: transcript) }; f.protection.locked = false
        f.protection.unavailable = true; refused(.unavailable) { try provider.load() }; f.protection.unavailable = false
        f.protection.key = nil; refused(.invalidated) { try provider.load() }
        let retainedEnvelope = try f.storage.read(.envelope); precondition(retainedEnvelope == original)
        let rotated = try provider.reset(); precondition(!f.pins.present)
        precondition(rotated.identity.signingKey != first.identity.signingKey && rotated.identity.agreementKey != first.identity.agreementKey)
        refused(.staleHandle) { try provider.sign(first.handle, transcript: transcript) }
        f.pins.present = true; f.pins.refuse = true; refused(.provider) { try provider.reset() }
        refused(.recoveryRequired) { try provider.load() }
        let retainedKey = try f.protection.exists(); precondition(retainedKey)
        f.pins.refuse = false; _ = try provider.reset(); precondition(!f.pins.present)
        for step in 1...7 {
            let x = Fixture(); let old = try x.provider().create(); x.fault.count = 0; x.fault.fail = step
            _ = try? x.provider().reset(); x.fault.fail = 0
            do { let after = try x.provider().load(); precondition(!x.pins.present && after.identity.signingKey != old.identity.signingKey) }
            catch let error as IdentityFailure { precondition(error == .recoveryRequired) }
            let recovered = try x.provider().reset(); precondition(!x.pins.present && recovered.identity.signingKey != old.identity.signingKey)
            refused(.staleHandle) { try x.provider().sign(old.handle, transcript: transcript) }
        }
        for step in 1...4 {
            let x = Fixture(); x.fault.fail = step
            _ = try? x.provider().create(); x.fault.fail = 0
            do { _ = try x.provider().load() }
            catch let error as IdentityFailure { precondition(error == .recoveryRequired) }
            _ = try x.provider().reset(); precondition(!x.pins.present)
        }
        let entropy = Fixture(); entropy.protection.entropyFailure = true
        refused(.unavailable) { try entropy.provider().create() }
        refused(.recoveryRequired) { try entropy.provider().load() }
        // Host execution cannot satisfy the production iPhone wrapping policy.
        refused(.unavailable) { try AppleIdentityProtection().requireUnlocked() }
        print("MC-017 Swift: provisioning/reopen/sign/agree/lock/key-loss/reset and 7 recovery faults PASS; injected wrapping/storage only")
    }
}
