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
