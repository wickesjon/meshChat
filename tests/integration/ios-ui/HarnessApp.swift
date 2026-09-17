import SwiftUI

@main struct HarnessApp: App {
    private let device: TestDevice
    init() {
        device = try! TestDevice()
    }
    var body: some Scene { WindowGroup {
        if ProcessInfo.processInfo.arguments.contains("--integration") { IntegrationCheckView() }
        else if ProcessInfo.processInfo.arguments.contains("--row-identities") { RowCollisionCheck() }
        else { MeshView(model: device.model) }
    } }
}

private struct IntegrationCheckView: View {
    @State private var result = "MC035 integration running"
    var body: some View {
        Text(result).accessibilityIdentifier("integration-result").task { @MainActor in
            await Task.yield()
            do { try await featureChecks(); result = "MC035 integration PASS" }
            catch { result = "MC035 integration FAILED: \(error)" }
        }
    }
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
