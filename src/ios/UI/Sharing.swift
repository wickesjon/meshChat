import SwiftUI
import UIKit
import CoreImage.CIFilterBuiltins
@preconcurrency import AVFoundation

enum Sharing {
    static let internetNotice = "HTTPS app opening is not verified yet. Installation needs internet and a published store listing. In-app QR scanning works offline."
    static func channelURI(_ name: String) throws -> String {
        let channel = try channelInfo(name: name)
        guard channel.private else { throw ChannelError.Invalid }
        return "meshfest://j/" + channel.name.replacingOccurrences(of: "|", with: "-")
    }
    static func canonical(_ uri: String) throws -> String {
        if let channel = try? channelLink(uri: uri) { return try channelURI(channel.name) }
        _ = try friendProposal(uri: uri)
        let parts = uri.split(separator: "/", omittingEmptySubsequences: true)
        return "meshfest://friend/" + parts.suffix(2).joined(separator: "/")
    }
    static func https(_ uri: String) throws -> String { try canonical(uri).replacingOccurrences(of: "meshfest://", with: "https://meshfest.app/") }
    static func qr(_ uri: String) throws -> UIImage {
        let filter = CIFilter.qrCodeGenerator(); filter.message = Data(try canonical(uri).utf8); filter.correctionLevel = "M"
        guard let raw = filter.outputImage else { throw ChannelError.Invalid }
        let scale: CGFloat = 8, margin: CGFloat = 32
        guard let cg = CIContext().createCGImage(raw, from: raw.extent) else { throw ChannelError.Invalid }
        let size = CGSize(width: raw.extent.width * scale + 2 * margin, height: raw.extent.height * scale + 2 * margin)
        let format = UIGraphicsImageRendererFormat(); format.scale = 1; format.opaque = true
        return UIGraphicsImageRenderer(size: size, format: format).image { renderer in
            UIColor.white.setFill(); renderer.fill(CGRect(origin: .zero, size: size))
            renderer.cgContext.interpolationQuality = .none
            UIImage(cgImage: cg).draw(in: CGRect(x: margin, y: margin, width: size.width - 2 * margin, height: size.height - 2 * margin))
        }
    }
    @MainActor static func copy(_ uri: String) throws {
        UIPasteboard.general.setItems([[UIPasteboard.typeAutomatic: try https(uri)]], options: [.localOnly: true, .expirationDate: Date().addingTimeInterval(60)])
    }
}

struct NativeShareSheet: UIViewControllerRepresentable {
    let items: [Any]
    func makeUIViewController(context: Context) -> UIActivityViewController { UIActivityViewController(activityItems: items, applicationActivities: nil) }
    func updateUIViewController(_ controller: UIActivityViewController, context: Context) {}
}

struct PublicShareView: View {
    let uri: String, title: String, explanation: String
    @Environment(\.dismiss) private var dismiss
    @State private var items: [Any] = []
    @State private var sharing = false
    @State private var error: String?
    var body: some View {
        NavigationView {
            ScrollView { VStack(alignment: .leading, spacing: 18) {
                Text(title).font(.title2); Text(explanation)
                if let image = try? Sharing.qr(uri) { Image(uiImage: image).interpolation(.none).resizable().scaledToFit().background(Color.white).accessibilityLabel("Offline QR code") }
                Text("Scanning works fully offline in the app.")
                if let channel = try? channelLink(uri: uri) {
                    ForEach(channel.name.components(separatedBy: "|"), id: \.self) { Text($0).font(.title) }
                    Text("Say them out loud — same channel.")
                } else if let friend = try? friendProposal(uri: uri) { Text(friend.fingerprint).font(.system(.body, design: .monospaced)) }
                Button("Share as QR") { do { items = [try Sharing.qr(uri)]; sharing = true } catch { self.error = "QR export unavailable. Show the code on screen instead." } }
                Button("Share link") { do { items = [try Sharing.https(uri)]; sharing = true } catch { self.error = "Link unavailable." } }
                Text(Sharing.internetNotice)
                Button("Copy link") { do { try Sharing.copy(uri) } catch { self.error = "Link unavailable." } }
                Text("Copy uses this device's clipboard for one minute. Other apps may be able to read it. Prefer QR or the share sheet.")
                if let error { Text(error) }
            }.padding() }.navigationTitle("Share").toolbar { Button("Done") { dismiss() } }
            .sheet(isPresented: $sharing) { NativeShareSheet(items: items) }
        }
    }
}

struct CodeScanner: UIViewControllerRepresentable {
    let result: (String) -> Void
    func makeUIViewController(context: Context) -> ScannerController { ScannerController(result: result) }
    func updateUIViewController(_ controller: ScannerController, context: Context) {}
    static func dismantleUIViewController(_ controller: ScannerController, coordinator: ()) { controller.stop() }
}

@MainActor final class ScannerController: UIViewController, @preconcurrency AVCaptureMetadataOutputObjectsDelegate {
    private let session = AVCaptureSession()
    private let result: (String) -> Void
    private var preview: AVCaptureVideoPreviewLayer?
    private var finished = false
    init(result: @escaping (String) -> Void) { self.result = result; super.init(nibName: nil, bundle: nil) }
    required init?(coder: NSCoder) { nil }
    override func viewDidLoad() {
        super.viewDidLoad(); view.backgroundColor = .black
        let label = UILabel(); label.text = "Scan the code on the other person's screen. Camera permission is required."; label.numberOfLines = 0; label.textColor = .white
        label.frame = CGRect(x: 24, y: 32, width: 280, height: 100); view.addSubview(label)
        Task { @MainActor [weak self] in
            guard await AVCaptureDevice.requestAccess(for: .video), let self, !self.finished,
                  let device = AVCaptureDevice.default(for: .video), let input = try? AVCaptureDeviceInput(device: device) else { return }
            let output = AVCaptureMetadataOutput()
            guard self.session.canAddInput(input), self.session.canAddOutput(output) else { return }
            self.session.addInput(input); self.session.addOutput(output)
            output.setMetadataObjectsDelegate(self, queue: .main); output.metadataObjectTypes = [.qr]
            let layer = AVCaptureVideoPreviewLayer(session: self.session); layer.videoGravity = .resizeAspectFill; layer.frame = self.view.bounds
            self.view.layer.insertSublayer(layer, at: 0); self.preview = layer; self.session.startRunning()
        }
    }
    override func viewDidLayoutSubviews() { super.viewDidLayoutSubviews(); preview?.frame = view.bounds }
    func stop() { finished = true; session.stopRunning() }
    func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput objects: [AVMetadataObject], from connection: AVCaptureConnection) {
        guard !finished, let value = (objects.first as? AVMetadataMachineReadableCodeObject)?.stringValue, value.utf8.count <= 2048 else { return }
        stop(); result(value)
    }
}
