import Foundation
// Synthetic SQL callback fixture, never production storage.
final class TransportSql: SqlDatabase, @unchecked Sendable {
    let process = Process()
    let input = Pipe()
    let output = Pipe()
    let lock = NSLock()
    init() throws {
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let folder = root.appendingPathComponent(".work/mc041/swift-sql")
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["python3", "-B", root.appendingPathComponent("tests/integration/friends/sqlite_worker.py").path,
            folder.appendingPathComponent("transport-\(UUID().uuidString).sqlite").path]
        process.standardInput = input; process.standardOutput = output
        try process.run()
    }
    func hex(_ bytes: Data) -> String { bytes.map { String(format: "%02x", $0) }.joined() }
    func unhex(_ text: Substring) throws -> Data {
        let bytes = Array(text.utf8)
        guard bytes.count % 2 == 0 else { throw StorageError.Database }
        return try Data(stride(from: 0, to: bytes.count, by: 2).map {
            guard let byte = UInt8(String(decoding: bytes[$0..<($0 + 2)], as: UTF8.self), radix: 16) else { throw StorageError.Database }
            return byte
        })
    }
    func line() throws -> String {
        var bytes = Data()
        while let next = try output.fileHandleForReading.read(upToCount: 1), !next.isEmpty {
            if next[0] == 10 { return String(decoding: bytes, as: UTF8.self) }
            bytes.append(next)
            guard bytes.count < 1_000_000 else { throw StorageError.Database }
        }
        throw StorageError.Database
    }
    func call(_ mode: String, _ sql: String, _ values: [SqlValue], _ limit: UInt32) throws -> [SqlRow] {
        lock.lock(); defer { lock.unlock() }
        let args = values.map { value in
            switch value {
            case let .integer(value): return "i\(value)"
            case let .bytes(value): return "b" + hex(value)
            case let .text(value): return "t" + hex(Data(value.utf8))
            }
        }.joined(separator: " ")
        try input.fileHandleForWriting.write(contentsOf: Data("\(mode) \(limit) \(hex(Data(sql.utf8))) \(args)\n".utf8))
        let answer = try line()
        guard answer.hasPrefix("ok "), let count = Int(answer.dropFirst(3)), count >= 0, count <= 1024 else { throw StorageError.Database }
        return try (0..<count).map { _ in
            SqlRow(cells: try line().split(separator: " ").map { cell in
                switch cell.first {
                case "i": guard let value = Int64(cell.dropFirst()) else { throw StorageError.Database }; return .integer(value: value)
                case "b": return .bytes(value: try unhex(cell.dropFirst()))
                case "t": return .text(value: String(decoding: try unhex(cell.dropFirst()), as: UTF8.self))
                default: throw StorageError.Database
                }
            })
        }
    }
    func execute(sql: String, values: [SqlValue]) throws { _ = try call("x", sql, values, 0) }
    func query(sql: String, values: [SqlValue], limit: UInt32) throws -> [SqlRow] { try call("q", sql, values, limit) }
    func finish() { try? input.fileHandleForWriting.close(); process.waitUntilExit(); try? output.fileHandleForReading.close() }
}
