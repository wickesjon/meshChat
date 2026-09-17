#if os(iOS)
import Foundation
import UIKit
import CryptoKit
import Darwin
@preconcurrency import CoreBluetooth

@MainActor
enum IOSRadioState: Equatable {
    case stopped, starting, foreground, backgroundLimited, restoring
    case permissionRequired, radioOff, locked, unavailable
}

/// Production service UUIDs, deliberately separate from the MC-004 probe.
@MainActor
private enum MeshGattIds {
    static let service = CBUUID(string: "c11b1d76-75da-4ae0-b0fe-cb273609c526")
    static let write = CBUUID(string: "78db2e71-ff31-46e0-8a8e-371f8199bcdc")
    static let info = CBUUID(string: "ac933506-2294-4d92-8a0c-58d9f23acfb3")
    static let notify = CBUUID(string: "87bbc0ab-e60a-4802-b45f-445ed492cf30")
}

/// Each peripheral delegate carries the driver's generation and manager epoch.
@MainActor
private final class IOSPeripheralCallbacks: NSObject, @preconcurrency CBPeripheralDelegate {
    weak var owner: IOSGattRadio?
    weak var manager: CBCentralManager?
    let id: UInt64
    init(_ owner: IOSGattRadio, _ manager: CBCentralManager, _ id: UInt64) {
        self.owner = owner; self.manager = manager; self.id = id
    }
    private func forward(_ p: CBPeripheral, _ body: (IOSGattRadio) -> Void) {
        guard let owner, let manager, owner.accepts(id, p, manager) else { return }; body(owner)
    }
    func peripheral(_ p: CBPeripheral, didDiscoverServices error: Error?) {
        forward(p) { $0.peripheral(p, didDiscoverServices: error) }
    }
    func peripheral(_ p: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
        forward(p) { $0.peripheral(p, didDiscoverCharacteristicsFor: service, error: error) }
    }
    func peripheral(_ p: CBPeripheral, didUpdateNotificationStateFor c: CBCharacteristic, error: Error?) {
        forward(p) { $0.peripheral(p, didUpdateNotificationStateFor: c, error: error) }
    }
    func peripheral(_ p: CBPeripheral, didUpdateValueFor c: CBCharacteristic, error: Error?) {
        forward(p) { $0.peripheral(p, didUpdateValueFor: c, error: error) }
    }
    func peripheralIsReady(toSendWriteWithoutResponse p: CBPeripheral) {
        forward(p) { $0.peripheralIsReady(toSendWriteWithoutResponse: p) }
    }
    func peripheral(_ p: CBPeripheral, didModifyServices services: [CBService]) {
        forward(p) { $0.peripheral(p, didModifyServices: services) }
    }
}

@MainActor
final class IOSGattRadio: NSObject, IOSGattPort, @preconcurrency CBCentralManagerDelegate,
    @preconcurrency CBPeripheralDelegate, @preconcurrency CBPeripheralManagerDelegate {
    private struct Client {
        let id: UInt64
        let peripheral: CBPeripheral
        var write: CBCharacteristic?
        var notify: CBCharacteristic?
        var info: CBCharacteristic?
        var capacity = 0
        var waitingForReadiness = false
        var callbacks: IOSPeripheralCallbacks?
    }
    private struct Subscriber { let id: UInt64; let central: CBCentral; let capacity: Int }
    private struct Seen { let peripheral: CBPeripheral; var at: UInt64; var retry: UInt64 }
    private var central: CBCentralManager!
    private var server: CBPeripheralManager!
    private var tx: CBMutableCharacteristic?
    private var rx: CBMutableCharacteristic?
    private var clients: [UInt64: Client] = [:]
    private var subscribers: [UInt64: Subscriber] = [:]
    private var seen: [UUID: Seen] = [:]
    private var driver: IOSGattDriver!
    private var task: Task<Void, Never>?
    private var observers: [NSObjectProtocol] = []
    private var published = false
    private var publishing = false
    private var notifyReady = true
    private var resettingCentral = false
    private var resettingServer = false
    private var running = false
    private var centralRestart: UInt64 = 0
    private var serverRestart: UInt64 = 0
    private var lastPolicy: UInt64 = 0
    private var powerChoice: TransportPowerSetting?
    private(set) var appliedPower: TransportPower?
    private(set) var state: IOSRadioState = .stopped
    private let receiveEvent: (UInt64, TransportEvent) -> Void
    private let changed: (IOSRadioState) -> Void
    private let catchUp: ([UInt64]) -> Void
    private let protectedAvailable: () -> Bool

    /// The feature owner supplies a fresh iOS core and protected-provider check.
    /// No unlocked identity session or plaintext fallback is retained here.
    init(core: NativeTransport, protectedAvailable: @escaping () -> Bool,
         event: @escaping (UInt64, TransportEvent) -> Void,
         state: @escaping (IOSRadioState) -> Void,
         catchUp: @escaping ([UInt64]) -> Void) {
        self.protectedAvailable = protectedAvailable; receiveEvent = event; changed = state; self.catchUp = catchUp
        super.init()
        driver = IOSGattDriver(core: core, port: self, clock: Self.now)
    }
    static func now() -> UInt64 {
        var info = mach_timebase_info_data_t()
        mach_timebase_info(&info)
        return UInt64(Double(mach_continuous_time()) * Double(info.numer) / Double(info.denom) / 1_000_000)
    }
    private func status(_ value: IOSRadioState) { if state != value { state = value; changed(value) } }
    private func available() -> Bool {
        guard running else { return false }
        guard UIApplication.shared.isProtectedDataAvailable && protectedAvailable() else { stop(.locked); return false }
        if CBManager.authorization == .denied || CBManager.authorization == .restricted { stop(.permissionRequired); return false }
        return true
    }
    func start() {
        guard !running, !driver.stopped else { return }
        running = true
        guard available() else { return }
        status(.starting)
        UIDevice.current.isBatteryMonitoringEnabled = true
        makeCentral(); makeServer()
        let center = NotificationCenter.default
        for name in [UIApplication.didBecomeActiveNotification, UIApplication.didEnterBackgroundNotification,
                     UIApplication.protectedDataWillBecomeUnavailableNotification, UIApplication.willTerminateNotification] {
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self] notification in
                MainActor.assumeIsolated { self?.lifecycle(notification.name) }
            })
        }
        task = Task { @MainActor [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 250_000_000)
                guard !Task.isCancelled, let self, self.running else { return }
                self.pulse()
            }
        }
    }
    private func makeCentral() {
        central = CBCentralManager(delegate: self, queue: .main,
            options: [CBCentralManagerOptionRestoreIdentifierKey: "meshchat-v1-central"])
    }
    private func makeServer() {
        published = false; publishing = false; tx = nil; rx = nil; notifyReady = true
        server = CBPeripheralManager(delegate: self, queue: .main,
            options: [CBPeripheralManagerOptionRestoreIdentifierKey: "meshchat-v1-peripheral"])
    }
    private func lifecycle(_ name: Notification.Name) {
        if name == UIApplication.protectedDataWillBecomeUnavailableNotification { stop(.locked); return }
        if name == UIApplication.willTerminateNotification { stop(); return }
        guard available() else { return }
        pulse()
        if name == UIApplication.didBecomeActiveNotification { catchUp(driver.ids) }
    }
    func setPower(_ setting: TransportPowerSetting) { powerChoice = setting; lastPolicy = 0; pulse() }
    private func pulse() {
        guard available() else { return }
        for c in Array(clients.values) where c.capacity > 0 {
            if min(c.peripheral.maximumWriteValueLength(for: .withoutResponse), 512) != c.capacity { driver.lost(c.id) }
        }
        for s in Array(subscribers.values) where min(s.central.maximumUpdateValueLength, 512) != s.capacity { driver.lost(s.id) }
        driver.tick()
        guard !driver.stopped else { stop(.unavailable); return }
        let now = Self.now()
        if central == nil && now >= centralRestart { makeCentral() }
        if server == nil && now >= serverRestart { makeServer() }
        if lastPolicy == 0 || now >= lastPolicy + 1000 {
            lastPolicy = now
            let battery = UIDevice.current.batteryLevel
            guard battery >= 0 && battery <= 1 else { stop(.unavailable); return }
            let charge = UIDevice.current.batteryState
            appliedPower = driver.power(setting: powerChoice, battery: UInt8((battery * 100).rounded()),
                charging: charge == .charging || charge == .full, visible: UInt16(min(64, seen.count + subscribers.count)))
            powerChoice = nil
            seen = seen.filter { now < $0.value.at + 300_000 || now < $0.value.retry }
            if let central, central.state == .poweredOn {
                let scan = appliedPower?.saver != true || now % 60_000 < 10_000
                if scan && !central.isScanning { central.scanForPeripherals(withServices: [MeshGattIds.service]) }
                if !scan && central.isScanning { central.stopScan() }
                for candidate in seen.values.sorted(by: { $0.at > $1.at }) where now >= candidate.retry {
                    if driver.count >= Int(appliedPower?.linkLimit ?? 4) { break }
                    connect(candidate.peripheral)
                }
            }
        }
        if central?.state == .poweredOn && server?.state == .poweredOn && published {
            status(UIApplication.shared.applicationState == .active ? .foreground : .backgroundLimited)
        }
    }
    private func digest(_ id: UUID) -> Data { Data(SHA256.hash(data: Data(id.uuidString.utf8)).prefix(16)) }
    private func connect(_ peripheral: CBPeripheral) {
        guard let central, Self.now() >= centralRestart, !clients.values.contains(where: { $0.peripheral.identifier == peripheral.identifier }),
              let id = driver.reserve(address: digest(peripheral.identifier), role: .central) else { return }
        guard self.central === central, running else { driver.lost(id); return }
        let callbacks = IOSPeripheralCallbacks(self, central, id)
        var client = Client(id: id, peripheral: peripheral)
        client.callbacks = callbacks; clients[id] = client
        if var record = seen[peripheral.identifier] { record.retry = Self.now() + 5000; seen[peripheral.identifier] = record }
        peripheral.delegate = callbacks
        if peripheral.state == .connected { peripheral.discoverServices([MeshGattIds.service]) }
        else { central.connect(peripheral) }
    }
    fileprivate func accepts(_ id: UInt64, _ peripheral: CBPeripheral, _ manager: CBCentralManager) -> Bool {
        running && central === manager && clients[id]?.peripheral === peripheral
    }
    private func client(_ peripheral: CBPeripheral) -> Client? { clients.values.first { $0.peripheral === peripheral } }
    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        guard self.central === central, available() else { return }
        if central.state == .poweredOn { pulse() }
        else if central.state != .unknown { stop(central.state == .unauthorized ? .permissionRequired : .radioOff) }
    }
    func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral,
                        advertisementData: [String: Any], rssi RSSI: NSNumber) {
        guard self.central === central, available() else { return }
        let now = Self.now()
        if var old = seen[peripheral.identifier] { old.at = now; seen[peripheral.identifier] = old }
        else if seen.count < 64 { seen[peripheral.identifier] = Seen(peripheral: peripheral, at: now, retry: 0) }
        guard let record = seen[peripheral.identifier], now >= record.retry else { return }
        connect(peripheral)
    }
    func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        guard self.central === central, available(), client(peripheral) != nil else { return }
        peripheral.discoverServices([MeshGattIds.service])
    }
    func centralManager(_ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?) {
        guard self.central === central, let c = client(peripheral) else { return }; driver.lost(c.id)
    }
    func centralManager(_ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?) {
        guard self.central === central, let c = client(peripheral) else { return }; driver.lost(c.id)
    }
    func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        guard available(), let c = client(peripheral) else { return }
        guard error == nil, let services = peripheral.services,
              let service = services.first(where: { $0.uuid == MeshGattIds.service }) else { driver.lost(c.id); return }
        peripheral.discoverCharacteristics([MeshGattIds.write, MeshGattIds.notify, MeshGattIds.info], for: service)
    }
    func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
        guard available(), var c = client(peripheral), service.uuid == MeshGattIds.service else { return }
        let chars = service.characteristics ?? []
        guard error == nil, let write = chars.first(where: { $0.uuid == MeshGattIds.write && $0.properties.contains(.writeWithoutResponse) }),
              let notify = chars.first(where: { $0.uuid == MeshGattIds.notify && $0.properties.contains(.notify) }),
              let info = chars.first(where: { $0.uuid == MeshGattIds.info && $0.properties.contains(.read) }) else { driver.lost(c.id); return }
        c.write = write; c.notify = notify; c.info = info; clients[c.id] = c
        peripheral.readValue(for: info)
    }
    func peripheral(_ peripheral: CBPeripheral, didUpdateNotificationStateFor characteristic: CBCharacteristic, error: Error?) {
        guard available(), var c = client(peripheral), c.notify === characteristic else { return }
        guard error == nil && characteristic.isNotifying else { driver.lost(c.id); return }
        c.capacity = min(peripheral.maximumWriteValueLength(for: .withoutResponse), 512); clients[c.id] = c
        driver.connected(c.id, transmitCapacity: c.capacity)
    }
    func peripheral(_ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?) {
        guard available(), let c = client(peripheral) else { return }
        if c.info === characteristic {
            guard error == nil, characteristic.value == Data([1, 2, 0]), let notify = c.notify else { driver.lost(c.id); return }
            peripheral.setNotifyValue(true, for: notify); return
        }
        guard c.notify === characteristic else { return }
        guard error == nil, let bytes = characteristic.value else { driver.lost(c.id); return }
        driver.value(c.id, bytes)
    }
    func peripheralIsReady(toSendWriteWithoutResponse peripheral: CBPeripheral) {
        guard available(), var c = client(peripheral) else { return }
        c.waitingForReadiness = false; clients[c.id] = c; driver.ready(c.id)
    }
    func peripheral(_ peripheral: CBPeripheral, didModifyServices invalidatedServices: [CBService]) {
        guard let c = client(peripheral), invalidatedServices.contains(where: { $0.uuid == MeshGattIds.service }) else { return }
        driver.lost(c.id)
    }
    func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        guard server === peripheral, available() else { return }
        if peripheral.state == .poweredOn { publish() }
        else if peripheral.state != .unknown { stop(peripheral.state == .unauthorized ? .permissionRequired : .radioOff) }
    }
    private func publish() {
        guard !published, !publishing else { return }
        tx = CBMutableCharacteristic(type: MeshGattIds.write, properties: [.writeWithoutResponse], value: nil, permissions: [.writeable])
        rx = CBMutableCharacteristic(type: MeshGattIds.notify, properties: [.notify], value: nil, permissions: [])
        let service = CBMutableService(type: MeshGattIds.service, primary: true)
        let info = CBMutableCharacteristic(type: MeshGattIds.info, properties: [.read], value: Data([1, 2, 0]), permissions: [.readable])
        service.characteristics = [tx!, rx!, info]; publishing = true; server.add(service)
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, didAdd service: CBService, error: Error?) {
        guard server === peripheral, available(), publishing, service.uuid == MeshGattIds.service else { return }
        publishing = false
        guard error == nil else { stop(.unavailable); return }
        published = true; server.startAdvertising([CBAdvertisementDataServiceUUIDsKey: [MeshGattIds.service]])
    }
    func peripheralManagerDidStartAdvertising(_ peripheral: CBPeripheralManager, error: Error?) {
        guard server === peripheral, available() else { return }
        if error != nil { stop(.unavailable) } else { pulse() }
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, central: CBCentral, didSubscribeTo characteristic: CBCharacteristic) {
        guard server === peripheral, available(), characteristic === rx else { return }
        guard !subscribers.values.contains(where: { $0.central.identifier == central.identifier }),
              let id = driver.reserve(address: digest(central.identifier), role: .peripheral) else { resetServer(); return }
        guard server === peripheral, running else { driver.lost(id); return }
        let capacity = min(central.maximumUpdateValueLength, 512)
        subscribers[id] = Subscriber(id: id, central: central, capacity: capacity)
        driver.connected(id, transmitCapacity: capacity)
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, central: CBCentral, didUnsubscribeFrom characteristic: CBCharacteristic) {
        guard server === peripheral, characteristic === rx,
              let p = subscribers.values.first(where: { $0.central === central }) else { return }; driver.lost(p.id)
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveWrite requests: [CBATTRequest]) {
        guard server === peripheral, let first = requests.first, available() else { return }
        // CoreBluetooth requires one response for the entire callback, even
        // though the central's write-without-response has no completion callback.
        guard requests.count <= 64, requests.allSatisfy({ request in
            request.characteristic === tx && request.offset == 0 &&
                request.value.map({ !$0.isEmpty && $0.count <= 512 }) == true &&
                subscribers.values.contains(where: { $0.central === request.central })
        }) else { peripheral.respond(to: first, withResult: .requestNotSupported); return }
        peripheral.respond(to: first, withResult: .success)
        for request in requests {
            if let p = subscribers.values.first(where: { $0.central === request.central }), let bytes = request.value {
                driver.value(p.id, bytes)
            }
        }
    }
    func peripheralManagerIsReady(toUpdateSubscribers peripheral: CBPeripheralManager) {
        guard server === peripheral, available() else { return }
        notifyReady = true
        for id in Array(subscribers.keys) { driver.ready(id) }
    }
    func writable(_ id: UInt64) -> Bool {
        if var c = clients[id] {
            guard c.capacity >= 146 && c.peripheral.state == .connected && !c.waitingForReadiness else { return false }
            if !c.peripheral.canSendWriteWithoutResponse {
                c.waitingForReadiness = true; clients[id] = c; return false
            }
            return true
        }
        return subscribers[id] != nil && notifyReady && published
    }
    func submit(_ id: UInt64, _ bytes: Data) -> Bool {
        if let c = clients[id], let characteristic = c.write, writable(id), bytes.count <= c.capacity {
            c.peripheral.writeValue(bytes, for: characteristic, type: .withoutResponse); return true
        }
        guard let p = subscribers[id], let rx, writable(id), bytes.count <= p.capacity else { return false }
        let accepted = server.updateValue(bytes, for: rx, onSubscribedCentrals: [p.central])
        if !accepted { notifyReady = false }
        return accepted
    }
    func event(_ id: UInt64, _ event: TransportEvent) { receiveEvent(id, event) }
    func enqueue(_ id: UInt64, bytes: Data, traffic: TransportTraffic, cookie: UInt64) -> Bool {
        guard available() else { return false }; return driver.enqueue(id, bytes: bytes, traffic: traffic, cookie: cookie)
    }
    func proof(_ id: UInt64, provider: IdentityKeySession) -> Bool {
        guard available() else { try? provider.invalidate(); return false }; return driver.proof(id, provider: provider)
    }
    func enqueueSync(_ id: UInt64, bytes: Data, cookie: UInt64) -> Bool {
        guard available() else { return false }; return driver.enqueueSync(id, bytes: bytes, cookie: cookie)
    }
    func close(_ id: UInt64) {
        if let c = clients.removeValue(forKey: id) {
            c.peripheral.delegate = nil
            if var record = seen[c.peripheral.identifier] { record.retry = Self.now() + 5000; seen[c.peripheral.identifier] = record }
            central?.cancelPeripheralConnection(c.peripheral)
            resetCentral()
        }
        if subscribers.removeValue(forKey: id) != nil { resetServer() }
    }
    /// Managers do not expose per-connection callback generations. Replacing the
    /// affected role's entire epoch avoids attributing late callbacks to reuse.
    private func resetCentral() {
        guard !resettingCentral else { return }; resettingCentral = true
        for id in Array(clients.keys) { driver.lost(id) }
        clients.removeAll()
        central?.stopScan(); central?.delegate = nil; central = nil
        seen.removeAll()
        centralRestart = Self.now() + 5000
        if running { status(.restoring) }
        resettingCentral = false
    }
    private func resetServer() {
        guard !resettingServer else { return }; resettingServer = true
        driver.restore(.peripheral) {
            subscribers.removeAll()
            server?.stopAdvertising(); server?.removeAllServices(); server?.delegate = nil; server = nil
            serverRestart = Self.now() + 5000
            if running { status(.restoring) }
        }
        resettingServer = false
    }
    func centralManager(_ central: CBCentralManager, willRestoreState dict: [String: Any]) {
        guard self.central === central, available() else { return }
        status(.restoring)
        // No serialized Rust session or unlocked provider is restored. Bound the
        // native set and start fresh HELLO/admission on the current protected core.
        for peripheral in dict[CBCentralManagerRestoredStatePeripheralsKey] as? [CBPeripheral] ?? [] {
            if driver.count < 4 { connect(peripheral) }
            if client(peripheral) == nil { central.cancelPeripheralConnection(peripheral) }
        }
        catchUp(driver.ids)
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, willRestoreState dict: [String: Any]) {
        guard server === peripheral, available() else { return }
        status(.restoring)
        // Restored characteristics/subscribers have no current logical generation.
        // Discard them and publish fresh objects before accepting new input.
        // A fresh manager is created by pulse after the bounded backoff; its
        // powered-on callback publishes and advertises fresh service objects.
        resetServer()
    }
    func stop(_ reason: IOSRadioState = .stopped) {
        guard running else { return }; running = false
        task?.cancel(); task = nil
        for observer in observers { NotificationCenter.default.removeObserver(observer) }; observers.removeAll()
        driver.stop(); resetCentral(); resetServer(); seen.removeAll(); status(reason)
    }
}
#endif
