import SwiftUI

struct ThemeTokens: Identifiable, Sendable {
    let id: String
    let title: String
    let paid: Bool
    let light: Bool
    let background: UInt32
    let surface: UInt32
    let text: UInt32
    let secondary: UInt32
    let accent: UInt32
    let onAccent: UInt32
    let error: UInt32

    static let all = [
        ThemeTokens(id: "afterhours", title: "Afterhours", paid: false, light: false, background: 0x16161A, surface: 0x1F1F25, text: 0xF1EFE8, secondary: 0xB4B2A9, accent: 0x5DCAA5, onAccent: 0x092D23, error: 0xF09595),
        ThemeTokens(id: "daylight", title: "Daylight", paid: false, light: true, background: 0xFAF9F6, surface: 0xFFFFFF, text: 0x22232A, secondary: 0x55565D, accent: 0x176B50, onAccent: 0xFFFFFF, error: 0x9B242C),
        ThemeTokens(id: "ember", title: "Ember", paid: true, light: false, background: 0x211A18, surface: 0x302521, text: 0xF8EDDE, secondary: 0xCCB7A5, accent: 0xEAB375, onAccent: 0x35210D, error: 0xFFAAA2),
        ThemeTokens(id: "lagoon", title: "Lagoon", paid: true, light: false, background: 0x102127, surface: 0x1A3037, text: 0xE3F2EF, secondary: 0xAEC8CD, accent: 0x82D8CC, onAccent: 0x103C37, error: 0xFFAAA2),
        ThemeTokens(id: "violet", title: "Violet", paid: true, light: false, background: 0x1E192A, surface: 0x2B243B, text: 0xF2EBFC, secondary: 0xC3B5D8, accent: 0xD0B2F3, onAccent: 0x352044, error: 0xFFAAA2),
    ]
    static func selected(_ id: String, active: Bool) -> ThemeTokens {
        all.first { $0.id == id && (!$0.paid || active) } ?? all[0]
    }
    static func color(_ rgb: UInt32) -> Color {
        Color(red: Double((rgb >> 16) & 255) / 255, green: Double((rgb >> 8) & 255) / 255, blue: Double(rgb & 255) / 255)
    }
    private static func luminance(_ rgb: UInt32) -> Double {
        func component(_ shift: UInt32) -> Double {
            let c = Double((rgb >> shift) & 255) / 255
            return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4)
        }
        return component(16) * 0.2126 + component(8) * 0.7152 + component(0) * 0.0722
    }
    static func contrast(_ a: UInt32, _ b: UInt32) -> Double {
        let x = luminance(a), y = luminance(b)
        return (max(x, y) + 0.05) / (min(x, y) + 0.05)
    }
    func readableNickname(_ rgb: UInt32) -> UInt32 {
        Self.contrast(rgb, surface) >= 4.5 && Self.contrast(rgb, background) >= 4.5 ? rgb : text
    }
}
