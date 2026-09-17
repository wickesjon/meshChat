import Foundation

@MainActor
private final class IOSPortDouble: IOSGattPort {
    var writable = true
    var accepted = true
    var frames: [(UInt64, Data)] = []
    var attempts = 0
    var closed: [UInt64] = []
    var events: [(UInt64, TransportEvent)] = []
    func writable(_ id: UInt64) -> Bool { writable }
    func submit(_ id: UInt64, _ bytes: Data) -> Bool {
        attempts += 1
        if accepted { frames.append((id, bytes)) } else { writable = false }
        return accepted
    }
    func close(_ id: UInt64) { closed.append(id) }
    func event(_ id: UInt64, _ event: TransportEvent) { events.append((id, event)) }
}
@MainActor
private final class IOSNode {
    var time: UInt64 = 0
    let port = IOSPortDouble()
    var driver: IOSGattDriver!
    init(_ core: NativeTransport) { driver = IOSGattDriver(core: core, port: port, clock: { [unowned self] in self.time }) }
    func connect(_ role: TransportRole, _ capacity: Int, address: UInt8 = 1) -> UInt64 {
        let id = driver.reserve(address: Data(repeating: address, count: 16), role: role)!
        driver.connected(id, transmitCapacity: capacity)
        return id
    }
    func received(_ bytes: Data) -> Bool {
        port.events.contains { if case let .received(_, value, _) = $0.1 { return value == bytes }; return false }
    }
    func finished(_ cookie: UInt64) -> Bool {
        port.events.contains { if case let .finished(_, c, status) = $0.1 { return c == cookie && status == .nativeComplete }; return false }
    }
}
@MainActor
private func withIOS(_ seed: UInt8, _ body: (IOSNode) throws -> Void) throws {
    let db = try TransportSql(); defer { db.finish() }
    let identity = try IdentityKeySession.importUnlocked(material: Data(repeating: seed, count: 64), generation: Data(repeating: seed, count: 16))
    defer { try? identity.invalidate() }
    let publicId = try identity.publicIdentity()
    let store = try EncryptedStore.open(db: db, generation: publicId.generation, create: true, now: 200000)
    let core = try NativeTransport.newIos(store: store, identity: publicId, instanceNonce: UInt64(seed), monotonicMs: 0)
    let n = IOSNode(core); defer { n.driver.stop() }
    try body(n)
}
private func baseFixture(_ name: String) throws -> Data {
    let text = try String(contentsOfFile: "tests/vectors/base/logical.txt", encoding: .utf8)
    let line = text.split(separator: "\n").first { $0.hasPrefix(name + " ") }!
    let hex = Array(line.split(separator: " ")[3].utf8)
    return Data(stride(from: 0, to: hex.count, by: 2).map {
        UInt8(String(decoding: hex[$0..<($0 + 2)], as: UTF8.self), radix: 16)!
    })
}
@MainActor
func iosDriverChecks() throws {
    try withIOS(100) { a in try withIOS(101) { b in
        let al = a.connect(.central, 512), bl = b.connect(.peripheral, 182)
        func exchange(_ sender: IOSNode, _ receiver: IOSNode, _ target: UInt64) {
            sender.driver.tick()
            let frames = sender.port.frames; sender.port.frames.removeAll()
            for (_, value) in frames { precondition(value.count <= 146); receiver.driver.value(target, value) }
        }
        exchange(a, b, bl); exchange(b, a, al)
        precondition(a.port.events.contains { if case .admitted = $0.1 { return true }; return false })
        precondition(b.port.events.contains { if case .admitted = $0.1 { return true }; return false })
        var now: UInt64 = 1000
        var cookie: UInt64 = 100
        for (sender, receiver, source, target) in [(a, b, al, bl), (b, a, bl, al)] {
            let cases: [(String, TransportTraffic)] = [("chat", .own), ("announce", .local),
                ("sync-request", .local), ("event-info", .own), ("reaction", .own),
                ("credential-request", .local), ("credential-offer", .own), ("signed-announce", .local)]
            for (name, traffic) in cases {
                var bytes = try baseFixture(name); cookie += 1
                bytes.replaceSubrange(4..<12, with: Data(repeating: UInt8(cookie), count: 8))
                a.time = now; b.time = now; a.driver.ready(al); b.driver.ready(bl)
                precondition(sender.driver.enqueue(source, bytes: bytes, traffic: traffic, cookie: cookie))
                for _ in 0..<8 {
                    exchange(sender, receiver, target)
                    if sender.finished(cookie) { break }
                    now += 1000; a.time = now; b.time = now
                }
                precondition(sender.finished(cookie), "missing native completion: \(name)")
                precondition(receiver.received(bytes), "missing bounded ingress: \(name)")
                now += 1000
            }
            // Transport type 1 uses the existing SYNC wrapper, not logical kind 4.
            let stored = try baseFixture("chat")
            var item = Data([0,1,0,0,0,0,0,0,0,0,UInt8(stored.count)])
            item.append(stored)
            for bytes in [item, Data([0,1,0,1,5,0,0,0,0,0,0])] {
                a.time = now; b.time = now; a.driver.ready(al); b.driver.ready(bl); cookie += 1
                precondition(sender.driver.enqueueSync(source, bytes: bytes, cookie: cookie))
                exchange(sender, receiver, target)
                precondition(sender.finished(cookie) && receiver.received(bytes))
                precondition(receiver.port.events.contains { if case let .received(_, value, intake) = $0.1 { return value == bytes && intake == .deferredSync }; return false })
                now += 1000
            }
        }
        // A queue-full refusal is held until readiness, without the Android 5s timeout.
        a.time = now; b.time = now; a.driver.ready(al); b.driver.ready(bl)
        a.port.accepted = false
        var chat = try baseFixture("chat"); chat[4] = 240
        precondition(a.driver.enqueue(al, bytes: chat, traffic: .own, cookie: 240))
        a.driver.tick(); let attempts = a.port.attempts
        a.time += 6000; a.driver.tick()
        precondition(a.driver.count == 1 && a.port.attempts == attempts && !a.finished(240))
        a.port.accepted = true; a.port.writable = true; a.driver.ready(al); a.driver.tick()
        precondition(a.finished(240) && a.port.frames.last?.1.suffix(chat.count) == chat)
        // A suspend/foreground time jump expires stale links before queued work resumes.
        let sent = a.port.attempts
        a.time += 15000; a.driver.tick()
        precondition(a.driver.count == 0 && a.port.attempts == sent && a.port.closed.contains(al))
        a.driver.ready(al); a.driver.value(al, Data([1,2,3])); a.driver.tick()
        precondition(a.driver.count == 0 && a.port.attempts == sent)
    } }
    try withIOS(102) { n in
        let small = n.connect(.central, 145)
        precondition(n.port.closed.contains(small) && n.port.frames.isEmpty)
        let id = n.driver.reserve(address: Data(repeating: 2, count: 16), role: .central)!
        for _ in 0..<30 { n.driver.value(id, Data(repeating: 1, count: 1024)) }
        precondition(n.driver.stagingDrops == 28)
        n.time = 30000; n.driver.tick()
        precondition(n.driver.count == 0 && n.port.closed.contains(id))
    }
    try withIOS(103) { n in
        let ids = (1...4).map { n.driver.reserve(address: Data(repeating: UInt8($0), count: 16), role: .central)! }
        precondition(n.driver.reserve(address: Data(repeating: 8, count: 16), role: .central) == nil)
        let p = n.driver.power(setting: .saver, battery: 10, charging: false, visible: 4)!
        precondition(p.linkLimit == 3 && n.driver.count == 3 && n.port.closed.contains(ids[3]))
        n.driver.resetLinks() // restoration must start fresh admission/HELLO
        precondition(n.driver.count == 0 && n.port.closed.count == 4)
        n.driver.stop(); n.driver.connected(ids[0], transmitCapacity: 512); n.driver.tick()
        precondition(n.driver.stopped && n.port.frames.isEmpty)
    }
    try withIOS(104) { n in
        let id = n.connect(.peripheral, 182)
        n.driver.connected(id, transmitCapacity: 146)
        precondition(n.driver.count == 0 && n.port.closed.contains(id))
    }
    print("MC-026 production Swift adapter: both roles/base kinds, SYNC wrappers, readiness/backpressure, caps, stale callbacks and lifecycle cleanup PASS")
}
