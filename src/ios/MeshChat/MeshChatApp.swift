import SwiftUI
import Security

@main
struct MeshChatApp: App {
    private let core: Core?

    init() {
        var nonce: UInt64 = 0
        var status = errSecSuccess
        repeat {
            status = SecRandomCopyBytes(kSecRandomDefault, MemoryLayout<UInt64>.size, &nonce)
        } while status == errSecSuccess && nonce == 0
        if status == errSecSuccess {
            core = try? Core(limits: Limits(maxLinks: 1, maxValueBytes: 64), instanceNonce: nonce,
                             monotonicMs: UInt64(ProcessInfo.processInfo.systemUptime * 1000))
            _ = try? core?.handleEvent(event: .powerChanged(state: .foreground))
        } else {
            core = nil
        }
    }

    var body: some Scene {
        WindowGroup {
            Text(core == nil ? "Unable to start. Please reopen the app." : "MeshChat")
        }
    }
}
