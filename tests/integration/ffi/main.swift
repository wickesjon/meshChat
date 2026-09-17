import Foundation

func trace() throws -> [String] {
    let core = try Core(limits: Limits(maxLinks: 1, maxValueBytes: 64), instanceNonce: 42, monotonicMs: 100)
    guard case let .linkConnected(link) = try core.connected(capacities: Capacities(writeBytes: 20, notifyBytes: 12, receiveBytes: 30)).uiEvents.first else { fatalError("missing connection") }
    precondition(link == LinkHandle(instanceNonce: 42, generation: 1))
    _ = try core.handleEvent(event: .timeAdvanced(monotonicMs: 101))
    let bytes = Data([0, 127, 255])
    let sent = try core.prepareSend(link: link, path: .write, bytes: bytes).sends[0]
    precondition(sent.bytes == bytes && sent.link == link && sent.path == .write)
    guard case let .inboundObserved(observed, count) = try core.handleEvent(event: .inboundBytes(link: link, bytes: bytes)).uiEvents.first else { fatalError("missing observation") }
    precondition(observed == link && count == 3)
    do { _ = try core.handleEvent(event: .inboundBytes(link: link, bytes: Data())); fatalError("accepted empty") } catch CoreError.InvalidValue {}
    do { _ = try core.prepareSend(link: link, path: .notify, bytes: Data(repeating: 1, count: 13)); fatalError("accepted oversized") } catch CoreError.InvalidValue {}
    do { _ = try core.handleEvent(event: .timeAdvanced(monotonicMs: 99)); fatalError("accepted old time") } catch CoreError.TimeRegression {}
    _ = try core.handleEvent(event: .powerChanged(state: .lowPower))
    _ = try core.handleEvent(event: .disconnected(link: link))
    do { _ = try core.prepareSend(link: link, path: .write, bytes: bytes); fatalError("accepted stale send") } catch CoreError.UnknownLink {}
    do { _ = try core.handleEvent(event: .disconnected(link: link)); fatalError("accepted duplicate disconnect") } catch CoreError.UnknownLink {}
    guard case let .linkConnected(next) = try core.connected(capacities: Capacities(writeBytes: 20, notifyBytes: 12, receiveBytes: 30)).uiEvents.first else { fatalError("missing reconnection") }
    precondition(next.generation == 2)
    return [String(link.generation), sent.bytes.base64EncodedString(), String(next.generation)]
}
let first = try trace()
let second = try trace()
precondition(first == second)
print("Swift FFI lifecycle, binary round trip and errors passed")

// Synthetic SQL callback double for the production transport boundary. Real
// SQLCipher is tested separately; this fixture never enters shipping builds.
final class TransportSql: SqlDatabase, @unchecked Sendable {
    let process = Process()
    let input = Pipe()
    let output = Pipe()
    let lock = NSLock()
    init() throws {
        let root = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let folder = root.appendingPathComponent(".work/mc023/swift")
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

func transportTrace() throws {
    let da = try TransportSql(), db = try TransportSql()
    defer { da.finish(); db.finish() }
    let ia = try IdentityKeySession.importUnlocked(material: Data(repeating: 61, count: 64), generation: Data(repeating: 61, count: 16))
    let ib = try IdentityKeySession.importUnlocked(material: Data(repeating: 62, count: 64), generation: Data(repeating: 62, count: 16))
    defer { try! ia.invalidate(); try! ib.invalidate() }
    let pa = try ia.publicIdentity(), pb = try ib.publicIdentity()
    let sa = try EncryptedStore.open(db: da, generation: pa.generation, create: true, now: 200000)
    let sb = try EncryptedStore.open(db: db, generation: pb.generation, create: true, now: 200000)
    let a = try NativeTransport(store: sa, identity: pa, instanceNonce: 61, monotonicMs: 0)
    let b = try NativeTransport(store: sb, identity: pb, instanceNonce: 62, monotonicMs: 0)
    let bad = try a.admitConnection(address: Data(repeating: 3, count: 16), now: 0)
    do { _ = try a.nativeReady(admission: bad, role: .central, transmitBytes: 145, receiveBytes: 512, now: 0); fatalError("accepted small capacity") } catch TransportError.Invalid {}
    let al = try a.nativeReady(admission: a.admitConnection(address: Data(repeating: 1, count: 16), now: 0), role: .central, transmitBytes: 512, receiveBytes: 146, now: 0).link
    let bl = try b.nativeReady(admission: b.admitConnection(address: Data(repeating: 2, count: 16), now: 0), role: .peripheral, transmitBytes: 182, receiveBytes: 512, now: 0).link
    let ah = try a.tick(now: 0).sends[0], bh = try b.tick(now: 0).sends[0]
    precondition(ah.bytes.count == 59 && ah.path == .write && bh.path == .notify)
    _ = try a.receive(link: al, reportedBytes: UInt64(bh.bytes.count), bytes: bh.bytes, now: 0)
    _ = try b.receive(link: bl, reportedBytes: UInt64(ah.bytes.count), bytes: ah.bytes, now: 0)
    guard case let .admitted(_, tx, rx) = try a.complete(link: al, token: ah.token, success: true, now: 0).events[0] else { fatalError("no admission") }
    precondition(tx == 512 && rx == 146)
    _ = try b.complete(link: bl, token: bh.token, success: true, now: 0)
    _ = try a.prepareProof(link: al, provider: ia, now: 0)
    _ = try b.prepareProof(link: bl, provider: ib, now: 0)
    let ap = try a.tick(now: 1000).sends[0], bp = try b.tick(now: 1000).sends[0]
    precondition(ap.bytes.count == 71 && bp.bytes.count == 71)
    _ = try a.receive(link: al, reportedBytes: 71, bytes: bp.bytes, now: 1000)
    _ = try b.receive(link: bl, reportedBytes: 71, bytes: ap.bytes, now: 1000)
    _ = try a.complete(link: al, token: ap.token, success: true, now: 1000)
    _ = try b.complete(link: bl, token: bp.token, success: true, now: 1000)
    let vectorFile = URL(fileURLWithPath: FileManager.default.currentDirectoryPath).appendingPathComponent("tests/vectors/crypto/friend-v1.tsv")
    let vectorLine = try String(contentsOf: vectorFile, encoding: .utf8).split(separator: "\n").first { $0.hasPrefix("peer_chat\t") }!
    let hex = Array(vectorLine.split(separator: "\t")[1].utf8)
    let signed = stride(from: 0, to: hex.count, by: 2).map { UInt8(String(bytes: hex[$0..<$0+2], encoding: .utf8)!, radix: 16)! }
    var frame = Data([0, 0, UInt8(signed.count >> 8), UInt8(signed.count & 255)] + signed)
    frame[frame.count - 1] ^= 1
    _ = try b.receive(link: bl, reportedBytes: UInt64(frame.count), bytes: frame, now: 1000)
    let unverified = try b.observations(now: 1000)
    precondition(unverified.count == 1 && unverified[0].firstValidMs == nil)
    frame[frame.count - 1] ^= 1
    _ = try b.receive(link: bl, reportedBytes: UInt64(frame.count), bytes: frame, now: 1000)
    let activity = try b.observations(now: 1000)
    precondition(activity[0].firstValidMs == 1000 && activity[0].link == bl)
    _ = try a.disconnect(link: al, now: 1001)
    do { _ = try a.complete(link: al, token: ap.token, success: true, now: 1001); fatalError("accepted stale callback") } catch TransportError.Stale {}
    _ = try b.disconnect(link: bl, now: 1001)
    let stalled = try b.nativeReady(admission: b.admitConnection(address: Data(repeating: 4, count: 16), now: 1001), role: .peripheral, transmitBytes: 146, receiveBytes: 146, now: 1001).link
    _ = try b.tick(now: 1001)
    guard let last = try b.tick(now: 6001).events.last, case let .closed(link) = last else { fatalError("missing timeout") }
    precondition(link == stalled)
    let saver = try b.updatePower(setting: .saver, batteryPercent: 40, charging: false, visiblePeers: 4, now: 6001)
    precondition(saver.saver && saver.linkLimit == 3 && saver.scanOffMs == 50000 && saver.announceMs == 60000)
    _ = try b.updatePower(setting: .auto, batteryPercent: 41, charging: false, visiblePeers: 4, now: 6001)
    let waiting = try b.updatePower(setting: nil, batteryPercent: 41, charging: false, visiblePeers: 4, now: 66000)
    precondition(waiting.saver)
    let normal = try b.updatePower(setting: nil, batteryPercent: 41, charging: false, visiblePeers: 4, now: 66001)
    try b.configureBeacon(manual: false, autoWhileCharging: true)
    let beacon = try b.updatePower(setting: nil, batteryPercent: 80, charging: true, visiblePeers: 4, now: 66002)
    precondition(beacon.beacon && beacon.infra && beacon.linkLimit == 8)
    let unplugged = try b.updatePower(setting: nil, batteryPercent: 80, charging: false, visiblePeers: 4, now: 66003)
    precondition(!unplugged.beacon && !unplugged.infra)
    let beaconStatus = try b.beaconStatus(now: 66003)
    precondition(!beaconStatus.active && beaconStatus.allocatedCacheBytes <= 320 * 1024)

    precondition(!normal.saver && normal.linkLimit == 6)
    let observations = try b.observations(now: 66003)
    precondition(observations.isEmpty)
}
try transportTrace()
print("MC-023 Swift real HELLO/proof, capacity refusal, stale callback and timeout passed")

print("MC-024 Swift power policy, Auto hysteresis and bounded observations passed")

#if MC026_DRIVER_TESTS
try iosDriverChecks()
#endif

func channelTrace() throws {
    let db = try TransportSql(); defer { db.finish() }
    let identity = try IdentityKeySession.importUnlocked(material: Data(repeating: 125, count: 64), generation: Data(repeating: 125, count: 16))
    defer { try? identity.invalidate() }
    let publicId = try identity.publicIdentity()
    let store = try EncryptedStore.open(db: db, generation: publicId.generation, create: true, now: 200000)
    let channels = try NativeChannels(identity: publicId, now: 0)
    try channels.setCosmetics(supporter: true, rgb: 0x12abef)
    let first = try channels.compose(name: "#confessions", nickname: "Alice", avatar: 0x23, message: String(repeating: "🎉", count: 70), wall: 200000, now: 0)
    let second = try channels.compose(name: "#confessions", nickname: "Alice", avatar: 0x23, message: "Another", wall: 200000, now: 0)
    precondition(first.subdata(in: 12..<20) != second.subdata(in: 12..<20))
    let accepted = try channels.accept(store: store, receipt: ChannelReceipt(link: nil, bytes: first, intake: .unverified, own: true, wall: 200000, now: 0))
    precondition(accepted)
    let message = try channels.history(store: store, name: "#confessions", ownNickname: "Alice")[0]
    precondition(message.nickname == "Anonymous" && message.avatar == 0)
    precondition(!message.supporterHint && message.nicknameRgb == nil)
    let decorated = try channels.compose(name: "#general", nickname: "Alice", avatar: 1, message: "Cosmetic", wall: 200000, now: 0)
    _ = try channels.accept(store: store, receipt: ChannelReceipt(link: nil, bytes: decorated, intake: .unverified, own: false, wall: 200000, now: 0))
    let hint = try channels.history(store: store, name: "#general", ownNickname: "Bob")[0]
    precondition(hint.supporterHint && hint.nicknameRgb == 0x12abef && !hint.signed && hint.verifiedPetname == nil)
    let reaction = try channels.reaction(name: "#confessions", target: message.id, code: 3, remove: false, now: 0)
    precondition(reaction.subdata(in: 12..<20) == publicId.senderId)
    _ = try channels.accept(store: store, receipt: ChannelReceipt(link: nil, bytes: reaction, intake: .unverified, own: true, wall: 200000, now: 0))
    let result = try channels.history(store: store, name: "#confessions", ownNickname: "Alice")
    precondition(result[0].reactions[3] == 1)
    do { _ = try channels.compose(name: "#general", nickname: "Alice", avatar: 1, message: String(repeating: "🎉", count: 71), wall: 200000, now: 0); fatalError("accepted oversized message") } catch ChannelError.Invalid {}
    print("MC-028 Swift real channel codec, disposable posts, byte limits and persistent reactions PASS")
}
try channelTrace()

func messagingTrace() throws {
    let db = try TransportSql(); defer { db.finish() }
    let identity = try IdentityKeySession.importUnlocked(material: Data(repeating: 71, count: 64), generation: Data(repeating: 71, count: 16))
    let peer = try IdentityKeySession.importUnlocked(material: Data(repeating: 72, count: 64), generation: Data(repeating: 72, count: 16))
    defer { try? identity.invalidate(); try? peer.invalidate() }
    let own = try identity.publicIdentity()
    let store = try EncryptedStore.open(db: db, generation: own.generation, create: true, now: 200000)
    let core = try NativeTransport(store: store, identity: own, instanceNonce: 71, monotonicMs: 0)
    let code = try friendCode(identity: peer.publicIdentity(), nickname: "Bob / é")
    let proposal = try friendProposal(uri: code.uri)
    precondition(proposal.keys == code.keys && proposal.fingerprint.count == 79)
    let before = try core.friendCards(store: store, now: 0)
    precondition(before.isEmpty)
    _ = try core.confirmFriend(store: store, provider: identity, uri: code.uri, petname: "Local Bob", previous: nil, now: 0)
    let pin = try core.friendCards(store: store, now: 0)[0]
    precondition(pin.petname == "Local Bob" && !pin.fresh)
    _ = try core.changeFriend(store: store, friend: pin.handle, replace: true, now: 0)
    let held = try core.friendCards(store: store, now: 0)[0]
    precondition(held.replacing)
    _ = try core.changeFriend(store: store, friend: pin.handle, replace: false, now: 0)
    let removed = try core.friendCards(store: store, now: 0)
    let history = try core.directHistory(store: store, keys: pin.keys, now: 0, wall: 200000)
    precondition(removed.isEmpty && history.isEmpty)
    do { _ = try friendProposal(uri: "meshfest://friend/bad/key"); fatalError("malformed friend proposal accepted") } catch MessagingError.Invalid {}
    print("MC-029 Swift inert proposals, full tuple, opaque pin handle and replacement/removal PASS")
}
try messagingTrace()

func organizerTrace() throws {
    let source=try String(contentsOfFile:".work/mc030/fixtures/events.tsv",encoding:.utf8)
    let fixtures=Dictionary(uniqueKeysWithValues:source.split(separator:"\n").map {line in
        let p=line.split(separator:"\t",maxSplits:1);return (String(p[0]),String(p[1]))
    })
    let wall=Int64(Date().timeIntervalSince1970)
    let db=try TransportSql();defer {db.finish()}
    let identity=try IdentityKeySession.importUnlocked(material:Data(repeating:30,count:64),generation:Data(repeating:30,count:16))
    let own=try identity.publicIdentity()
    let store=try EncryptedStore.open(db:db,generation:own.generation,create:true,now:wall)
    let core=try NativeTransport(store:store,identity:own,instanceNonce:30,monotonicMs:0)
    let event=try eventProposal(uri:fixtures["event"]!)
    let before=try core.eventCards(store:store,wall:wall);precondition(before.isEmpty)
    do {_=try core.importStaffKey(store:store,uri:fixtures["staff"]!,now:0,wall:wall);fatalError("unadopted import")}
    catch OrganizerError.Authority {}
    try core.confirmEvent(store:store,uri:fixtures["event"]!,now:0,wall:wall)
    let cards=try core.eventCards(store:store,wall:wall);precondition(cards.count==1 && cards[0].active && event.fingerprint.count==79)
    let session=try core.importStaffKey(store:store,uri:fixtures["staff"]!,now:1000,wall:wall);session.invalidate()
    do {_=try core.importStaffKey(store:store,uri:fixtures["expired"]!,now:1000,wall:wall);fatalError("expired import")}
    catch OrganizerError.Authority {}
    try core.removeEvent(store:store,key:event.key,wall:wall)
    let removed=try core.eventCards(store:store,wall:wall);precondition(removed.isEmpty)
    print("MC-030 Swift production organizer adoption, protected-session boundary and expired import PASS; synthetic SQL")
}
try organizerTrace()
