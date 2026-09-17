import Foundation

@main
struct WireChecks {
    static func vectors(_ root: URL, _ name: String) throws -> [String: Data] {
        let text = try String(contentsOf: root.appendingPathComponent("tests/vectors/crypto/\(name)-v1.tsv"), encoding: .utf8)
        return Dictionary(uniqueKeysWithValues: text.split(separator: "\n").map { line in
            let fields = line.split(separator: "\t")
            let hex = Array(fields[1].utf8)
            let bytes = stride(from: 0, to: hex.count, by: 2).map {
                UInt8(String(bytes: hex[$0..<$0 + 2], encoding: .utf8)!, radix: 16)!
            }
            return (String(fields[0]), Data(bytes))
        })
    }
    static func main() throws {
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        let friend = try vectors(root, "friend")
        let dm = try vectors(root, "dm")
        let organizer = try vectors(root, "organizer")
        let expected = ["friend:8", "dm:17", "organizer:8"]
        let positive = try checkWireVectors(friendVectors: friend, dmVectors: dm, organizerVectors: organizer)
        precondition(positive == expected)
        for (family, key) in [(0, "peer_chat"), (1, "chat_2_1"), (2, "chat_a")] {
            var inputs = [friend, dm, organizer]
            var bytes = inputs[family][key]!
            bytes[bytes.count - 1] ^= 1
            inputs[family][key] = bytes
            do {
                _ = try checkWireVectors(friendVectors: inputs[0], dmVectors: inputs[1], organizerVectors: inputs[2])
                fatalError("altered reference vector accepted")
            } catch FixtureError.Mismatch {
                // A typed Rust error must survive the generated Swift binding.
            }
        }
        var renamedDm = dm
        var renamedBytes = renamedDm.removeValue(forKey: "chat_2_1")!
        renamedBytes[renamedBytes.count - 1] ^= 1
        renamedDm["unexpected"] = renamedBytes
        precondition(renamedDm.count == 17)
        do {
            _ = try checkWireVectors(friendVectors: friend, dmVectors: renamedDm, organizerVectors: organizer)
            fatalError("renamed expected fixture accepted")
        } catch FixtureError.Mismatch {
            // Same-count substitutions must not silently remove an expected case.
        }
        let recovered = try checkWireVectors(friendVectors: friend, dmVectors: dm, organizerVectors: organizer)
        precondition(recovered == expected)
        print("Swift public crypto vectors and corruption/error parity passed")
    }
}
