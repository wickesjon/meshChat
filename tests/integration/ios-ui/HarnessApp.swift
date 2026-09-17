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
        else { MeshView(model: device.model) }
    } }
}
