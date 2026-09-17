import Foundation

@main struct OrganizerImportChecks {
    static func main() throws {
        let source = try String(contentsOfFile: ".work/mc041/fixtures.tsv", encoding: .utf8)
        let fixtures = Dictionary(uniqueKeysWithValues: source.split(separator: "\n").map {
            let pair = $0.split(separator: "\t", maxSplits: 1)
            return (String(pair[0]), String(pair[1]))
        })
        func imported(_ event: String, _ staff: String, _ now: Int64) throws -> Bool {
            let database = try TransportSql(); defer { database.finish() }
            let identity = try IdentityKeySession.importUnlocked(material: Data(repeating: 41, count: 64), generation: Data(repeating: 41, count: 16))
            let own = try identity.publicIdentity()
            let store = try EncryptedStore.open(db: database, generation: own.generation, create: true, now: now)
            do { return try probeOrganizerImport(store: store, own: own, event: event, staff: staff, wall: now) }
            catch { return false }
        }
        let event = fixtures["event"]!
        for name in ["staff_a", "staff_b"] {
            let staff = fixtures[name]!
            let valid = try imported(event, staff, 200000); precondition(valid)
            for (root, uri, now) in [(event, staff, Int64(201001)), (event, staff, Int64(199989)),
                                     (fixtures["other"]!, staff, Int64(200000)),
                                     (event, String(staff.prefix(upTo: staff.lastIndex(of: "/")!)) + "/" + String(repeating: "A", count: 43), Int64(200000))] {
                let bad = try imported(root, uri, now); precondition(!bad)
            }
        }
        let malformed = try imported(event, "meshfest://staff/bad/bad", 200000); precondition(!malformed)
        print("MC-041 Swift generated bundle imports and negative cases PASS; synthetic storage only")
    }
}
