import CryptoKit
import Security
import SQLCipher
import SwiftUI
import UIKit

// Feasibility fixture only: never import real identities, keys, or messages.
private enum BenchError: Error { case refused, key(OSStatus), crypto, database(Int32) }

@MainActor
private final class Bench: ObservableObject {
    @Published var rows: [String] = []
    private let tag = Data("org.meshchat.mc005.wrapping.v1".utf8)
    private var held: OpaquePointer?
    private let manager = FileManager.default
    private var folder: URL {
        manager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("mc005", isDirectory: true)
    }
    private var envelope: URL { folder.appendingPathComponent("material.enc") }
    private var database: URL { folder.appendingPathComponent("fixture.db") }
    private var query: [String: Any] {
        [kSecClass as String: kSecClassKey, kSecAttrApplicationTag as String: tag,
         kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom]
    }
    func event(_ text: String) {
        if rows.count == 512 { rows.removeFirst() }
        rows.append("\(Int(ProcessInfo.processInfo.systemUptime)) protected_data_available=\(UIApplication.shared.isProtectedDataAvailable) \(text)")
    }
    func run(_ name: String, _ work: () throws -> Void) {
        do { try work(); event("operation_ok=\(name)") }
        catch let error as BenchError {
            switch error {
            case .key(let status): event("operation_failed=\(name) key_status=\(status)")
            case .database(let code): event("operation_failed=\(name) database_code=\(code)")
            default: event("operation_failed=\(name) category=refused_or_crypto")
            }
        } catch { event("operation_failed=\(name); details_omitted") }
    }
    private func require(_ condition: Bool) throws { if !condition { throw BenchError.refused } }
    private func key() throws -> SecKey {
        var request = query; request[kSecReturnRef as String] = true
        var item: CFTypeRef?
        let status = SecItemCopyMatching(request as CFDictionary, &item)
        guard status == errSecSuccess, let item else { throw BenchError.key(status) }
        return item as! SecKey
    }
    private func createKey() throws -> SecKey {
        // A simulator cannot satisfy this policy. There is no software wrapping fallback.
        var error: Unmanaged<CFError>?
        guard let access = SecAccessControlCreateWithFlags(nil, kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
                                                           .privateKeyUsage, &error) else {
            if let error { throw error.takeRetainedValue() }
            throw BenchError.crypto
        }
        let attributes: [String: Any] = [
            kSecAttrKeyType as String: kSecAttrKeyTypeECSECPrimeRandom,
            kSecAttrKeySizeInBits as String: 256,
            kSecAttrTokenID as String: kSecAttrTokenIDSecureEnclave,
            kSecPrivateKeyAttrs as String: [kSecAttrIsPermanent as String: true,
                kSecAttrApplicationTag as String: tag, kSecAttrAccessControl as String: access]
        ]
        guard let result = SecKeyCreateRandomKey(attributes as CFDictionary, &error) else {
            if let error { throw error.takeRetainedValue() }
            throw BenchError.crypto
        }
        event("wrapping=SecureEnclave_P256; accessibility=WhenUnlockedThisDeviceOnly; software_curves_in_process_memory")
        return result
    }
    private let wrappingAlgorithm = SecKeyAlgorithm.eciesEncryptionCofactorX963SHA256AESGCM
    private func material() throws -> Data {
        let encoded = try Data(contentsOf: envelope)
        try require(encoded.count > 1 && encoded.count < 1024 && encoded.first == 1)
        let privateKey = try key()
        var error: Unmanaged<CFError>?
        guard let result = SecKeyCreateDecryptedData(privateKey, wrappingAlgorithm, encoded.dropFirst() as CFData, &error) else {
            if let error { throw error.takeRetainedValue() }
            throw BenchError.crypto
        }
        let bytes = result as Data
        try require(bytes.count == 96)
        return bytes
    }
    func create() throws {
        try require(!manager.fileExists(atPath: folder.path))
        let status = SecItemCopyMatching(query as CFDictionary, nil)
        guard status == errSecItemNotFound else { throw BenchError.key(status) }
        let privateKey = try createKey()
        try manager.createDirectory(at: folder, withIntermediateDirectories: true,
                                    attributes: [.protectionKey: FileProtectionType.complete])
        var excluded = folder; var values = URLResourceValues(); values.isExcludedFromBackup = true
        try excluded.setResourceValues(values)
        try require(try folder.resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup == true)
        guard let publicKey = SecKeyCopyPublicKey(privateKey),
              SecKeyIsAlgorithmSupported(publicKey, .encrypt, wrappingAlgorithm) else { throw BenchError.crypto }
        var bytes = try probeRandomMaterial(); defer { bytes.resetBytes(in: 0..<bytes.count) }
        var error: Unmanaged<CFError>?
        guard let encrypted = SecKeyCreateEncryptedData(publicKey, wrappingAlgorithm, bytes as CFData, &error) else {
            if let error { throw error.takeRetainedValue() }
            throw BenchError.crypto
        }
        try (Data([1]) + (encrypted as Data)).write(to: envelope, options: [.atomic, .completeFileProtection])
        var databaseKey = Data(bytes[64..<96]); defer { databaseKey.resetBytes(in: 0..<databaseKey.count) }
        let db = try openDatabase(key: databaseKey, create: true); defer { sqlite3_close(db) }
        try cipherPresent(db)
        try sql(db, "CREATE TABLE fixture(value INTEGER NOT NULL)")
        try sql(db, "INSERT INTO fixture VALUES (5)")
        try checkRow(db)
        let protection = try manager.attributesOfItem(atPath: database.path)[.protectionKey] as? FileProtectionType
        try require(protection == .complete)
        event("created; complete_file_protection=true; backup_exclusion_attribute=true; actual_restore_untested")
    }
    private func openDatabase(key: Data, create: Bool) throws -> OpaquePointer {
        var handle: OpaquePointer?
        let result = sqlite3_open_v2(database.path, &handle, SQLITE_OPEN_READWRITE | (create ? SQLITE_OPEN_CREATE : 0), nil)
        guard result == SQLITE_OK, let db = handle else {
            if let handle { sqlite3_close(handle) }; throw BenchError.database(result)
        }
        let keyed = key.withUnsafeBytes { sqlite3_key(db, $0.baseAddress, Int32($0.count)) }
        guard keyed == SQLITE_OK else { sqlite3_close(db); throw BenchError.database(keyed) }
        return db
    }
    private func open(wrong: Bool = false) throws -> OpaquePointer {
        try require(manager.fileExists(atPath: database.path))
        var bytes = try material(); defer { bytes.resetBytes(in: 0..<bytes.count) }
        var key = Data(bytes[64..<96]); defer { key.resetBytes(in: 0..<key.count) }
        if wrong { key[0] ^= 1 }
        return try openDatabase(key: key, create: false)
    }
    private func sql(_ db: OpaquePointer, _ statement: String) throws {
        let result = sqlite3_exec(db, statement, nil, nil, nil)
        if result != SQLITE_OK { throw BenchError.database(result) }
    }
    private func prepare(_ db: OpaquePointer, _ sql: String) throws -> OpaquePointer {
        var statement: OpaquePointer?
        let result = sqlite3_prepare_v2(db, sql, -1, &statement, nil)
        guard result == SQLITE_OK, let statement else {
            if let statement { sqlite3_finalize(statement) }; throw BenchError.database(result)
        }
        return statement
    }
    private func cipherPresent(_ db: OpaquePointer) throws {
        let statement = try prepare(db, "PRAGMA cipher_version"); defer { sqlite3_finalize(statement) }
        try require(sqlite3_step(statement) == SQLITE_ROW)
        guard let value = sqlite3_column_text(statement, 0) else { throw BenchError.refused }
        let version = String(cString: value); try require(!version.isEmpty)
        event("cipher_version=\(version)")
    }
    private func checkRow(_ db: OpaquePointer) throws {
        let statement = try prepare(db, "SELECT value FROM fixture"); defer { sqlite3_finalize(statement) }
        let first = sqlite3_step(statement)
        guard first == SQLITE_ROW else { throw BenchError.database(first) }
        try require(sqlite3_column_int(statement, 0) == 5 && sqlite3_step(statement) == SQLITE_DONE)
    }
    func verify() throws {
        try require(held == nil)
        let db = try open(); defer { sqlite3_close(db) }
        try cipherPresent(db); try checkRow(db); event("reopen_ok")
        try curves()
        var rejected = false
        do { let wrong = try open(wrong: true); defer { sqlite3_close(wrong) }; try checkRow(wrong) }
        catch BenchError.database(let code) where code == SQLITE_NOTADB { rejected = true }
        try require(rejected); event("wrong_key_rejected")
        let reopened = try open(); defer { sqlite3_close(reopened) }; try checkRow(reopened)
        event("correct_key_still_opens")
    }
    private func curves() throws {
        var bytes = try material(); defer { bytes.resetBytes(in: 0..<bytes.count) }
        var signing = Data(bytes[0..<32]); defer { signing.resetBytes(in: 0..<signing.count) }
        var agreement = Data(bytes[32..<64]); defer { agreement.resetBytes(in: 0..<agreement.count) }
        let message = Data("MC005 synthetic challenge".utf8)
        let nativeSigning = try Curve25519.Signing.PrivateKey(rawRepresentation: signing)
        let publicKey = try probeSigningPublic(secret: signing)
        try require(publicKey == nativeSigning.publicKey.rawRepresentation)
        let signature = try probeSign(secret: signing, message: message)
        try require(nativeSigning.publicKey.isValidSignature(signature, for: message))
        try require(probeVerify(public: publicKey, message: message, signature: try nativeSigning.signature(for: message)))
        var bad = signature; bad[0] ^= 1
        try require(!probeVerify(public: publicKey, message: message, signature: bad))
        let peer = Curve25519.KeyAgreement.PrivateKey()
        let publicAgreement = try Curve25519.KeyAgreement.PublicKey(rawRepresentation: probeAgreementPublic(secret: agreement))
        let shared = try peer.sharedSecretFromKeyAgreement(with: publicAgreement)
        var expected = shared.withUnsafeBytes { Data($0) }; defer { expected.resetBytes(in: 0..<expected.count) }
        try require(try probeAgreementMatches(secret: agreement, peer: peer.publicKey.rawRepresentation, expected: expected))
        event("CryptoKit_Rust_Ed25519_X25519_interop_ok; Curve25519_software_only; FFI_copies_not_guaranteed_wiped")
    }
    func hold() throws { try require(held == nil); held = try open(); event("db_held_open") }
    func closeHeld() { if let held { sqlite3_close(held) }; held = nil; event("held_closed") }
    func lockReads() {
        run("lock_reopen") { let db = try open(); defer { sqlite3_close(db) }; try checkRow(db) }
        if let held { run("lock_held_read") { try checkRow(held) } }
    }
    func scheduleLockReads() {
        event("lock_reads_scheduled_15s; suspension_may_delay; inspect_protected_data_state")
        DispatchQueue.main.asyncAfter(deadline: .now() + 15) { self.lockReads() }
    }
    func invalidate() throws {
        closeHeld(); let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else { throw BenchError.key(status) }
        event("wrapping_key_deleted; ciphertext_retained")
    }
    func reset() throws {
        try invalidate()
        if manager.fileExists(atPath: folder.path) { try manager.removeItem(at: folder) }
        event("synthetic_fixture_reset")
    }
}

@main
struct SecurityProbeApp: App {
    @StateObject private var bench = Bench()
    var body: some Scene {
        WindowGroup {
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    Text("MC-005 synthetic security bench").font(.headline)
                    Text("Test fixture only. Copy the report before restart. Reset deletes this fixture. Secure Enclave requires a physical iPhone.")
                    Button("Create fresh protected fixture") { bench.run("create") { try bench.create() } }
                    Button("Reopen + curves + wrong-key rejection") { bench.run("verify") { try bench.verify() } }
                    Button("Open and hold DB across lock") { bench.run("hold") { try bench.hold() } }
                    Button("Read closed and held DB") { bench.lockReads() }
                    Button("Close held DB") { bench.closeHeld() }
                    Button("Schedule lock reads in 15 seconds") { bench.scheduleLockReads() }
                    Button("Invalidate wrapping key only") { bench.run("invalidate") { try bench.invalidate() } }
                    Button("Reset synthetic fixture") { bench.run("reset") { try bench.reset() } }
                    Button("Copy sanitized report") { UIPasteboard.general.string = bench.rows.joined(separator: "\n") }
                    Text(bench.rows.joined(separator: "\n")).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                }.padding()
            }
            .onAppear { bench.event("start iOS=\(UIDevice.current.systemVersion); reports_not_persisted; manual_reopen_required") }
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.protectedDataWillBecomeUnavailableNotification)) { _ in
                bench.event("protected_data_will_become_unavailable; not_yet_proof_of_lock_refusal")
            }
            .onReceive(NotificationCenter.default.publisher(for: UIApplication.protectedDataDidBecomeAvailableNotification)) { _ in
                bench.event("protected_data_available")
            }
        }
    }
}
