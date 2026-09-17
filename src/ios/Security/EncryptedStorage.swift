import Foundation
import Darwin
import SQLCipher

// The connection is private to a synchronous, unlocked native operation.
// A lock covers callback entry; callers receive typed store operations only.
final class CipherConnection: SqlDatabase, @unchecked Sendable {
    private var db: OpaquePointer?
    private let lock = NSRecursiveLock()
    init(file: URL, key: Data, create: Bool) throws {
        var handle: OpaquePointer?
        let rc = sqlite3_open_v2(file.path, &handle, SQLITE_OPEN_READWRITE | (create ? SQLITE_OPEN_CREATE : 0) | SQLITE_OPEN_FULLMUTEX, nil)
        guard rc == SQLITE_OK, let handle else { if let handle { sqlite3_close(handle) }; throw StorageError.Database }
        db = handle
        do {
            let keyed = key.withUnsafeBytes { sqlite3_key(handle, $0.baseAddress, Int32($0.count)) }
            guard keyed == SQLITE_OK else { throw StorageError.Database }
            _ = try query(sql: "PRAGMA journal_mode=DELETE", values: [], limit: 1)
            try execute(sql: "PRAGMA synchronous=FULL", values: [])
        } catch { close(); throw error }
    }
    func close() { lock.lock(); defer { lock.unlock() }; if let db { sqlite3_close_v2(db) }; db = nil }
    deinit { if let db { sqlite3_close_v2(db) } }
    private func prepare(_ sql: String, _ values: [SqlValue]) throws -> OpaquePointer {
        guard let db else { throw StorageError.Unavailable }
        var statement: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &statement, nil) == SQLITE_OK, let statement else {
            if let statement { sqlite3_finalize(statement) }; throw StorageError.Database
        }
        do {
            guard sqlite3_bind_parameter_count(statement) == values.count else { throw StorageError.InvalidInput }
            let transient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
            for (offset, value) in values.enumerated() {
                let index = Int32(offset + 1); let result: Int32
                switch value {
                case .integer(let value): result = sqlite3_bind_int64(statement, index, value)
                case .text(let value): result = value.withCString { sqlite3_bind_text(statement, index, $0, -1, transient) }
                case .bytes(let value):
                    result = value.isEmpty ? sqlite3_bind_zeroblob(statement, index, 0) : value.withUnsafeBytes { sqlite3_bind_blob(statement, index, $0.baseAddress, Int32($0.count), transient) }
                }
                guard result == SQLITE_OK else { throw StorageError.Database }
            }
            return statement
        } catch { sqlite3_finalize(statement); throw error }
    }
    func execute(sql: String, values: [SqlValue]) throws {
        lock.lock(); defer { lock.unlock() }
        let statement = try prepare(sql, values); defer { sqlite3_finalize(statement) }
        guard sqlite3_step(statement) == SQLITE_DONE else { throw StorageError.Database }
    }
    func query(sql: String, values: [SqlValue], limit: UInt32) throws -> [SqlRow] {
        lock.lock(); defer { lock.unlock() }
        guard limit <= 100 else { throw StorageError.InvalidInput }
        let statement = try prepare(sql, values); defer { sqlite3_finalize(statement) }
        var rows: [SqlRow] = []
        while true {
            let rc = sqlite3_step(statement)
            if rc == SQLITE_DONE { return rows }
            guard rc == SQLITE_ROW, rows.count < limit, sqlite3_column_count(statement) <= 12 else { throw StorageError.Database }
            var cells: [SqlValue] = []
            for column in 0..<sqlite3_column_count(statement) {
                switch sqlite3_column_type(statement, column) {
                case SQLITE_INTEGER: cells.append(.integer(value: sqlite3_column_int64(statement, column)))
                case SQLITE_BLOB:
                    let count = Int(sqlite3_column_bytes(statement, column)); guard count <= 4096 else { throw StorageError.Database }
                    let bytes = sqlite3_column_blob(statement, column)
                    cells.append(.bytes(value: count == 0 ? Data() : Data(bytes: bytes!, count: count)))
                case SQLITE_TEXT:
                    guard sqlite3_column_bytes(statement, column) <= 4096, let text = sqlite3_column_text(statement, column) else { throw StorageError.Database }
                    cells.append(.text(value: String(cString: text)))
                default: throw StorageError.Database
                }
            }
            rows.append(SqlRow(cells: cells))
        }
    }
}

@MainActor final class StorageVault: IdentityResetStore {
    private let folder: URL
    private let files: IdentityStorage
    private let protection: IdentityProtection
    init(folder: URL, files: IdentityStorage, protection: IdentityProtection) { self.folder = folder; self.files = files; self.protection = protection }
    static func native() throws -> StorageVault {
        let folder = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: false).appendingPathComponent("meshchat-storage-v1", isDirectory: true)
        return StorageVault(folder: folder, files: try AppleIdentityStorage(name: "meshchat-storage-v1"), protection: AppleIdentityProtection(tag: "org.meshchat.storage.wrapping.v1"))
    }
    private func sync(_ directory: URL) throws {
        let fd = Darwin.open(directory.path, O_RDONLY); guard fd >= 0 else { throw IdentityFailure.provider }
        defer { Darwin.close(fd) }; guard fsync(fd) == 0 else { throw IdentityFailure.provider }
    }
    func access<T>(generation: Data, create: Bool, now: Int64?, work: (EncryptedStore) throws -> T) throws -> T {
        try protection.requireUnlocked()
        guard generation.count == 16 else { throw IdentityFailure.invalidInput }
        let database = folder.appendingPathComponent("history.db")
        if create {
            guard !files.hasArtifacts(), try !protection.exists() else { throw IdentityFailure.recoveryRequired }
            try files.write(.journal, Data([1])); try protection.create()
            var key = try protection.random(64); defer { key.resetBytes(in: 0..<key.count) }
            let header = Data([1]) + generation
            try files.write(.envelope, header + protection.seal(key, aad: header))
        } else {
            guard !files.exists(.journal) else { throw IdentityFailure.recoveryRequired }
            guard files.exists(.envelope), FileManager.default.fileExists(atPath: database.path) else {
                if try files.hasArtifacts() || protection.exists() { throw IdentityFailure.recoveryRequired }
                throw IdentityFailure.missing
            }
        }
        guard try protection.exists() else { throw IdentityFailure.invalidated }
        let encoded = try files.read(.envelope)
        guard (18...1024).contains(encoded.count), encoded.first == 1, Data(encoded[1..<17]) == generation else { throw IdentityFailure.invalidInput }
        var key = try protection.open(Data(encoded.dropFirst(17)), aad: Data(encoded.prefix(17)))
        defer { key.resetBytes(in: 0..<key.count) }
        guard key.count == 64 else { throw IdentityFailure.invalidInput }
        // Files inherit the protected/excluded directory; set and verify the DB
        // explicitly before releasing the creation recovery marker.
        let connection = try CipherConnection(file: database, key: key, create: create)
        let result: T
        do {
            let core = try EncryptedStore.open(db: connection, generation: generation, create: create, now: now)
            result = try work(core)
            connection.close()
        } catch { connection.close(); throw error }
        try protection.requireUnlocked()
        #if os(iOS)
        try FileManager.default.setAttributes([.protectionKey: FileProtectionType.complete], ofItemAtPath: database.path)
        // Foundation documents this dictionary value as NSString, rather than
        // the Swift RawRepresentable wrapper used when setting the attribute.
        guard try FileManager.default.attributesOfItem(atPath: database.path)[.protectionKey] as? String == FileProtectionType.complete.rawValue else { throw IdentityFailure.provider }
        #endif
        if create { try sync(folder); try files.delete(.journal) }
        return result
    }
    func clearIdentityState() throws {
        try IdentityProvider.requireResetInProgress()
        try protection.requireUnlocked()
        if try files.hasArtifacts() || protection.exists() {
            try files.write(.journal, Data([2])); try protection.delete()
            for name in ["history.db", "history.db-wal", "history.db-shm", "history.db-journal"] {
                let url = folder.appendingPathComponent(name)
                if FileManager.default.fileExists(atPath: url.path) { try FileManager.default.removeItem(at: url) }
            }
            try files.delete(.envelope); try files.delete(.journal)
            // Refuse unknown retained artifacts; never recursively remove them.
            guard try FileManager.default.contentsOfDirectory(atPath: folder.path).isEmpty else { throw IdentityFailure.recoveryRequired }
            try FileManager.default.removeItem(at: folder); try sync(folder.deletingLastPathComponent())
        }
    }
}

@MainActor final class EncryptedStorage {
    private let identity: IdentityProvider
    private let vault: StorageVault
    init(identity: IdentityProvider, vault: StorageVault) { self.identity = identity; self.vault = vault }
    /// Synchronous feature operations never retain the store/connection after return.
    func operation<T>(create: Bool = false, _ work: (EncryptedStore) throws -> T) throws -> T {
        let generation = try identity.load().identity.generation
        let now = Int64(Date().timeIntervalSince1970)
        return try vault.access(generation: generation, create: create, now: now, work: work)
    }
    func create() throws { try operation(create: true) { _ in } }
    func reopen() throws { try operation { _ in } }
    func putRecord(kind: RecordKind, key: Data, value: Data) throws { try operation { try $0.putRecord(kind: kind, key: key, value: value) } }
    func getRecord(kind: RecordKind, key: Data) throws -> Data? { try operation { try $0.getRecord(kind: kind, key: key) } }
    func deleteRecord(kind: RecordKind, key: Data) throws { try operation { try $0.deleteRecord(kind: kind, key: key) } }
    func appendUnverified(_ item: HistoryItem) throws { try operation { try $0.appendUnverified(item: item, now: Int64(Date().timeIntervalSince1970)) } }
    func acceptAuthenticated(_ item: HistoryItem, subject: Data, immutableBytes: Data) throws -> AcceptResult { try operation { try $0.acceptAuthenticated(item: item, subject: subject, immutableBytes: immutableBytes, now: Int64(Date().timeIntervalSince1970)) } }
    func history(conversation: Data, direct: Bool, limit: UInt32) throws -> [HistoryItem] { try operation { try $0.history(conversation: conversation, direct: direct, limit: limit) } }
    func deleteHistory(conversation: Data, direct: Bool) throws { try operation { try $0.deleteHistory(conversation: conversation, direct: direct) } }
    func ambiguousTarget(subject: Data, messageId: Data) throws -> Bool { try operation { try $0.ambiguousTarget(subject: subject, messageId: messageId) } }
    func prune() throws { try operation { try $0.prune(now: Int64(Date().timeIntervalSince1970)) } }
}
