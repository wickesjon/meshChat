import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

/// MC-035 binds these choices to protected profile storage and applies the tokens
/// to its full feature UI. No store identifiers or test entitlement switches live here.
struct SupporterSettings: View {
    @ObservedObject var store: StoreEntitlements
    @Binding var theme: String
    @Binding var nicknameRGB: UInt32?

    var body: some View {
        Section("Appearance") {
            Picker("Theme", selection: Binding(
                get: { ThemeTokens.selected(theme, active: store.active).id },
                set: { theme = $0 })) {
                ForEach(ThemeTokens.all.filter { !$0.paid || store.active }) { t in
                    Text(t.title).tag(t.id)
                }
            }
            if store.active {
                ColorPicker("Nickname color", selection: Binding(
                    get: { ThemeTokens.color(nicknameRGB ?? 0x5DCAA5) },
                    set: { value in
                        #if canImport(UIKit)
                        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
                        if UIColor(value).getRed(&r, green: &g, blue: &b, alpha: &a) {
                            func byte(_ value: CGFloat) -> UInt32 { UInt32(max(0, min(1, value)) * 255) }
                            nicknameRGB = (byte(r) << 16) | (byte(g) << 8) | byte(b)
                        }
                        #endif
                    }), supportsOpacity: false)
                Button("Use automatic nickname color") { nicknameRGB = nil }
            }
            Text("Receivers may adjust colors for readability. Suffix and trust labels stay separate.")
        }
        Section("Supporter · optional") {
            Text("Extra themes, nickname color and up to 30 private channel slots. Messaging, friends and relaying stay free.")
            Text(store.status)
            Button(store.price.map { "Supporter · \($0) · one-time" } ?? "Purchases unavailable") {
                Task { await store.purchase() }
            }.disabled(store.price == nil || store.active)
            Button("Restore Supporter") { Task { await store.restore() } }
        }
    }
}
