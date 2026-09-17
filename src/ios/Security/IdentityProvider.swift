import Foundation
import Darwin
import Security
#if canImport(UIKit)
import UIKit
#endif

enum IdentityFailure: Error, Equatable { case locked, unavailable, invalidated, missing, invalidInput, recoveryRequired, staleHandle, provider }
enum WrappingProtection { case unknown, software, hardware }
struct IdentityCapabilities { let wrapping: WrappingProtection; let softwareCurves = true }
struct IdentityHandle { fileprivate let generation: Data }
struct IdentityInfo { let handle: IdentityHandle; let identity: PublicIdentity; let capabilities: IdentityCapabilities }

// Implementations must durably/idempotently invalidate identity-bound pins and
// state. MC-018 supplies that store; failure retains the reset barrier.
@MainActor protocol IdentityResetStore: AnyObject { func clearIdentityState() throws }
enum IdentityFile: Hashable { case envelope, journal }
@MainActor protocol IdentityStorage: AnyObject {
    func exists(_ file: IdentityFile) -> Bool
    func read(_ file: IdentityFile) throws -> Data
    func write(_ file: IdentityFile, _ bytes: Data) throws
    func delete(_ file: IdentityFile) throws
    func hasArtifacts() -> Bool
}
@MainActor protocol IdentityProtection: AnyObject {
    func requireUnlocked() throws
    func exists() throws -> Bool
    func create() throws
    func delete() throws
    func random(_ size: Int) throws -> Data
    func seal(_ plain: Data, aad: Data) throws -> Data
    func open(_ cipher: Data, aad: Data) throws -> Data
    func capabilities() throws -> IdentityCapabilities
}

// Actor serialization prevents reset racing an operation. The application's
// methods take opaque generation handles and never return private material.
@MainActor final class IdentityProvider {
    private static var resettingState = false
    static func requireResetInProgress() throws {
        guard resettingState else { throw IdentityFailure.recoveryRequired }
    }
    private let storage: IdentityStorage
    private let protection: IdentityProtection
    private let state: IdentityResetStore
    init(storage: IdentityStorage, protection: IdentityProtection, state: IdentityResetStore) {
        self.storage = storage; self.protection = protection; self.state = state
    }
    static func native(state: IdentityResetStore) throws -> IdentityProvider {
        IdentityProvider(storage: try AppleIdentityStorage(), protection: AppleIdentityProtection(), state: state)
    }
    private func guarded<T>(_ work: () throws -> T) throws -> T {
        do { return try work() }
        catch let error as IdentityFailure { throw error }
        catch IdentityError.InvalidInput { throw IdentityFailure.invalidInput }
        catch IdentityError.InvalidPeer { throw IdentityFailure.invalidInput }
        catch IdentityError.Unavailable { throw IdentityFailure.unavailable }
        catch { throw IdentityFailure.provider }
    }
    func create() throws -> IdentityInfo { try guarded {
        try protection.requireUnlocked()
        guard !storage.hasArtifacts(), try !protection.exists() else { throw IdentityFailure.recoveryRequired }
        try storage.write(.journal, Data([1])); try provision(); try storage.delete(.journal)
        return try loadInternal()
    } }
    func load() throws -> IdentityInfo { try guarded { try loadInternal() } }
    private func checkAvailable() throws {
        try protection.requireUnlocked()
        guard !storage.exists(.journal) else { throw IdentityFailure.recoveryRequired }
        guard storage.exists(.envelope) else {
            if try protection.exists() || storage.hasArtifacts() { throw IdentityFailure.recoveryRequired }
            throw IdentityFailure.missing
        }
        guard try protection.exists() else { throw IdentityFailure.invalidated }
    }
    private func unlocked<T>(_ handle: IdentityHandle?, _ work: (IdentityKeySession) throws -> T) throws -> T {
        try checkAvailable()
        let encoded = try storage.read(.envelope)
        guard (18...1024).contains(encoded.count), encoded.first == 1 else { throw IdentityFailure.invalidInput }
        let prefix = Data(encoded.prefix(17)), generation = Data(encoded[1..<17])
        if let handle, handle.generation != generation { throw IdentityFailure.staleHandle }
        var material = try protection.open(Data(encoded.dropFirst(17)), aad: prefix)
        defer { material.resetBytes(in: 0..<material.count) }
        guard material.count == 64 else { throw IdentityFailure.invalidInput }
        let session = try IdentityKeySession.importUnlocked(material: material, generation: generation)
        defer { try? session.invalidate() }
        let result = try work(session)
        try protection.requireUnlocked()
        return result
    }
    private func loadInternal() throws -> IdentityInfo { try unlocked(nil) { session in
        let metadata = try session.publicIdentity()
        return IdentityInfo(handle: IdentityHandle(generation: metadata.generation), identity: metadata, capabilities: try protection.capabilities())
    } }
    func publicIdentity(_ handle: IdentityHandle) throws -> PublicIdentity { try guarded { try unlocked(handle) { try $0.publicIdentity() } } }
    /// The synchronous caller may use the session only inside this operation.
    /// Every return or failure invalidates it before protected access ends.
    func messaging<T>(_ work: (IdentityKeySession) throws -> T) throws -> T {
        try guarded { try unlocked(nil, work) }
    }
    func sign(_ handle: IdentityHandle, transcript: Data) throws -> Data { try guarded {
        guard transcript.count <= 2048 else { throw IdentityFailure.invalidInput }
        return try unlocked(handle) { try $0.sign(message: transcript) }
    } }
    func agree(_ handle: IdentityHandle, peer: Data) throws -> Data { try guarded {
        guard peer.count == 32 else { throw IdentityFailure.invalidInput }
        return try unlocked(handle) { try $0.agree(peer: peer) }
    } }
    func reset() throws -> IdentityInfo { try guarded {
        try protection.requireUnlocked()
        try storage.write(.journal, Data([2]))
        guard !Self.resettingState else { throw IdentityFailure.recoveryRequired }
        Self.resettingState = true
        do {
            defer { Self.resettingState = false }
            try state.clearIdentityState()
        }
        try protection.delete(); try storage.delete(.envelope)
        try provision(); try storage.delete(.journal)
        return try loadInternal()
    } }
    private func provision() throws {
        try protection.create()
        let generation = try protection.random(16)
        guard generation.count == 16, generation.contains(where: { $0 != 0 }) else { throw IdentityFailure.provider }
        let prefix = Data([1]) + generation
        var material = try protection.random(64)
        defer { material.resetBytes(in: 0..<material.count) }
        guard material.count == 64 else { throw IdentityFailure.provider }
        let encoded = prefix + (try protection.seal(material, aad: prefix))
        guard encoded.count <= 1024 else { throw IdentityFailure.provider }
        try storage.write(.envelope, encoded)
    }
}

@MainActor final class AppleIdentityStorage: IdentityStorage {
    private let folder: URL
    init(folder: URL) { self.folder = folder }
    init(name: String = "meshchat-identity-v1") throws {
        folder = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: false)
            .appendingPathComponent(name, isDirectory: true)
    }
    private func path(_ file: IdentityFile) -> URL { folder.appendingPathComponent(file == .envelope ? "identity.enc" : "operation") }
    func hasArtifacts() -> Bool { FileManager.default.fileExists(atPath: folder.path) }
    func exists(_ file: IdentityFile) -> Bool { FileManager.default.fileExists(atPath: path(file).path) }
    func read(_ file: IdentityFile) throws -> Data {
        let handle = try FileHandle(forReadingFrom: path(file)); defer { try? handle.close() }
        let data = try handle.read(upToCount: 1025) ?? Data()
        guard data.count <= 1024 else { throw IdentityFailure.invalidInput }
        return data
    }
    private func syncDirectory(_ directory: URL) throws {
        let fd = Darwin.open(directory.path, O_RDONLY)
        guard fd >= 0 else { throw IdentityFailure.provider }
        defer { Darwin.close(fd) }
        guard fsync(fd) == 0 else { throw IdentityFailure.provider }
    }
    func write(_ file: IdentityFile, _ bytes: Data) throws {
        guard bytes.count <= 1024 else { throw IdentityFailure.invalidInput }
        if !hasArtifacts() {
            #if os(iOS)
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true, attributes: [.protectionKey: FileProtectionType.complete])
            #else
            try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
            #endif
            try syncDirectory(folder.deletingLastPathComponent())
        }
        // Retry protection/exclusion even after an interrupted directory setup.
        #if os(iOS)
        try FileManager.default.setAttributes([.protectionKey: FileProtectionType.complete], ofItemAtPath: folder.path)
        #endif
        var excluded = folder; var values = URLResourceValues(); values.isExcludedFromBackup = true
        try excluded.setResourceValues(values)
        // Verify the filesystem value, not metadata cached earlier in this run loop.
        excluded.removeAllCachedResourceValues()
        guard try excluded.resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup == true else { throw IdentityFailure.provider }
        #if os(iOS)
        try bytes.write(to: path(file), options: [.atomic, .completeFileProtection])
        #else
        // Native key protection still refuses host operations. Filesystem tests
        // can exercise actual backup attributes with test-only wrapping.
        try bytes.write(to: path(file), options: [.atomic])
        #endif
        let handle = try FileHandle(forWritingTo: path(file)); defer { try? handle.close() }
        try handle.synchronize(); try syncDirectory(folder)
    }
    func delete(_ file: IdentityFile) throws {
        if exists(file) { try FileManager.default.removeItem(at: path(file)); try syncDirectory(folder) }
    }
}

@MainActor final class AppleIdentityProtection: IdentityProtection {
    private let tag: Data
    private let payloadRange: ClosedRange<Int>
    init(tag: String = "org.meshchat.identity.wrapping.v1", payloadRange: ClosedRange<Int> = 64...64) {
        self.tag = Data(tag.utf8); self.payloadRange = payloadRange
    }
    private let algorithm = SecKeyAlgorithm.eciesEncryptionCofactorX963SHA256AESGCM
    private var query: [String: Any] { [kSecClass as String: kSecClassKey, kSecAttrApplicationTag as String: tag, kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom] }
    func requireUnlocked() throws {
        #if os(iOS)
        guard UIApplication.shared.isProtectedDataAvailable else { throw IdentityFailure.locked }
        #else
        throw IdentityFailure.unavailable
        #endif
    }
    private func check(_ status: OSStatus) throws {
        if status == errSecInteractionNotAllowed { throw IdentityFailure.locked }
        if status == errSecItemNotFound { throw IdentityFailure.invalidated }
        guard status == errSecSuccess else { throw IdentityFailure.provider }
    }
    func exists() throws -> Bool {
        let status = SecItemCopyMatching(query as CFDictionary, nil)
        if status == errSecItemNotFound { return false }; try check(status); return true
    }
    private func key() throws -> SecKey {
        try requireUnlocked(); var request = query; request[kSecReturnRef as String] = true
        var result: CFTypeRef?; try check(SecItemCopyMatching(request as CFDictionary, &result))
        guard let result else { throw IdentityFailure.invalidated }
        let key = result as! SecKey
        guard let attributes = SecKeyCopyAttributes(key) as? [String: Any],
              attributes[kSecAttrTokenID as String] as? String == kSecAttrTokenIDSecureEnclave as String else { throw IdentityFailure.unavailable }
        return key
    }
    func create() throws {
        try requireUnlocked(); guard try !exists() else { throw IdentityFailure.recoveryRequired }
        var error: Unmanaged<CFError>?
        guard let access = SecAccessControlCreateWithFlags(nil, kSecAttrAccessibleWhenUnlockedThisDeviceOnly, .privateKeyUsage, &error) else {
            if let error { _ = error.takeRetainedValue() }; throw IdentityFailure.unavailable
        }
        let attributes: [String: Any] = [kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom, kSecAttrKeySizeInBits as String: 256,
            kSecAttrTokenID as String: kSecAttrTokenIDSecureEnclave,
            kSecPrivateKeyAttrs as String: [kSecAttrIsPermanent as String: true, kSecAttrApplicationTag as String: tag, kSecAttrAccessControl as String: access]]
        guard SecKeyCreateRandomKey(attributes as CFDictionary, &error) != nil else {
            if let error { _ = error.takeRetainedValue() }; throw IdentityFailure.unavailable
        }
    }
    func delete() throws { try requireUnlocked(); let status = SecItemDelete(query as CFDictionary); if status != errSecItemNotFound { try check(status) } }
    func random(_ size: Int) throws -> Data {
        var data = Data(count: size)
        let status = data.withUnsafeMutableBytes { SecRandomCopyBytes(kSecRandomDefault, size, $0.baseAddress!) }
        guard status == errSecSuccess else { data.resetBytes(in: 0..<data.count); throw IdentityFailure.unavailable }
        return data
    }
    func seal(_ plain: Data, aad: Data) throws -> Data {
        guard payloadRange.contains(plain.count) else { throw IdentityFailure.invalidInput }
        let privateKey = try key(); guard let publicKey = SecKeyCopyPublicKey(privateKey), SecKeyIsAlgorithmSupported(publicKey, .encrypt, algorithm) else { throw IdentityFailure.unavailable }
        // ECIES API has no AAD argument: authenticate the full header inside the
        // encrypted payload and require exact equality before importing seeds.
        var bound = aad + plain; defer { bound.resetBytes(in: 0..<bound.count) }
        var error: Unmanaged<CFError>?
        guard let encrypted = SecKeyCreateEncryptedData(publicKey, algorithm, bound as CFData, &error) else {
            if let error { _ = error.takeRetainedValue() }; throw IdentityFailure.provider
        }
        return encrypted as Data
    }
    func open(_ cipher: Data, aad: Data) throws -> Data {
        let privateKey = try key(); var error: Unmanaged<CFError>?
        guard let decoded = SecKeyCreateDecryptedData(privateKey, algorithm, cipher as CFData, &error) else {
            if let error { _ = error.takeRetainedValue() }; throw IdentityFailure.invalidInput
        }
        var bound = decoded as Data; defer { bound.resetBytes(in: 0..<bound.count) }
        guard payloadRange.contains(bound.count - aad.count), bound.prefix(aad.count) == aad else { throw IdentityFailure.invalidInput }
        return Data(bound.dropFirst(aad.count))
    }
    func capabilities() throws -> IdentityCapabilities { _ = try key(); return IdentityCapabilities(wrapping: .hardware) }
}
