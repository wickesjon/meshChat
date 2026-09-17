import Foundation
import CoreImage

enum CheckFailure: Error { case failed(String) }
@MainActor private func check(_ condition: Bool, _ label: String) throws { if !condition { throw CheckFailure.failed(label) } }
@MainActor private func refuses<T>(_ label: String, _ action: () throws -> T) throws {
    do { _ = try action() } catch { return }; throw CheckFailure.failed(label)
}

@MainActor func featureChecks() throws {
    let clock = TestClock(), a = try TestDevice(clock), b = try TestDevice(clock)
    a.model.load(); b.model.load()
    try check(!a.model.state.onboarded && !a.model.state.locked, "explicit onboarding")
    let words = channelWords(), name = [words.descriptors[0], words.genres[0], words.locations[0]].joined(separator: "|")
    let link = try Sharing.channelURI(name)
    a.model.shareInput(link)
    a.model.create(nickname: "Alice", avatar: 0xf8); b.model.create(nickname: "Bob", avatar: 0x21)
    try check(a.model.state.onboarded && a.model.state.avatar == 0xf8, "packed free avatar")
    try check(a.model.state.channelProposal?.name == name && !a.model.state.channels.contains { $0.name == name }, "deferred link is confirmation only")
    a.model.join(name); b.model.join(name)
    a.model.mute(); try check(a.model.state.muted, "mute")
    a.model.mute(); a.model.select("#general")
    a.model.send("offline"); try check(a.model.state.posted == 0 && !a.model.state.locked, "offline send fails visibly")
    let aCode = a.model.state.myCode!, bCode = b.model.state.myCode!
    a.model.shareInput(bCode.uri); try check(a.model.state.friends.isEmpty, "link cannot pin")
    a.model.confirmFriend("My Bob"); b.model.shareInput(aCode.uri, scanned: true); b.model.confirmFriend("My Alice")
    try check(a.model.state.friends.count == 1 && b.model.state.friends.count == 1, "explicit reciprocal pins")
    a.model.connect(); b.model.connect(); a.radio!.ready(.central); b.radio!.ready(.peripheral)
    TestRadio.pump(a.radio!, b.radio!, seconds: 7)
    try check(a.model.state.peers == 1 && b.model.state.peers == 1, "real driver handshake")
    a.model.select("#general"); b.model.select("#general"); a.model.send("<b>plain literal</b>")
    TestRadio.pump(a.radio!, b.radio!, seconds: 4); b.model.refresh()
    try check(b.model.state.rows.contains { $0.text == "<b>plain literal</b>" && $0.verifiedPetname == nil }, "unverified plaintext channel")
    a.model.select(name); b.model.select(name); a.model.send("signed private words")
    TestRadio.pump(a.radio!, b.radio!, seconds: 4); b.model.refresh()
    try check(b.model.state.rows.contains { $0.text == "signed private words" && $0.verifiedPetname == "My Alice" }, "signed friend attribution")
    a.model.openDirect(a.model.state.friends[0]); b.model.openDirect(b.model.state.friends[0]); a.model.send("encrypted synthetic DM")
    TestRadio.pump(a.radio!, b.radio!, seconds: 4); b.model.refresh()
    try check(b.model.state.directRows.last?.text == "encrypted synthetic DM", "authenticated encrypted DM")
    let dm = b.model.state.directRows.last!
    b.model.react(dm.id, code: 1, remove: false)
    TestRadio.pump(a.radio!, b.radio!, seconds: 3); a.model.refresh()
    try check(a.model.state.directRows.last?.reactions[1] == 1, "encrypted reaction")
    // A queued protected send must not cross a lock even if native readiness returns.
    a.radio!.writable = false; a.model.send("must never reach port")
    let sent = a.radio!.submitted; a.protection.unlocked = false
    a.model.protectedDataUnavailable(); a.radio!.writable = true; a.radio!.driver.tick()
    try check(a.model.state.locked && a.radio!.submitted == sent, "protected egress invalidation")
    a.protection.unlocked = true; a.model.load()
    try check(a.model.state.onboarded && !a.model.state.locked, "unlock reopens existing identity")
    b.model.stop()
    a.model.changeFriend(a.model.state.friends[0], replace: true)
    try check(a.model.state.friends[0].replacing && a.model.state.archives.count == 1, "replacement keeps separate archive")
    a.model.shareInput(bCode.uri); try check(a.model.state.friendProposal == nil, "replacement requires scan")
    let c = try TestDevice(clock); c.model.load(); c.model.create(nickname: "New Bob", avatar: 2)
    a.model.shareInput(c.model.state.myCode!.uri, scanned: true); a.model.confirmFriend("New Bob")
    try check(a.model.state.friends.count == 1 && a.model.state.friends[0].keys != bCode.keys, "confirmed new identity")
    a.model.openDirect(a.model.state.archives[0], archived: true)
    try check(a.model.state.directArchived && a.model.state.directRows.contains { $0.text == "encrypted synthetic DM" }, "old history read only")
    a.model.deleteArchive(); try check(a.model.state.archives.isEmpty, "explicit archive removal")
    let image = try Sharing.qr(link)
    let detector = CIDetector(ofType: CIDetectorTypeQRCode, context: CIContext(), options: [CIDetectorAccuracy: CIDetectorAccuracyHigh])!
    let features = detector.features(in: CIImage(cgImage: image.cgImage!)).compactMap { ($0 as? CIQRCodeFeature)?.messageString }
    try check(features == [link], "offline QR exact round trip")
    try refuses("secret must not export") { try Sharing.qr("meshfest://staff/not-public") }
    let fixtureURL = Bundle.main.url(forResource: "events", withExtension: "tsv")!
    let fixtures = Dictionary(uniqueKeysWithValues: try String(contentsOf: fixtureURL, encoding: .utf8).split(separator: "\n").map { line in let pair = line.split(separator: "\t", maxSplits: 1); return (String(pair[0]), String(pair[1])) })
    a.model.shareInput(fixtures["event"]!); try check(a.model.state.events.isEmpty, "event confirmation")
    a.model.adoptEvent(); try check(a.model.state.events.first?.active == true, "adopted root")
    a.model.staffInput(fixtures["staff"]!); a.model.foreground(false)
    try check(a.model.state.staffProposal == nil, "background clears provisioning")
    a.model.foreground(true); a.model.staffInput(fixtures["staff"]!); a.model.confirmStaff()
    try check(a.model.state.staffPresent && a.model.state.staff != nil, "wrapped staff import")
    let generation = try a.identity.load().identity.generation
    try refuses("staff generation binding") { try a.staff.access(generation: Data(repeating: 1, count: 16)) { _ in } }
    a.model.foreground(false)
    try refuses("staff background refusal") { try a.staff.access(generation: generation) { _ in } }
    a.model.foreground(true); a.model.staffInput(fixtures["expired"]!); a.model.confirmStaff()
    try check(a.model.state.staff != nil && !a.model.state.locked, "invalid import preserves credential")
    a.model.forgetStaff(); try check(!a.model.state.staffPresent, "forget staff")
    try a.storage.putRecord(kind: .setting, key: Data("supporter-v1".utf8), value: Data("1".utf8))
    a.reopen(); a.model.load(); try check(a.model.store?.active == true, "offline purchase cache")
    a.model.updateProfile(nickname: "Alice", avatar: 0xf8, theme: "daylight", color: 0xff00ff)
    a.reopen(); a.model.load(); try check(a.model.state.avatar == 0xf8 && a.model.state.nicknameRGB == 0xff00ff, "encrypted settings persist")
    let old = a.model.state.myCode!.keys
    a.model.resetIdentity(); try check(!a.model.state.onboarded, "reset requires profile")
    a.model.create(nickname: "Reset", avatar: 1)
    try check(a.model.state.myCode?.keys != old && a.model.state.friends.isEmpty && a.model.state.events.isEmpty && !a.model.state.staffPresent, "identity reset removes trust and history")
    try a.protection.delete(); a.model.protectedDataUnavailable(); a.model.load()
    try check(a.model.state.locked && !a.model.state.onboarded, "key loss never silently recreates")
    a.model.stop(); b.model.stop(); c.model.stop()
}
