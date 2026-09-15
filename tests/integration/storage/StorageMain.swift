import Foundation
import CryptoKit
import Security

// Test-only wrapping key. No production provider permits this host fallback.
@MainActor private final class TestProtection: IdentityProtection {
    let flag: URL; let key = SymmetricKey(data: Data(repeating: 71, count: 32))
    init(_ flag: URL) { self.flag = flag }
    func requireUnlocked() throws {}
    func exists() throws -> Bool { FileManager.default.fileExists(atPath: flag.path) }
    func create() throws { guard try !exists() else { throw IdentityFailure.recoveryRequired }; try Data([1]).write(to: flag, options: .atomic) }
    func delete() throws { if try exists() { try FileManager.default.removeItem(at: flag) } }
    func random(_ size: Int) throws -> Data { var data = Data(count: size); let rc = data.withUnsafeMutableBytes { SecRandomCopyBytes(kSecRandomDefault, size, $0.baseAddress!) }; guard rc == errSecSuccess else { throw IdentityFailure.unavailable }; return data }
    func seal(_ plain: Data, aad: Data) throws -> Data { guard try exists() else { throw IdentityFailure.invalidated }; return try AES.GCM.seal(plain, using: key, authenticating: aad).combined! }
    func open(_ cipher: Data, aad: Data) throws -> Data { guard try exists() else { throw IdentityFailure.invalidated }; return try AES.GCM.open(AES.GCM.SealedBox(combined: cipher), using: key, authenticating: aad) }
    func capabilities() throws -> IdentityCapabilities { IdentityCapabilities(wrapping: .software) }
}
private final class FaultDatabase: SqlDatabase, @unchecked Sendable {
    let connection: CipherConnection; let fail: String
    init(_ connection: CipherConnection, _ fail: String) { self.connection = connection; self.fail = fail }
    func execute(sql: String, values: [SqlValue]) throws { if sql.hasPrefix(fail) { throw StorageError.Database }; try connection.execute(sql: sql, values: values) }
    func query(sql: String, values: [SqlValue], limit: UInt32) throws -> [SqlRow] { try connection.query(sql: sql, values: values, limit: limit) }
}
@MainActor private func refused<T>(_ work: () throws -> T) { do { _ = try work(); fatalError("expected refusal") } catch {} }
@main private struct StorageChecks {
    @MainActor static func main() throws {
        let root = URL(fileURLWithPath: CommandLine.arguments[1], isDirectory: true); let phase = CommandLine.arguments[2]
        let folder = root.appendingPathComponent("store", isDirectory: true)
        let files = AppleIdentityStorage(folder: folder)
        let protection = TestProtection(root.appendingPathComponent("storage-wrapping-present"))
        let vault = StorageVault(folder: folder, files: files, protection: protection)
        let identity = IdentityProvider(storage: AppleIdentityStorage(folder: root.appendingPathComponent("identity", isDirectory: true)), protection: TestProtection(root.appendingPathComponent("identity-wrapping-present")), state: vault)
        let store = EncryptedStorage(identity: identity, vault: vault)
        let marker = Data("MC018 synthetic plaintext marker".utf8), peer = Data((1...64).map(UInt8.init)), channel = Data([1, 2, 3, 4])
        let now = Int64(Date().timeIntervalSince1970), subject = Data([2]) + peer
        func item(_ direct: Bool, _ id: UInt8) -> HistoryItem { HistoryItem(conversation: direct ? peer : channel, direct: direct, direction: 0, logicalType: 1, messageId: Data(repeating: id, count: 8), timestamp: now, body: marker, provenance: direct ? Data([1, 2, 3]) : Data()) }
        func passphrase() throws -> Data { let encoded = try files.read(.envelope); return try protection.open(Data(encoded.dropFirst(17)), aad: Data(encoded.prefix(17))) }
        switch phase {
        case "create":
            refused { try store.reopen() }; let own = try identity.create(); try store.create()
            try store.putRecord(kind: .setting, key: Data("own-public".utf8), value: own.identity.signingKey)
            try store.putRecord(kind: .friend, key: peer, value: marker)
            try store.putRecord(kind: .subscription, key: channel, value: Data([1]))
            try store.putRecord(kind: .eventRoot, key: Data(repeating: 7, count: 32), value: marker)
            try store.putRecord(kind: .staffCredential, key: Data(repeating: 8, count: 64), value: marker)
            try store.appendUnverified(item(false, 1))
            let accepted = try store.acceptAuthenticated(item(true, 2), subject: subject, immutableBytes: Data([3, 4])); precondition(accepted == .accepted)
            let excluded = try folder.resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup
            precondition(excluded == true)
        case "reopen":
            try store.reopen()
            let own = try store.getRecord(kind: .setting, key: Data("own-public".utf8)), current = try identity.load().identity.signingKey
            precondition(own == current)
            let history = try store.history(conversation: channel, direct: false, limit: 100), dm = try store.history(conversation: peer, direct: true, limit: 100)
            precondition(history.count == 1 && history[0].body == marker && dm.count == 1 && dm[0].body == marker)
            var key = try passphrase(); defer { key.resetBytes(in: 0..<key.count) }
            for file in try FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil) { let bytes = try Data(contentsOf: file); precondition(bytes.range(of: marker) == nil && bytes.range(of: key) == nil) }
            var wrong = key; wrong[0] ^= 1; defer { wrong.resetBytes(in: 0..<wrong.count) }
            refused { let db = try CipherConnection(file: folder.appendingPathComponent("history.db"), key: wrong, create: false); defer { db.close() }; _ = try EncryptedStore.open(db: db, generation: identity.load().identity.generation, create: false, now: now) }
            try store.reopen(); refused { try store.create() }
        case "checks":
            let generation = try identity.load().identity.generation
            try store.deleteHistory(conversation: peer, direct: true)
            let replay = try store.acceptAuthenticated(item(true, 2), subject: subject, immutableBytes: Data([3, 4])); precondition(replay == .replay)
            let conflict = try store.acceptAuthenticated(item(true, 2), subject: subject, immutableBytes: Data([9])); precondition(conflict == .conflict)
            let ambiguous = try store.ambiguousTarget(subject: subject, messageId: Data(repeating: 2, count: 8)); precondition(ambiguous)
            let empty = try store.history(conversation: peer, direct: true, limit: 100); precondition(empty.isEmpty)
            var key = try passphrase(); defer { key.resetBytes(in: 0..<key.count) }
            let db = try CipherConnection(file: folder.appendingPathComponent("history.db"), key: key, create: false); defer { db.close() }
            try db.execute(sql: "BEGIN IMMEDIATE", values: [])
            do {
                try db.execute(sql: "UPDATE records SET value=? WHERE kind=2", values: [.bytes(value: marker + Data([9]))])
                precondition(FileManager.default.fileExists(atPath: folder.appendingPathComponent("history.db-journal").path))
                for file in try FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil) { let bytes = try Data(contentsOf: file); precondition(bytes.range(of: marker) == nil && bytes.range(of: key) == nil) }
                try db.execute(sql: "ROLLBACK", values: [])
            } catch { try? db.execute(sql: "ROLLBACK", values: []); throw error }
            let fault = try EncryptedStore.open(db: FaultDatabase(db, "INSERT INTO history"), generation: generation, create: false, now: now)
            refused { try fault.acceptAuthenticated(item: item(true, 3), subject: subject, immutableBytes: Data([5]), now: now) }
            let normal = try EncryptedStore.open(db: db, generation: generation, create: false, now: now)
            let accepted = try normal.acceptAuthenticated(item: item(true, 3), subject: subject, immutableBytes: Data([5]), now: now); precondition(accepted == .accepted)
            try db.execute(sql: "DROP TABLE ledger", values: []); try db.execute(sql: "DROP TABLE clock", values: []); try db.execute(sql: "PRAGMA user_version=1", values: [])
            refused { try EncryptedStore.open(db: FaultDatabase(db, "CREATE TABLE clock"), generation: generation, create: false, now: now) }
            let version = try db.query(sql: "PRAGMA user_version", values: [], limit: 1)
            guard case .integer(let number) = version[0].cells[0], number == 1 else { fatalError("migration rollback") }
            let migrated = try EncryptedStore.open(db: db, generation: generation, create: false, now: now)
            let pin = try migrated.getRecord(kind: .friend, key: peer); precondition(pin == marker)
            try migrated.prune(now: now + 1000); refused { try migrated.prune(now: now) }; refused { try migrated.prune(now: nil) }; try migrated.prune(now: now + 1000)
            try migrated.prune(now: now + 172801)
            let expired = try migrated.history(conversation: channel, direct: false, limit: 100); precondition(expired.isEmpty)
        case "key-loss":
            let before = try Data(contentsOf: folder.appendingPathComponent("history.db")); try protection.delete()
            refused { try store.reopen() }; let after = try Data(contentsOf: folder.appendingPathComponent("history.db")); precondition(before == after)
        case "reset":
            let old = try identity.load(), fresh = try identity.reset()
            precondition(old.identity.signingKey != fresh.identity.signingKey && !FileManager.default.fileExists(atPath: folder.path))
            refused { try store.reopen() }; try store.create()
            let pin = try store.getRecord(kind: .friend, key: peer); precondition(pin == nil)
            let empty = try store.history(conversation: peer, direct: true, limit: 100); precondition(empty.isEmpty)
            refused { try identity.sign(old.handle, transcript: Data([1])) }
        default: fatalError("unknown phase")
        }
        print("MC-018 Swift SQLCipher \(phase) PASS; test wrapping only, physical protection pending")
    }
}
