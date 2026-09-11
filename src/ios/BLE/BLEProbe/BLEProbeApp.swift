import SwiftUI
import UIKit
@preconcurrency import CoreBluetooth

private enum ProbeIds {
    static let service = CBUUID(string: "5f45c0de-71a5-4f81-9f52-52f1ee004001")
    static let write = CBUUID(string: "5f45c0de-71a5-4f81-9f52-52f1ee004002")
    static let notify = CBUUID(string: "5f45c0de-71a5-4f81-9f52-52f1ee004003")
}

// All managers deliver callbacks on the main queue. The conformance annotations
// bridge CoreBluetooth's delegate protocols to this serialized probe state.
@MainActor
final class ProbeModel: NSObject, ObservableObject, @preconcurrency CBCentralManagerDelegate,
    @preconcurrency CBPeripheralDelegate, @preconcurrency CBPeripheralManagerDelegate {
    @Published var lines: [String] = []
    @Published var sizeText = "20"
    @Published var periodic = false
    private var dropped = 0
    private var sequence: UInt32 = 0
    private var central: CBCentralManager!
    private var peripheralManager: CBPeripheralManager!
    private var client: CBPeripheral?
    private var writeCharacteristic: CBCharacteristic?
    private var localNotify: CBMutableCharacteristic?
    private var subscriber: CBCentral?
    private var ready = false
    private var disconnecting = false
    private var scanning = false
    private var scanGeneration = 0
    private var clientGeneration = 0
    private var advertisingWanted = false
    private var servicePending = false
    private var servicePublished = false
    private var writes: [Data] = []
    private var notifications: [Data] = []
    private var trafficTask: Task<Void, Never>?

    override init() {
        super.init()
        record("start os=\(ProcessInfo.processInfo.operatingSystemVersionString)")
        central = CBCentralManager(delegate: self, queue: .main,
            options: [CBCentralManagerOptionRestoreIdentifierKey: "mc004-central"])
        peripheralManager = CBPeripheralManager(delegate: self, queue: .main,
            options: [CBPeripheralManagerOptionRestoreIdentifierKey: "mc004-peripheral"])
    }
    func record(_ event: String) {
        if lines.count == 4096 { lines.removeFirst(); dropped += 1 }
        lines.append("\(UInt64(ProcessInfo.processInfo.systemUptime * 1000)) \(event)")
    }
    var report: String { "MC004 synthetic BLE probe; dropped_trace_rows=\(dropped)\n" + lines.joined(separator: "\n") }
    private func payload() -> Data? {
        guard let size = Int(sizeText), (1...1024).contains(size) else { record("invalid_size allowed=1..1024"); return nil }
        sequence &+= 1
        return Data((0..<size).map { index in
            index < 4 ? UInt8(truncatingIfNeeded: sequence >> ((3 - index) * 8)) : UInt8(truncatingIfNeeded: index)
        })
    }
    private func received(_ path: String, _ value: Data) {
        guard value.count <= 1024 else { record("rx_rejected path=\(path) length=\(value.count)"); return }
        let seq = value.count >= 4 ? value.prefix(4).reduce(Int64(0)) { ($0 << 8) | Int64($1) } : -1
        record("rx path=\(path) length=\(value.count) fixture_seq=\(seq)")
    }
    func scan() {
        guard central.state == .poweredOn, client == nil, !scanning else { record("scan_refused state_or_client"); return }
        scanning = true; scanGeneration += 1
        let generation = scanGeneration
        central.scanForPeripherals(withServices: [ProbeIds.service], options: nil)
        record("scan_start timeout_ms=60000")
        Task { [weak self] in
            try? await Task.sleep(nanoseconds: 60_000_000_000)
            guard let self, self.scanning, self.scanGeneration == generation else { return }
            self.central.stopScan(); self.scanning = false; self.record("scan_timeout")
        }
    }
    func centralManagerDidUpdateState(_ central: CBCentralManager) {
        record("central_state=\(central.state.rawValue)")
        if central.state == .poweredOn {
            if let client, client.state == .connected, !ready, !disconnecting {
                client.discoverServices([ProbeIds.service])
            }
        } else if central.state != .unknown {
            scanning = false; scanGeneration += 1
            clearClient()
        }
    }
    func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral,
                        advertisementData: [String: Any], rssi RSSI: NSNumber) {
        guard scanning, client == nil else { return }
        central.stopScan(); scanning = false; scanGeneration += 1
        client = peripheral; peripheral.delegate = self; clientGeneration += 1
        let generation = clientGeneration
        record("discovered rssi=\(RSSI.intValue); connect_start generation=\(generation)")
        central.connect(peripheral)
        Task { [weak self, weak peripheral] in
            try? await Task.sleep(nanoseconds: 30_000_000_000)
            guard let self, let peripheral, self.client === peripheral,
                  self.clientGeneration == generation, !self.ready else { return }
            self.record("connection_setup_timeout"); self.disconnectClient()
        }
    }
    func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        guard client === peripheral, !disconnecting else { return }
        record("client_connected write_limit=\(peripheral.maximumWriteValueLength(for: .withoutResponse))")
        peripheral.discoverServices([ProbeIds.service])
    }
    func centralManager(_ central: CBCentralManager, didFailToConnect peripheral: CBPeripheral, error: Error?) {
        guard client === peripheral else { return }
        record("connect_failed code=\((error as NSError?)?.code ?? 0)"); clearClient()
    }
    func centralManager(_ central: CBCentralManager, didDisconnectPeripheral peripheral: CBPeripheral, error: Error?) {
        guard client === peripheral else { return }
        record("client_disconnected code=\((error as NSError?)?.code ?? 0)"); clearClient()
    }
    func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        guard client === peripheral, !disconnecting else { return }
        guard error == nil, let service = peripheral.services?.first(where: { $0.uuid == ProbeIds.service }) else {
            record("service_discovery_failed"); disconnectClient(); return
        }
        peripheral.discoverCharacteristics([ProbeIds.write, ProbeIds.notify], for: service)
    }
    func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
        guard client === peripheral, !disconnecting else { return }
        writeCharacteristic = service.characteristics?.first(where: { $0.uuid == ProbeIds.write })
        guard error == nil, let writeCharacteristic,
              writeCharacteristic.properties.contains(.writeWithoutResponse),
              let rx = service.characteristics?.first(where: { $0.uuid == ProbeIds.notify }),
              rx.properties.contains(.notify) else { record("service_contract_missing"); disconnectClient(); return }
        peripheral.setNotifyValue(true, for: rx)
    }
    func peripheral(_ peripheral: CBPeripheral, didUpdateNotificationStateFor characteristic: CBCharacteristic, error: Error?) {
        guard client === peripheral, characteristic.uuid == ProbeIds.notify, !disconnecting else { return }
        ready = error == nil && characteristic.isNotifying
        record("client_ready=\(ready)")
        if !ready { writes.removeAll() }
    }
    func peripheral(_ peripheral: CBPeripheral, didUpdateValueFor characteristic: CBCharacteristic, error: Error?) {
        guard client === peripheral, !disconnecting, characteristic.uuid == ProbeIds.notify else { return }
        if error == nil, let value = characteristic.value { received("notify", value) }
        else { record("notify_receive_error") }
    }
    func peripheralIsReady(toSendWriteWithoutResponse peripheral: CBPeripheral) {
        guard client === peripheral, !disconnecting else { return }
        record("write_ready_callback"); drainWrites()
    }
    private func clearClient() {
        record("client_cleanup dropped=\(writes.count)")
        client?.delegate = nil; client = nil; writeCharacteristic = nil
        writes.removeAll(); ready = false; disconnecting = false; clientGeneration += 1
    }
    private func disconnectClient() {
        ready = false; disconnecting = true; writes.removeAll(); clientGeneration += 1
        if let client { central.cancelPeripheralConnection(client) }
        else { disconnecting = false }
        // Keep the object until didDisconnect; don't reuse a pending connection.
    }
    func advertise() {
        advertisingWanted = true
        publishIfReady()
    }
    private func publishIfReady() {
        guard peripheralManager.state == .poweredOn, advertisingWanted, !servicePending else { return }
        if servicePublished { startAdvertising(); return }
        let tx = CBMutableCharacteristic(type: ProbeIds.write, properties: [.writeWithoutResponse], value: nil, permissions: [.writeable])
        let rx = CBMutableCharacteristic(type: ProbeIds.notify, properties: [.notify], value: nil, permissions: [])
        localNotify = rx
        let service = CBMutableService(type: ProbeIds.service, primary: true)
        service.characteristics = [tx, rx]
        servicePending = true
        peripheralManager.add(service)
    }
    private func startAdvertising() {
        if !peripheralManager.isAdvertising {
            peripheralManager.startAdvertising([CBAdvertisementDataServiceUUIDsKey: [ProbeIds.service]])
        }
    }
    func peripheralManagerDidUpdateState(_ peripheral: CBPeripheralManager) {
        record("peripheral_state=\(peripheral.state.rawValue)")
        if peripheral.state == .poweredOn { publishIfReady() }
        else if peripheral.state != .unknown {
            subscriber = nil; notifications.removeAll(); localNotify = nil
            servicePublished = false; servicePending = false
        }
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, didAdd service: CBService, error: Error?) {
        guard let localNotify, service.characteristics?.contains(where: { $0 === localNotify }) == true else { return }
        servicePending = false; servicePublished = error == nil
        record("service_added success=\(error == nil)")
        if servicePublished && advertisingWanted { startAdvertising() }
    }
    func peripheralManagerDidStartAdvertising(_ peripheral: CBPeripheralManager, error: Error?) {
        record("advertising_result code=\((error as NSError?)?.code ?? 0)")
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, central: CBCentral, didSubscribeTo characteristic: CBCharacteristic) {
        guard characteristic.uuid == ProbeIds.notify else { return }
        if let subscriber, subscriber.identifier != central.identifier { record("extra_subscriber_excluded"); return }
        subscriber = central
        record("subscribed notify_limit=\(central.maximumUpdateValueLength)")
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, central: CBCentral, didUnsubscribeFrom characteristic: CBCharacteristic) {
        guard characteristic.uuid == ProbeIds.notify, subscriber?.identifier == central.identifier else { return }
        record("unsubscribe dropped=\(notifications.count)")
        subscriber = nil; notifications.removeAll()
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, didReceiveWrite requests: [CBATTRequest]) {
        guard let first = requests.first else { return }
        let valid = requests.count <= 16 && requests.allSatisfy {
            $0.characteristic.uuid == ProbeIds.write && $0.offset == 0 &&
            $0.central.identifier == subscriber?.identifier && ($0.value?.count ?? 0) <= 1024
        }
        guard valid else { peripheral.respond(to: first, withResult: .requestNotSupported); record("write_request_rejected"); return }
        for request in requests { received("write", request.value ?? Data()) }
        // CoreBluetooth requires one response to the first request for the batch.
        peripheral.respond(to: first, withResult: .success)
    }
    func peripheralManagerIsReady(toUpdateSubscribers peripheral: CBPeripheralManager) {
        record("notify_ready_callback"); drainNotifications()
    }
    func burst(_ path: String, count: Int = 16) {
        guard (1...16).contains(count) else { return }
        for _ in 0..<count {
            if path == "write" {
                guard ready else { record("write_refused not_ready"); return }
                guard writes.count < 8 else { record("queue_full path=write"); continue }
                guard let value = payload() else { return }
                writes.append(value); drainWrites()
            } else {
                guard subscriber != nil else { record("notify_refused not_ready"); return }
                guard notifications.count < 8 else { record("queue_full path=notify"); continue }
                guard let value = payload() else { return }
                notifications.append(value); drainNotifications()
            }
        }
    }
    private func drainWrites() {
        guard let client, let writeCharacteristic, ready, !disconnecting else { return }
        while !writes.isEmpty {
            guard client.canSendWriteWithoutResponse else { record("write_waiting_readiness"); return }
            let value = writes.removeFirst()
            let capacity = client.maximumWriteValueLength(for: .withoutResponse)
            guard value.count <= capacity else { record("probe_refused_write length=\(value.count) api_limit=\(capacity)"); continue }
            client.writeValue(value, for: writeCharacteristic, type: .withoutResponse)
            record("write_enqueued length=\(value.count) ready_after=\(client.canSendWriteWithoutResponse)")
        }
    }
    private func drainNotifications() {
        guard let subscriber, let localNotify, peripheralManager.state == .poweredOn else { return }
        while let value = notifications.first {
            guard value.count <= subscriber.maximumUpdateValueLength else {
                notifications.removeFirst(); record("probe_refused_notify length=\(value.count) api_limit=\(subscriber.maximumUpdateValueLength)"); continue
            }
            guard peripheralManager.updateValue(value, for: localNotify, onSubscribedCentrals: [subscriber]) else {
                record("notify_waiting_readiness"); return
            }
            notifications.removeFirst(); record("notify_enqueued length=\(value.count)")
        }
    }
    func togglePeriodic() {
        periodic.toggle(); trafficTask?.cancel(); trafficTask = nil
        record("periodic=\(periodic) interval_ms=2000; suspension_may_pause_timer")
        if periodic {
            trafficTask = Task { [weak self] in
                while !Task.isCancelled {
                    do { try await Task.sleep(nanoseconds: 2_000_000_000) } catch { return }
                    guard let self, self.periodic else { return }
                    if self.ready { self.burst("write", count: 1) }
                    if self.subscriber != nil { self.burst("notify", count: 1) }
                }
            }
        }
    }
    func disconnect() {
        disconnectClient()
        notifications.removeAll()
        record("disconnect_client; iOS_cannot_force_disconnect_subscribed_central")
    }
    func stop() {
        periodic = false; trafficTask?.cancel(); trafficTask = nil
        central.stopScan(); scanning = false; scanGeneration += 1
        disconnectClient()
        advertisingWanted = false; peripheralManager.stopAdvertising(); peripheralManager.removeAllServices()
        subscriber = nil; localNotify = nil; notifications.removeAll(); servicePublished = false; servicePending = false
        record("stopped; remote_central_controls_inbound_disconnect")
    }
    func centralManager(_ central: CBCentralManager, willRestoreState dict: [String: Any]) {
        let restored = dict[CBCentralManagerRestoredStatePeripheralsKey] as? [CBPeripheral] ?? []
        record("central_restored count=\(restored.count); prior_trace_unavailable")
        if let first = restored.first {
            client = first; first.delegate = self; clientGeneration += 1
            if central.state == .poweredOn && first.state == .connected { first.discoverServices([ProbeIds.service]) }
        }
        for extra in restored.dropFirst() { central.cancelPeripheralConnection(extra) }
        central.stopScan(); scanning = false
    }
    func peripheralManager(_ peripheral: CBPeripheralManager, willRestoreState dict: [String: Any]) {
        let services = dict[CBPeripheralManagerRestoredStateServicesKey] as? [CBMutableService] ?? []
        localNotify = services.first(where: { $0.uuid == ProbeIds.service })?.characteristics?.first(where: { $0.uuid == ProbeIds.notify }) as? CBMutableCharacteristic
        servicePublished = localNotify != nil
        subscriber = localNotify?.subscribedCentrals?.first
        advertisingWanted = dict[CBPeripheralManagerRestoredStateAdvertisementDataKey] != nil
        record("peripheral_restored service=\(servicePublished) subscriber=\(subscriber != nil); prior_trace_unavailable")
    }
}

@main
struct BLEProbeApp: App {
    @StateObject private var model = ProbeModel()
    var body: some Scene {
        WindowGroup {
            ScrollView {
                VStack(alignment: .leading, spacing: 12) {
                    Text("MC-004 Synthetic BLE Bench").font(.headline)
                    Text("Test-only UUIDs. Copy the report before restarting. API acceptance is not receiver delivery.")
                    Button("Scan and connect (central)") { model.scan() }
                    Button("Advertise (peripheral)") { model.advertise() }
                    TextField("Value size: 1–1024 bytes", text: $model.sizeText).keyboardType(.numberPad)
                    Button("One write") { model.burst("write", count: 1) }
                    Button("One notification") { model.burst("notify", count: 1) }
                    Button("Burst 16 writes (queue limit 8)") { model.burst("write") }
                    Button("Burst 16 notifications (queue limit 8)") { model.burst("notify") }
                    Button(model.periodic ? "Stop periodic traffic" : "Traffic every 2 seconds") { model.togglePeriodic() }
                    Button("Disconnect central link") { model.disconnect() }
                    Button("Copy sanitized report") { UIPasteboard.general.string = model.report }
                    Button("Stop") { model.stop() }
                    Text(model.lines.suffix(45).joined(separator: "\n")).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                }.padding()
            }.onReceive(NotificationCenter.default.publisher(for: UIApplication.didEnterBackgroundNotification)) { _ in
                model.record("scene=background; suspension_unconfirmed")
            }.onReceive(NotificationCenter.default.publisher(for: UIApplication.didBecomeActiveNotification)) { _ in
                model.record("scene=active")
            }
        }
    }
}
