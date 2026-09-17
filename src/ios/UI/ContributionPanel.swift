import SwiftUI
import CoreTransferable
import UniformTypeIdentifiers
import UIKit

/// MC-035 connects the production protected transport snapshot/reset to this view.
struct ContributionPanel: View {
    let stats: TransportStats
    let reset: () -> Void
    @State private var confirmingReset = false
    @State private var sharing = false
    @State private var snapshot: TransportStats?

    var body: some View {
        Form {
            Section("Private local activity") {
                Text("This app session only. Reset, app restart or protected identity reopening starts a new window. Radio stop/start preserves counts.")
                Text("Window: \(stats.elapsedMs / 60_000) minutes")
                Text("Received frames: \(stats.receivedFrames) · includes rejected input")
                Text("Reassembled packets: \(stats.receivedPackets) · includes repeated copies")
                Text("Received chat packets: \(stats.receivedChatPackets)")
                Text("Frames offered to radio: \(stats.scheduledFrames) · may be refused")
                Text("Native frame completions: \(stats.completedFrames)")
                Text("Completed outgoing objects: \(stats.completedObjects)")
                Text("Live chat relay copies: \(stats.relayedChatCopies)")
                Text("Each outgoing connection counts separately. Delivery, unique people, reach and bridge coverage are unknown. No telemetry or leaderboard.")
            }
            Section("Power") {
                if let battery = stats.batteryPercent {
                    Text("Device battery: \(battery)% · \(stats.charging ? "charging" : "unplugged")")
                    Text("Active policy: \(stats.powerMode)")
                } else {
                    Text("Device battery reading unavailable or older than one minute.")
                }
                Text("App battery use unavailable. No reliable app-specific measurement or calibrated estimate is available; device charge is not app consumption.")
            }
            Button("Preview contribution card") { snapshot = stats; sharing = true }
            Button("Reset local counters") { confirmingReset = true }
        }
        .navigationTitle("Contribution & power")
        .confirmationDialog("Reset only local contribution counts?", isPresented: $confirmingReset) {
            Button("Reset counters", role: .destructive) { reset() }
        } message: { Text("Connections, traffic limits, history and trust are unchanged.") }
        .sheet(isPresented: $sharing) { if let snapshot { ContributionPreview(stats: snapshot) } }
    }
}

private struct ContributionImage: Transferable {
    let data: Data
    static var transferRepresentation: some TransferRepresentation {
        DataRepresentation(exportedContentType: .png) { $0.data }
    }
}
private struct ContributionPreview: View {
    let stats: TransportStats
    @Environment(\.dismiss) private var dismiss
    @State private var received = false
    @State private var sent = false
    @State private var relayed = false
    private var text: String { contributionShareText(stats: stats, received: received, sent: sent, relayed: relayed) }
    private var image: UIImage {
        let attributes: [NSAttributedString.Key: Any] = [.font: UIFont.systemFont(ofSize: 40), .foregroundColor: UIColor.darkText]
        let size = (text as NSString).boundingRect(with: CGSize(width: 960, height: 3000), options: [.usesLineFragmentOrigin, .usesFontLeading], attributes: attributes, context: nil)
        let format = UIGraphicsImageRendererFormat(); format.scale = 1; format.opaque = true
        return UIGraphicsImageRenderer(size: CGSize(width: 1080, height: ceil(size.height) + 120), format: format).image { renderer in
            UIColor(red: 0.965, green: 0.98, blue: 0.969, alpha: 1).setFill()
            renderer.fill(CGRect(x: 0, y: 0, width: 1080, height: ceil(size.height) + 120))
            (text as NSString).draw(in: CGRect(x: 60, y: 60, width: 960, height: ceil(size.height)), withAttributes: attributes)
        }
    }
    var body: some View {
        NavigationStack {
            Form {
                Text("Frozen preview. Only selected local aggregates are included in the image.")
                Toggle("Received activity", isOn: $received)
                Toggle("Native handoffs", isOn: $sent)
                Toggle("Live chat relays", isOn: $relayed)
                if received || sent || relayed {
                    Text(text)
                    let rendered = image
                    if let data = rendered.pngData() {
                        ShareLink(item: ContributionImage(data: data), preview: SharePreview("Local contribution", image: Image(uiImage: rendered))) { Text("Share image") }
                    }
                }
            }
            .navigationTitle("Choose what to share")
            .toolbar { Button("Cancel") { dismiss() } }
        }
    }
}
