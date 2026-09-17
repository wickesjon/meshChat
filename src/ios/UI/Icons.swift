import SwiftUI

// Append-only v1 IDs and bundled vector geometry match Android Icons.kt.
let meshAnimals = ["mask", "cat", "dog", "lizard", "raccoon", "turtle", "fish", "bird", "whale"]
let meshReactions = ["fire", "heart", "laugh", "thumb", "party", "surprise", "melt", "speaker"]
let meshPalette: [UInt32] = [0x5DCAA5,0xF49B83,0xBDA6EF,0xE8A0CE,0x8DBBEF,0xA3CB8C,0xEFBB67,0x91CCCA,0x9FE1CB,0xF7BEAA,0xD3C5F5,0xF1C1DF,0xB9D4F6,0xC5DEB3,0xF5D79E,0xBCE2E0]

struct MeshIcon: View {
    let name: String
    var body: some View { MeshSymbol(name: name).stroke(style: StrokeStyle(lineWidth: 1.7, lineCap: .round, lineJoin: .round)).frame(width: 28, height: 28).accessibilityLabel(name) }
}
struct MeshAvatar: View {
    let value: UInt8
    @Environment(\.colorScheme) private var scheme
    private var tint: Color {
        let rgb = value == 0 ? 0xB4B2A9 : meshPalette[Int(value >> 4)]
        let scale = scheme == .light ? 0.45 : 1.0
        return Color(red: Double((rgb >> 16) & 255) / 255 * scale, green: Double((rgb >> 8) & 255) / 255 * scale, blue: Double(rgb & 255) / 255 * scale)
    }
    var body: some View { MeshIcon(name: Int(value & 15) < meshAnimals.count ? meshAnimals[Int(value & 15)] : "paw").foregroundColor(tint).padding(8).background(tint.opacity(0.12)).clipShape(Circle()) }
}
private struct MeshSymbol: Shape {
    let name: String
    func path(in rect: CGRect) -> Path {
        var p = Path()
        func m(_ x: CGFloat, _ y: CGFloat) { p.move(to: CGPoint(x: x, y: y)) }
        func l(_ x: CGFloat, _ y: CGFloat) { p.addLine(to: CGPoint(x: x, y: y)) }
        func c(_ a: CGFloat,_ b: CGFloat,_ d: CGFloat,_ e: CGFloat,_ x: CGFloat,_ y: CGFloat) { p.addCurve(to: CGPoint(x: x,y: y),control1: CGPoint(x: a,y: b),control2: CGPoint(x: d,y: e)) }
        func q(_ a: CGFloat,_ b: CGFloat,_ x: CGFloat,_ y: CGFloat) { p.addQuadCurve(to: CGPoint(x: x,y: y),control: CGPoint(x: a,y: b)) }
        func o(_ x: CGFloat,_ y: CGFloat,_ r: CGFloat) { p.addEllipse(in: CGRect(x: x-r,y: y-r,width: r*2,height: r*2)) }
        switch name {
        case "wave": m(2,13);c(6,3,9,22,13,12);c(17,2,19,17,22,9)
        case "lightning-bolt": m(14,2);l(4,14);l(11,14);l(10,22);l(20,9);l(13,9);p.closeSubpath()
        case "fire": m(12,2);c(17,8,23,13,18,19);c(9,27,1,16,8,9);c(7,14,15,10,12,2);p.closeSubpath()
        case "heart": m(12,21);c(-7,8,7,-1,12,7);c(18,-1,31,8,12,21);p.closeSubpath()
        case "moon": m(17,3);c(1,-1,1,26,21,18);c(9,20,7,7,17,3);p.closeSubpath()
        case "star": m(12,2);l(16,9);l(22,12);l(16,15);l(12,22);l(8,15);l(2,12);l(8,9);p.closeSubpath()
        case "crystal": m(12,2);l(20,8);l(17,20);l(7,22);l(4,9);p.closeSubpath();m(12,2);l(10,10);l(7,22);m(10,10);l(20,8)
        case "sun": o(12,12,5);for i in 0...7 { let a = CGFloat(i) * .pi / 4; m(12+8*cos(a),12+8*sin(a));l(12+11*cos(a),12+11*sin(a)) }
        case "spiral": m(12,12);for i in 1...120 { let a=CGFloat(i)*0.12,r=CGFloat(i)*0.075;l(12+r*cos(a),12+r*sin(a)) }
        case "disco-ball": o(12,13,9);m(12,0);l(12,4);for x in [CGFloat(8),12,16] {m(x,6);l(x,20)};for y in [CGFloat(9),13,17] {m(5,y);l(19,y)}
        case "open-lock", "closed-lock": m(7,11);l(7,7);c(7,0,18,0,18,7);if name == "closed-lock" {l(18,11)};m(4,11);l(21,11);l(21,22);l(4,22);p.closeSubpath()
        case "mushroom": m(2,14);c(3,-1,22,-1,23,14);p.closeSubpath();m(10,14);l(9,22);l(16,22);l(15,14)
        case "cactus": m(10,22);l(10,4);c(10,0,15,0,15,4);l(15,22);m(10,13);l(5,13);l(5,7);m(15,16);l(20,16);l(20,10)
        case "speaker": m(3,9);l(8,9);l(14,4);l(14,21);l(8,16);l(3,16);p.closeSubpath();m(18,6);c(25,9,25,16,18,19)
        case "mask": m(2,7);q(12,3,22,7);q(21,21,12,15);q(3,21,2,7);p.closeSubpath();o(7,11,1);o(17,11,1)
        case "cat", "dog", "raccoon":
            o(12,13,8)
            if name == "cat" || name == "raccoon" {m(5,9);l(3,2);l(10,6);m(14,6);l(21,2);l(19,9)}
            if name == "dog" {m(5,6);q(-3,14,5,17);m(19,6);q(28,14,19,17)}
            if name == "raccoon" {m(5,10);l(19,10);l(16,15);l(8,15);p.closeSubpath()}
            o(9,12,1);o(15,12,1);m(9,17);q(12,19,15,17)
        case "lizard": m(12,3);c(3,6,18,14,10,20);q(6,24,4,19);m(10,8);l(4,8);l(2,5);m(12,11);l(20,8);l(22,5);m(11,15);l(18,17);l(19,21);o(12,4,2)
        case "turtle": p.addEllipse(in: CGRect(x:5,y:7,width:14,height:13));o(12,4,3);m(6,9);l(2,6);m(18,9);l(22,6);m(7,18);l(3,22);m(17,18);l(21,22);m(12,7);l(8,13);l(12,19);l(16,13);p.closeSubpath()
        case "fish": m(2,12);q(9,1,17,9);l(23,5);l(23,19);l(17,15);q(9,23,2,12);p.closeSubpath();o(7,11,1)
        case "bird": m(4,20);l(4,10);c(4,1,17,0,18,9);l(23,12);l(18,14);q(16,23,4,20);m(6,14);q(11,21,15,13);o(14,8,1)
        case "whale": m(2,12);c(2,1,17,5,17,14);l(22,9);l(22,17);c(10,25,2,21,2,12);p.closeSubpath();m(8,5);l(8,1);m(8,3);l(12,1);o(6,12,1)
        case "paw": p.addEllipse(in: CGRect(x:6,y:12,width:12,height:9));for i in 0...3 {o(CGFloat(4+i*5),i == 0 || i == 3 ? 9 : 5,2)}
        case "thumb": m(5,10);l(10,10);l(13,2);l(16,3);l(15,10);l(22,10);l(20,21);l(5,21);p.closeSubpath()
        case "party": m(2,22);l(7,7);l(17,17);p.closeSubpath();m(10,4);l(13,1);m(17,9);l(22,5);m(20,14);l(24,13)
        case "comet": o(7,17,5);m(3,12);l(21,2);l(12,21);m(11,13);l(21,3)
        case "surprise": o(12,12,9);o(8,9,1);o(16,9,1);o(12,16,3)
        case "laugh": o(12,12,9);m(6,10);l(8,8);l(10,10);m(14,10);l(16,8);l(18,10);m(6,14);l(18,14);q(12,25,6,14)
        case "melt": m(3,15);c(-1,0,24,0,21,15);c(28,19,19,24,13,20);c(5,26,-3,20,3,15);o(8,9,1);o(16,11,1);m(8,15);q(13,20,17,14)
        default: o(12,12,8);o(9,10,1);o(15,10,1);m(8,15);q(12,20,16,15)
        }
        return p.applying(CGAffineTransform(scaleX: rect.width/24, y: rect.height/24).translatedBy(x: rect.minX, y: rect.minY))
    }
}
