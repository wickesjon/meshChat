import SwiftUI

@main struct HarnessApp: App {
    private let device: TestDevice
    private let result: String?
    init() {
        device = try! TestDevice()
        if ProcessInfo.processInfo.arguments.contains("--integration") {
            do { try featureChecks(); result = "MC035 integration PASS" }
            catch { result = "MC035 integration FAILED: \(error)" }
        } else { result = nil }
    }
    var body: some Scene { WindowGroup {
        if let result { Text(result).accessibilityIdentifier("integration-result") }
        else if ProcessInfo.processInfo.arguments.contains("--row-identities") { RowCollisionCheck() }
        else { MeshView(model: device.model) }
    } }
}

// Renderer-only synthetic collision: both directions legitimately share an ID.
private struct RowCollisionCheck: View {
    @State private var rows = [
        DirectMessage(id: Data(repeating: 1, count: 8), text: "Own collision", own: true, claimedTimestamp: 0, reactions: Data(repeating: 0, count: 9), ownReaction: nil),
        DirectMessage(id: Data(repeating: 1, count: 8), text: "Peer collision", own: false, claimedTimestamp: 0, reactions: Data(repeating: 0, count: 9), ownReaction: nil)
    ]
    var body: some View { VStack {
        DirectRows(rows: rows) { row in Text(verbatim: row.text) }
        Button("Remove own row") { rows.removeAll { $0.own } }
    } }
}
