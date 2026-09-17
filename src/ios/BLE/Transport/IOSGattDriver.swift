import Foundation

/// Main-queue native operations; protocol ownership remains in Rust.
@MainActor
protocol IOSGattPort: AnyObject {
    func writable(_ id: UInt64) -> Bool
    func submit(_ id: UInt64, _ bytes: Data) -> Bool
    func close(_ id: UInt64)
    func event(_ id: UInt64, _ event: TransportEvent)
}

@MainActor
final class IOSGattDriver {
    private final class Peer {
        let id: UInt64, admission: UInt64, born: UInt64
        let role: TransportRole
        let address: Data
        var link: LinkHandle?
        var capacity = 0
        var early: [(UInt64, Data)] = []
        init(_ id: UInt64, _ admission: UInt64, _ born: UInt64, _ role: TransportRole, _ address: Data) {
            self.id = id; self.admission = admission; self.born = born; self.role = role; self.address = address
        }
    }
    private let core: NativeTransport
    private unowned let port: any IOSGattPort
    private let clock: @MainActor () -> UInt64
    private let egress: ((TransportSend, () -> Bool) throws -> Bool)?
    private var peers: [UInt64: Peer] = [:]
    private var serial: UInt64 = 0
    private var limit = 4
    private var blocked: [Data: UInt64] = [:]
    private(set) var stopped = false
    private(set) var stagingDrops: UInt64 = 0
    var count: Int { peers.count }
    var ids: [UInt64] { peers.keys.sorted() }

    /// Supply NativeTransport.newIos backed by the application's protected store.
    init(core: NativeTransport, port: any IOSGattPort, clock: @escaping @MainActor () -> UInt64,
         egress: ((TransportSend, () -> Bool) throws -> Bool)? = nil) {
        self.core = core; self.port = port; self.clock = clock; self.egress = egress
    }
    private func guarded<T>(_ fallback: T, _ body: () throws -> T) -> T {
        guard !stopped else { return fallback }
        do { return try body() }
        catch let TransportError.Refused(_, effects) { apply(effects); return fallback }
        catch TransportError.Busy { return fallback }
        catch TransportError.Invalid { return fallback }
        catch TransportError.Stale { return fallback }
        catch { stop(); return fallback }
    }
    func reserve(address: Data, role: TransportRole) -> UInt64? {
        guarded(nil) {
            try advance()
            guard blocked[address] == nil, blocked.count < 64 else { return nil }
            guard peers.count < limit, !peers.values.contains(where: { $0.address == address && $0.role == role }) else { return nil }
            guard serial < UInt64.max else { stop(); return nil }
            let permit = try core.admitConnection(address: address, now: clock())
            serial += 1
            peers[serial] = Peer(serial, permit, clock(), role, address)
            return serial
        }
    }
    /// TX is queried from CoreBluetooth. RX=512 is this receiver's buffer ceiling,
    /// not an invented negotiated MTU; the peer supplies its actual native TX.
    func connected(_ id: UInt64, transmitCapacity: Int) {
        guarded(()) {
            try advance()
            guard let p = peers[id] else { return }
            let capacity = min(transmitCapacity, 512)
            guard capacity >= 146, p.capacity == 0 || p.capacity == capacity else { try release(id); return }
            if p.link != nil { return }
            p.capacity = capacity
            do {
                let c = try core.nativeReady(admission: p.admission, role: p.role,
                    transmitBytes: UInt16(capacity), receiveBytes: 512, now: clock())
                p.link = c.link; apply(c.effects)
                let staged = p.early; p.early.removeAll()
                for (reported, value) in staged where peers[id] === p {
                    apply(try core.receive(link: c.link, reportedBytes: reported, bytes: value, now: clock()))
                }
            } catch {
                try release(id)
                throw error
            }
        }
    }
    func value(_ id: UInt64, _ bytes: Data) {
        guarded(()) {
            try advance()
            guard let p = peers[id] else { return }
            let reported = UInt64(bytes.count)
            let bounded = bytes.count <= 512 ? bytes : Data()
            if let link = p.link { apply(try core.receive(link: link, reportedBytes: reported, bytes: bounded, now: clock())) }
            else if p.early.count < 2 { p.early.append((reported, bounded)) }
            else { if stagingDrops < UInt64.max { stagingDrops += 1 } }
        }
    }
    func ready(_ id: UInt64) {
        guarded(()) {
            try advance()
            guard let link = peers[id]?.link else { return }
            try core.readiness(link: link, now: clock())
        }
    }
    func enqueue(_ id: UInt64, bytes: Data, traffic: TransportTraffic, cookie: UInt64) -> Bool {
        guarded(false) {
            try advance()
            guard let link = peers[id]?.link else { return false }
            apply(try core.enqueue(link: link, bytes: bytes, traffic: traffic, cookie: cookie, now: clock()))
            return true
        }
    }
    func enqueueSync(_ id: UInt64, bytes: Data, cookie: UInt64) -> Bool {
        guarded(false) {
            try advance()
            guard let link = peers[id]?.link else { return false }
            apply(try core.enqueueSync(link: link, bytes: bytes, cookie: cookie, now: clock()))
            return true
        }
    }
    /// A short-lived protected signing session is invalidated on every path.
    func proof(_ id: UInt64, provider: IdentityKeySession) -> Bool {
        defer { try? provider.invalidate() }
        return guarded(false) {
            guard let link = peers[id]?.link else { return false }
            apply(try core.prepareProof(link: link, provider: provider, now: clock()))
            return true
        }
    }
    func power(setting: TransportPowerSetting?, battery: UInt8, charging: Bool, visible: UInt16) -> TransportPower? {
        guarded(nil) {
            try advance()
            let p = try core.updatePower(setting: setting, batteryPercent: battery, charging: charging,
                visiblePeers: min(visible, 64), now: clock())
            limit = Int(p.linkLimit)
            for id in ids where peers[id].map({ p.cancelledAdmissions.contains($0.admission) }) == true { try release(id) }
            apply(p.effects)
            return p
        }
    }
    func tick() { guarded(()) { try advance() } }
    @discardableResult
    func operation(_ work: (NativeTransport) throws -> TransportEffects) -> Bool {
        guarded(false) { apply(try work(core)); return true }
    }
    private func advance() throws {
        let now = clock()
        blocked = blocked.filter { now < $0.value }
        let observations = try core.observations(now: now)
        for id in ids {
            guard let p = peers[id] else { continue }
            let first = observations.first(where: { $0.link == p.link })?.firstValidMs
            if now >= p.born + 20_000 && (first == nil || first! > p.born + 20_000) {
                if blocked.count < 64 { blocked[p.address] = now + 300_000 }
                try release(id)
            }
            else if let link = p.link { try core.setWritable(link: link, writable: port.writable(id), now: now) }
        }
        apply(try core.tick(now: now))
    }
    private func apply(_ batch: TransportEffects) {
        for e in batch.events {
            let link: LinkHandle
            switch e {
            case let .admitted(l, _, _), let .received(l, _, _), let .finished(l, _, _), let .closed(l): link = l
            }
            guard let p = peers.values.first(where: { $0.link == link }) else { continue }
            if case .closed = e { peers.removeValue(forKey: p.id); p.early.removeAll(); port.close(p.id) }
            port.event(p.id, e)
        }
        for send in batch.sends {
            guard let p = peers.values.first(where: { $0.link == send.link }) else { continue }
            guard send.bytes.count <= p.capacity else { stop(); return }
            do {
                // CoreBluetooth is called on this same serialized queue. False
                // means queue-full/readiness loss, never successful delivery.
                var accepted = false
                if port.writable(p.id) {
                    if let egress { accepted = try egress(send) { port.submit(p.id, send.bytes) } }
                    else {
                        // Probe consumers may omit a gate, but authenticated
                        // application traffic must never bypass protected recheck.
                        guard try !core.messageNeedsAuthorization(link: send.link, token: send.token) else { stop(); return }
                        accepted = port.submit(p.id, send.bytes)
                    }
                }
                if accepted {
                    apply(try core.complete(link: send.link, token: send.token, success: true, now: clock()))
                } else { apply(try core.backpressure(link: send.link, token: send.token, now: clock())) }
            } catch { stop(); return }
        }
    }
    func lost(_ id: UInt64) { guarded(()) { try release(id) } }
    private func release(_ id: UInt64) throws {
        guard let p = peers.removeValue(forKey: id) else { return }
        p.early.removeAll()
        defer { port.close(id) }
        if let link = p.link {
            let effects = try core.disconnect(link: link, now: clock())
            for e in effects.events { port.event(id, e) }
        } else { try core.cancelConnection(admission: p.admission, now: clock()) }
    }
    /// Restoration drops old logical state without resetting node admission credit.
    func resetLinks() { for id in ids { lost(id) } }
    /// Drop the affected role before rebuilding its native service/manager. The
    /// other role remains owned; callbacks from the discarded IDs stay stale.
    func restore(_ role: TransportRole, rebuild: () -> Void) {
        for id in ids where peers[id]?.role == role { lost(id) }
        rebuild()
    }
    func stop() {
        guard !stopped else { return }
        stopped = true
        for id in ids { try? release(id) }
    }
}
