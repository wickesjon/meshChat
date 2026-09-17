import SwiftUI

@main
struct MeshChatApp: App {
    private let model: MeshModel?
    init() { model = try? MeshModel.native() }
    var body: some Scene {
        WindowGroup {
            if let model { MeshView(model: model) }
            else { Text("Protected storage could not open. Unlock and reopen the app. No identity or data was recreated.").padding() }
        }
    }
}
