import Foundation
import CryptoKit
import Security

// Record only adapter operation names and public error categories, never bytes.
@MainActor final class TestFileTrace { var failure = "none" }
@MainActor final class TracedTestFiles: IdentityStorage {
    private let base: AppleIdentityStorage, trace: TestFileTrace
    private let label: String
    init(_ folder: URL, _ label: String, _ trace: TestFileTrace) {
        base = AppleIdentityStorage(folder: folder); self.label = label; self.trace = trace
    }
    private func checked<T>(_ operation: String, _ work: () throws -> T) throws -> T {
        do { return try work() }
        catch { trace.failure = "\(label).\(operation): \(error)"; throw error }
    }
    func exists(_ file: IdentityFile) -> Bool { base.exists(file) }
    func hasArtifacts() -> Bool { base.hasArtifacts() }
    func read(_ file: IdentityFile) throws -> Data { try checked("read \(file)") { try base.read(file) } }
    func write(_ file: IdentityFile, _ bytes: Data) throws { try checked("write \(file)") { try base.write(file, bytes) } }
    func delete(_ file: IdentityFile) throws { try checked("delete \(file)") { try base.delete(file) } }
}

// Synthetic wrapping exists only in the test host. Production uses Secure Enclave.
@MainActor final class TestProtection: IdentityProtection {
    var unlocked = true
    let flag: URL
    private let key = SymmetricKey(data: Data(repeating: 71, count: 32))
    init(_ flag: URL) { self.flag = flag }
    func requireUnlocked() throws { if !unlocked { throw IdentityFailure.locked } }
    func exists() throws -> Bool { FileManager.default.fileExists(atPath: flag.path) }
    func create() throws { guard try !exists() else { throw IdentityFailure.recoveryRequired }; try Data([1]).write(to: flag, options: .atomic) }
    func delete() throws { if try exists() { try FileManager.default.removeItem(at: flag) } }
    func random(_ size: Int) throws -> Data { var data = Data(count: size); let rc = data.withUnsafeMutableBytes { SecRandomCopyBytes(kSecRandomDefault, size, $0.baseAddress!) }; guard rc == errSecSuccess else { throw IdentityFailure.unavailable }; return data }
    func seal(_ plain: Data, aad: Data) throws -> Data { guard try exists() else { throw IdentityFailure.invalidated }; return try AES.GCM.seal(plain, using: key, authenticating: aad).combined! }
    func open(_ cipher: Data, aad: Data) throws -> Data { guard try exists() else { throw IdentityFailure.invalidated }; return try AES.GCM.open(AES.GCM.SealedBox(combined: cipher), using: key, authenticating: aad) }
    func capabilities() throws -> IdentityCapabilities { IdentityCapabilities(wrapping: .software) }
}

@MainActor final class TestClock { var now: UInt64 = 0 }

/// Only the physical port is synthetic. All framing, admission, authorization,
/// crypto, queues, callbacks and UI state use production owners.
@MainActor final class TestRadio: MeshRadio, IOSGattPort {
    let callbacks: MeshRadioCallbacks
    let clock: TestClock
    var driver: IOSGattDriver!
    var appliedPower: TransportPower?
    var frames: [Data] = []
    var writable = true
    var submitted = 0
    var peer: UInt64?
    init(_ core: NativeTransport, _ callbacks: MeshRadioCallbacks, _ clock: TestClock) {
        self.callbacks = callbacks; self.clock = clock
        driver = IOSGattDriver(core: core, port: self, clock: { clock.now }, egress: callbacks.egress)
    }
    func start() { callbacks.state(.foreground) }
    func stop(_ reason: IOSRadioState) { driver.stop(); frames.removeAll() }
    func setPower(_ setting: TransportPowerSetting) { appliedPower = driver.power(setting: setting, battery: 80, charging: false, visible: 1) }
    func operation(_ work: (NativeTransport) throws -> TransportEffects) -> Bool { driver.operation(work) }
    func enqueue(_ id: UInt64, bytes: Data, traffic: TransportTraffic, cookie: UInt64) -> Bool { driver.enqueue(id, bytes: bytes, traffic: traffic, cookie: cookie) }
    func writable(_ id: UInt64) -> Bool { writable }
    func submit(_ id: UInt64, _ bytes: Data) -> Bool { guard writable else { return false }; frames.append(bytes); submitted += 1; return true }
    func close(_ id: UInt64) {}
    func event(_ id: UInt64, _ event: TransportEvent) { callbacks.event(id, event) }
    func ready(_ role: TransportRole) {
        peer = driver.reserve(address: Data(repeating: role == .central ? 1 : 2, count: 16), role: role)
        precondition(peer != nil); driver.connected(peer!, transmitCapacity: 512)
    }
    static func pump(_ a: TestRadio, _ b: TestRadio, seconds: Int) {
        for _ in 0..<seconds {
            a.clock.now += 1000
            if let id = a.peer { a.driver.ready(id) }; if let id = b.peer { b.driver.ready(id) }
            a.driver.tick(); b.driver.tick()
            let af = a.frames, bf = b.frames; a.frames.removeAll(); b.frames.removeAll()
            for frame in af { b.driver.value(b.peer!, frame) }
            for frame in bf { a.driver.value(a.peer!, frame) }
        }
    }
}

@MainActor final class TestDevice {
    let root: URL, clock: TestClock
    let fileTrace = TestFileTrace()
    let protection: TestProtection, staffProtection: TestProtection
    let identity: IdentityProvider, storage: EncryptedStorage, staff: StaffKeyVault
    var radio: TestRadio?
    var model: MeshModel!
    init(_ clock: TestClock = TestClock()) throws {
        self.clock = clock
        root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        protection = TestProtection(root.appendingPathComponent("identity-key"))
        staffProtection = TestProtection(root.appendingPathComponent("staff-key"))
        let folder = root.appendingPathComponent("database", isDirectory: true)
        let vault = StorageVault(folder: folder, files: TracedTestFiles(folder, "storage", fileTrace), protection: TestProtection(root.appendingPathComponent("storage-key")))
        staff = StaffKeyVault(files: AppleIdentityStorage(folder: root.appendingPathComponent("staff", isDirectory: true)), protection: staffProtection)
        identity = IdentityProvider(storage: TracedTestFiles(root.appendingPathComponent("identity", isDirectory: true), "identity", fileTrace), protection: protection, state: FeatureResetStore(storage: vault, staff: staff))
        storage = EncryptedStorage(identity: identity, vault: vault)
        reopen()
    }
    func reopen() {
        model?.stop()
        model = MeshModel(identity: identity, storage: storage, staff: staff, clock: { [clock] in clock.now }, makeRadio: { [weak self] core, callbacks in
            let radio = TestRadio(core, callbacks, self!.clock); self!.radio = radio; return radio
        })
    }
}
