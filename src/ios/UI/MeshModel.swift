import Foundation
import SwiftUI
import UIKit
import Security

struct MeshState {
    var loading = true, onboarded = false, locked = false
    var nickname = "", avatar: UInt8 = 1, theme = "afterhours", nicknameRGB: UInt32?
    var channels: [ChannelInfo] = [], selected: String?, rows: [ChannelMessage] = []
    var friends: [FriendCard] = [], archives: [FriendProposal] = [], direct: FriendProposal?, directArchived = false
    var directRows: [DirectMessage] = [], myCode: FriendProposal?, friendProposal: FriendProposal?
    var replacement: FriendCard?, scanned = false, channelProposal: ChannelInfo?
    var events: [EventCard] = [], eventRows: [EventMessage] = [], eventProposal: EventProposal?
    var discoveries: [String] = [], staff: StaffCard?, staffProposal: StaffCard?, staffPresent = false
    var contribution: TransportStats?, power = TransportPowerSetting.auto
    var status = "Nearby connection is off", notice: String?, peers = 0, waitSeconds: UInt64 = 0
    var muted = false, previews: [String: String] = [:], unread: [String: Int] = [:]
    var sendStates: [Data: String] = [:], posted = 0
    var catchup = "Open the app to request bounded nearby history.", delayed: Set<Data> = []
}

/// All feature/radio operations run synchronously on the main actor. Public UI
/// snapshots retain neither database handles nor private signing sessions.
@MainActor final class MeshModel: ObservableObject {
    @Published private(set) var state = MeshState()
    @Published private(set) var store: StoreEntitlements?
    private let identity: IdentityProvider
    private let storage: EncryptedStorage
    private let staff: StaffKeyVault
    private var core: NativeTransport?, owner: NativeChannels?, radio: IOSGattRadio?
    private var joined = ["#general", "#event updates", "#confessions"]
    private var muted: Set<String> = []
    private var links: [UInt64: LinkHandle] = [:]
    private var pending: [UInt64: (Data, Set<UInt64>)] = [:]
    private var cookie: UInt64 = 3, announceAt: UInt64 = 0, epoch: UInt64 = 0
    private var replacingIdentity = false
    private var eventURI: String?, staffCandidate = Data(), staffAt: UInt64 = 0
    private var pulse: Task<Void, Never>?
    private let clock: @MainActor () -> UInt64
    static func native() throws -> MeshModel {
        let vault = try StorageVault.native(), staff = try StaffKeyVault.native()
        let identity = try IdentityProvider.native(state: FeatureResetStore(storage: vault, staff: staff))
        return MeshModel(identity: identity, storage: EncryptedStorage(identity: identity, vault: vault), staff: staff)
    }
    init(identity: IdentityProvider, storage: EncryptedStorage, staff: StaffKeyVault,
         clock: @escaping @MainActor () -> UInt64 = IOSGattRadio.now) {
        self.identity = identity; self.storage = storage; self.staff = staff; self.clock = clock
    }
    deinit { pulse?.cancel() }
    private var wall: Int64 { Int64(Date().timeIntervalSince1970) }
    private func requiredCore() throws -> NativeTransport { guard let core else { throw IdentityFailure.unavailable }; return core }
    private func requiredOwner() throws -> NativeChannels { guard let owner else { throw IdentityFailure.unavailable }; return owner }
    private func perform(_ work: () throws -> Void) {
        do { try work() }
        catch ChannelError.Invalid { state.notice = "Check channel words and UTF-8 byte limits. Unsupported invisible characters are not allowed." }
        catch ChannelError.Limited { state.notice = "Wait for the countdown before posting again." }
        catch MessagingError.Invalid { state.notice = "Check the friend code or message. No friend was added." }
        catch MessagingError.Stale { state.notice = "This identity is awaiting replacement. Scan and confirm the new device." }
        catch MessagingError.Busy { state.notice = "Not sent. Connect to nearby people or wait for queue space, then retry." }
        catch OrganizerError.Invalid { state.notice = "Invalid event or staff credential. Existing trust was preserved." }
        catch OrganizerError.Authority { state.notice = "Event authority is unavailable or expired. Scan and confirm a current official code." }
        catch OrganizerError.Busy { state.notice = "Organizer work is busy. Wait or connect, then retry." }
        catch { unavailable() }
    }
    private func setting(_ name: String) throws -> String? {
        try storage.getRecord(kind: .setting, key: Data(name.utf8)).flatMap { String(data: $0, encoding: .utf8) }
    }
    private func put(_ name: String, _ value: String) throws {
        try storage.putRecord(kind: .setting, key: Data(name.utf8), value: Data(value.utf8))
    }
    private func persist() throws {
        try put("ui-profile-v1", "\(state.avatar)|0|\(state.nickname)")
        try put("ui-channels-v1", joined.joined(separator: "\n"))
        try put("ui-muted-v1", muted.sorted().joined(separator: "\n"))
        try put("ui-cosmetics-v1", "\(state.theme)|\(state.nicknameRGB.map(String.init) ?? "")")
        try put("ui-power-v1", state.power == .normal ? "NORMAL" : state.power == .saver ? "SAVER" : "AUTO")
    }
    private func initialize() throws {
        let info = try identity.load()
        var nonce: UInt64 = 0
        repeat { guard SecRandomCopyBytes(kSecRandomDefault, MemoryLayout<UInt64>.size, &nonce) == errSecSuccess else { throw IdentityFailure.unavailable } } while nonce == 0
        core = try storage.operation { try NativeTransport.newIos(store: $0, identity: info.identity, instanceNonce: nonce, monotonicMs: clock()) }
        owner = try NativeChannels(identity: info.identity, now: clock())
        state.myCode = try friendCode(identity: info.identity, nickname: state.nickname)
        store = StoreEntitlements(cached: try setting("supporter-v1") == "1") { [weak self] value in
            guard let self, self.state.onboarded, !self.state.locked else { throw IdentityFailure.unavailable }
            try self.put("supporter-v1", value ? "1" : "0")
        }
        state.onboarded = true; state.loading = false; state.locked = false
        startPulse()
    }
    func load() {
        if state.onboarded { refresh(); return }
        do { _ = try identity.load() }
        catch IdentityFailure.missing { state = MeshState(); state.loading = false; return }
        catch { unavailable(); return }
        perform {
            try storage.reopen()
            guard let profile = try setting("ui-profile-v1")?.split(separator: "|", maxSplits: 2, omittingEmptySubsequences: false), profile.count == 3,
                  let avatar = UInt8(profile[0]), (1...8).contains(avatar) else { throw IdentityFailure.recoveryRequired }
            state.nickname = try channelNickname(raw: String(profile[2])); state.avatar = avatar
            let cosmetics = try setting("ui-cosmetics-v1")?.split(separator: "|", maxSplits: 1, omittingEmptySubsequences: false)
            state.theme = cosmetics?.first.map(String.init) ?? "afterhours"
            state.nicknameRGB = cosmetics?.last.flatMap { UInt32($0) }.flatMap { $0 <= 0xffffff ? $0 : nil }
            let power = try setting("ui-power-v1"); state.power = power == "NORMAL" ? .normal : power == "SAVER" ? .saver : .auto
            joined = try (setting("ui-channels-v1")?.components(separatedBy: "\n") ?? joined).prefix(33).map { try channelInfo(name: $0).name }
            joined = joined.reduce(into: []) { if !$0.contains($1) { $0.append($1) } }
            muted = Set(try setting("ui-muted-v1")?.components(separatedBy: "\n").filter { joined.contains($0) } ?? [])
            try initialize(); try refreshThrowing()
        }
    }
    func create(nickname: String, avatar: UInt8) { perform {
        guard !state.onboarded, (1...8).contains(avatar) else { return }
        state.nickname = try channelNickname(raw: nickname); state.avatar = avatar
        if !replacingIdentity { _ = try identity.create(); try storage.create() }
        try persist(); try initialize(); replacingIdentity = false; try refreshThrowing()
    } }
    func updateProfile(nickname: String, avatar: UInt8, theme: String, color: UInt32?) { perform {
        guard (1...8).contains(avatar) else { throw ChannelError.Invalid }
        state.nickname = try channelNickname(raw: nickname); state.avatar = avatar
        state.theme = ThemeTokens.selected(theme, active: store?.active == true).id
        state.nicknameRGB = store?.active == true ? color.flatMap { $0 <= 0xffffff ? $0 : nil } : nil
        try persist(); state.myCode = try friendCode(identity: identity.load().identity, nickname: state.nickname); try refreshThrowing()
    } }
    func refresh() { if state.onboarded { perform { try refreshThrowing() } } }
    private func refreshThrowing() throws {
        let core = try requiredCore(), owner = try requiredOwner()
        try owner.setCosmetics(supporter: store?.active == true, rgb: state.nicknameRGB)
        try drainCatchup()
        state.channels = try joined.map { try channelInfo(name: $0) }
        state.friends = try storage.operation { try core.friendCards(store: $0, now: clock()) }
        state.archives = try archives()
        state.events = try storage.operation { try core.eventCards(store: $0, wall: wall) }
        state.eventRows = try storage.operation { try core.eventMessages(store: $0, wall: wall, now: clock()) }
        state.discoveries = try core.eventDiscoveries(now: clock())
        state.staffPresent = try staff.present()
        state.staff = state.staffPresent && staff.foreground ? try staff.access(generation: identity.load().identity.generation) { try staffProposal(uri: $0) } : nil
        state.contribution = try core.contributionStats(now: clock()); state.peers = links.count
        if let name = state.selected {
            state.rows = try storage.operation { try core.messagingChannelHistory(store: $0, owner: owner, name: name, nickname: state.nickname) }
            state.waitSeconds = (try owner.waitMs(name: name, reaction: false, now: clock()) + 999) / 1000
            state.muted = muted.contains(name)
        } else { state.rows = []; state.waitSeconds = 0 }
        if let direct = state.direct { state.directRows = try storage.operation { try core.directHistory(store: $0, keys: direct.keys, now: clock(), wall: wall) } }
        else { state.directRows = [] }
    }
    func select(_ name: String?) { perform {
        state.notice = nil; state.direct = nil; state.selected = try name.map { try channelInfo(name: $0).name }
        if let name = state.selected { state.unread[name] = 0 }; try refreshThrowing()
    } }
    func join(_ name: String) { perform {
        let channel = try channelInfo(name: name)
        if !joined.contains(channel.name) {
            let count = try joined.filter { try channelInfo(name: $0).private }.count
            guard !channel.private || count < (store?.active == true ? 30 : 3) else { state.notice = "Private channel slots are full. Existing channels stay available."; return }
            joined.append(channel.name); try persist()
        }
        state.channelProposal = nil; state.selected = channel.name; state.direct = nil; try refreshThrowing()
    } }
    func mute() { perform { if let name = state.selected { if !muted.insert(name).inserted { muted.remove(name) } }; try persist(); try refreshThrowing() } }
    func leave() { perform {
        if let name = state.selected, try channelInfo(name: name).private { joined.removeAll { $0 == name }; muted.remove(name); state.selected = nil; try persist() }
        try refreshThrowing()
    } }
    func clearHistory() { perform {
        if let direct = state.direct { try storage.deleteHistory(conversation: direct.keys, direct: true) }
        else if let name = state.selected { try storage.deleteHistory(conversation: channelInfo(name: name).id, direct: false) }
        try refreshThrowing()
    } }
    func resetIdentity() { perform {
        stop(); clearCandidates(); core = nil; owner = nil; store = nil
        _ = try identity.reset(); try storage.create()
        joined = ["#general", "#event updates", "#confessions"]; muted = []; state = MeshState(); state.loading = false; replacingIdentity = true
    } }
    private func unavailable() {
        stop(); pulse?.cancel(); pulse = nil; clearCandidates(); core = nil; owner = nil; store = nil
        state = MeshState(); state.loading = false; state.locked = true
        state.notice = "Protected data is unavailable. Unlock and reopen, or explicitly reset identity and local data. Nothing was recreated."
    }
    func power(_ value: TransportPowerSetting) { perform { state.power = value; try persist(); radio?.setPower(value); try refreshThrowing() } }
    func resetContribution() { perform { try requiredCore().resetContributionStats(now: clock()); try refreshThrowing() } }
    func foreground(_ active: Bool) {
        staff.setForeground(active)
        if !active { clearCandidates(); try? core?.forgetStaffOperations(); state.staff = nil }
        else { load() }
    }
    func protectedDataUnavailable() { unavailable() }
    private func clearCandidates() {
        staffCandidate.resetBytes(in: 0..<staffCandidate.count); staffCandidate.removeAll()
        eventURI = nil; state.eventProposal = nil; state.staffProposal = nil
        state.friendProposal = nil; state.channelProposal = nil; state.scanned = false
    }
    func cancelProposal() { clearCandidates() }
    func shareInput(_ uri: String, scanned: Bool = false) { perform {
        clearCandidates()
        if let event = try? eventProposal(uri: uri) { eventURI = uri; state.eventProposal = event; return }
        if let channel = try? channelLink(uri: uri) { state.channelProposal = channel; return }
        let proposal = try friendProposal(uri: uri)
        guard state.replacement == nil || scanned else { state.notice = "Replacement requires a fresh scan of the new device."; return }
        state.friendProposal = proposal; state.scanned = scanned
    } }
    func confirmFriend(_ petname: String) { perform {
        guard let proposal = state.friendProposal else { return }
        let core = try requiredCore()
        stop()
        let effects = try storage.operation { db in try identity.messaging { try core.confirmFriend(store: db, provider: $0, uri: proposal.uri, petname: petname, previous: state.replacement?.handle, now: clock()) } }
        apply(effects); state.replacement = nil; clearCandidates(); try refreshThrowing()
    } }
    func changeFriend(_ friend: FriendCard, replace: Bool) { perform {
        try archive(friend); stop()
        let effects = try storage.operation { try requiredCore().changeFriend(store: $0, friend: friend.handle, replace: replace, now: clock()) }
        apply(effects); state.replacement = replace ? friend : nil; state.direct = nil; clearCandidates(); try refreshThrowing()
    } }
    func openDirect(_ friend: FriendProposal, archived: Bool) {
        state.direct = friend; state.directArchived = archived; state.selected = nil; refresh()
    }
    func openDirect(_ friend: FriendCard) {
        openDirect(FriendProposal(uri: "", nickname: friend.petname, fingerprint: friend.fingerprint, keys: friend.keys), archived: false)
    }
    private func archives() throws -> [FriendProposal] {
        try (0..<8).flatMap { index in try setting("ui-old-friends-\(index)")?.split(separator: "\n").map { try friendProposal(uri: String($0)) } ?? [] }
    }
    private func archive(_ friend: FriendCard) throws {
        if try archives().contains(where: { $0.keys == friend.keys }) { return }
        let metadata = PublicIdentity(generation: Data(), signingKey: Data(friend.keys.prefix(32)), agreementKey: Data(friend.keys.suffix(32)), senderId: Data(), agreementHint: Data())
        let uri = try friendCode(identity: metadata, nickname: friend.petname).uri
        for slot in 0..<8 {
            var values = try setting("ui-old-friends-\(slot)")?.components(separatedBy: "\n").filter { !$0.isEmpty } ?? []
            if values.count < 8 { values.append(uri); try put("ui-old-friends-\(slot)", values.joined(separator: "\n")); return }
        }
        throw MessagingError.Busy
    }
    func deleteArchive() { perform {
        guard state.directArchived, let old = state.direct else { return }
        try storage.deleteHistory(conversation: old.keys, direct: true)
        for slot in 0..<8 {
            let values = try setting("ui-old-friends-\(slot)")?.split(separator: "\n").map(String.init) ?? []
            try put("ui-old-friends-\(slot)", values.filter { try friendProposal(uri: $0).keys != old.keys }.joined(separator: "\n"))
        }
        state.direct = nil; try refreshThrowing()
    } }
    func adoptEvent() { perform {
        guard let uri = eventURI else { return }
        try storage.operation { try requiredCore().confirmEvent(store: $0, uri: uri, now: clock(), wall: wall) }
        clearCandidates(); try refreshThrowing()
    } }
    func removeEvent(_ key: Data) { perform { try storage.operation { try requiredCore().removeEvent(store: $0, key: key, wall: wall) }; try refreshThrowing() } }
    func staffInput(_ uri: String) { perform {
        clearCandidates(); guard staff.foreground else { throw OrganizerError.Authority }
        state.staffProposal = try staffProposal(uri: uri); staffCandidate = Data(uri.utf8); staffAt = clock()
    } }
    func confirmStaff() { perform {
        guard !staffCandidate.isEmpty, clock() - staffAt < 60_000 else { clearCandidates(); throw OrganizerError.Authority }
        defer { clearCandidates() }
        try storage.operation { db in
            try staff.importConfirmed(generation: identity.load().identity.generation, candidate: &staffCandidate) { uri in
                let session = try requiredCore().importStaffKey(store: db, uri: uri, now: clock(), wall: wall); try session.invalidate()
            }
        }
        try refreshThrowing()
    } }
    func forgetStaff() { perform { try requiredCore().forgetStaffOperations(); try staff.forget(); try refreshThrowing() } }

    // Radio integration and send handling are serialized with protected calls.
    func connect() { perform {
        guard state.onboarded, radio == nil else { return }
        let core = try requiredCore(); epoch += 1; let generation = epoch
        let next = IOSGattRadio(core: core, protectedAvailable: { [weak self] in self?.state.onboarded == true },
            event: { [weak self] id, event in guard let self, self.epoch == generation else { return }; self.event(id, event) },
            state: { [weak self] value in guard let self, self.epoch == generation else { return }; self.radioState(value) },
            catchUp: { [weak self] ids in guard let self, self.epoch == generation else { return }; self.catchUp(ids) },
            egress: { [weak self] send, submit in guard let self else { return false }; return try self.egress(send, submit) })
        radio = next; next.start(); next.setPower(state.power); announceAt = 0; try refreshThrowing()
    } }
    func stop() {
        epoch += 1; let old = radio; radio = nil; old?.stop(); links.removeAll()
        for (_, item) in pending where state.sendStates[item.0] != "Handed to mesh · delivery unknown" { state.sendStates[item.0] = "Not sent · retry" }
        pending.removeAll(); state.peers = 0; state.status = "Nearby connection is off"
    }
    private func egress(_ send: TransportSend, _ submit: () -> Bool) throws -> Bool {
        let core = try requiredCore()
        if try core.messageNeedsAuthorization(link: send.link, token: send.token) {
            let organizer = try storage.operation { db in
                let guarded = try core.authorizeOrganizerEgress(store: db, link: send.link, token: send.token, wall: wall)
                if !guarded { try core.authorizeMessageEgress(store: db, link: send.link, token: send.token) }; return guarded
            }
            if organizer && !staff.foreground { return false }
        }
        return submit()
    }
    private func apply(_ effects: TransportEffects) { _ = radio?.operation { _ in effects } }
    private func radioState(_ value: IOSRadioState) {
        switch value {
        case .locked: unavailable(); return
        case .foreground: state.status = "Searching for nearby people…"
        case .backgroundLimited: state.status = "iOS background relay is limited. Open the app for catch-up."
        case .restoring: state.status = "Restoring nearby connections; old radio state is discarded."
        case .permissionRequired: state.status = "Allow Bluetooth access in Settings, then reconnect."
        case .radioOff: state.status = "Turn on Bluetooth to connect."
        case .starting: state.status = "Starting nearby connections…"
        case .stopped, .unavailable: stop()
        }
    }
    private func catchUp(_ ids: [UInt64]) {
        perform {
            let core = try requiredCore()
            for id in ids {
                guard let link = links[id] else { continue }
                do { apply(try core.requestCatchup(link: link, now: clock())) }
                catch TransportError.Busy { continue }
                catch TransportError.Stale { continue }
            }
            try drainCatchup()
        }
    }
    private func drainCatchup() throws {
        let core = try requiredCore(), owner = try requiredOwner()
        let progress = try storage.operation { db in try identity.messaging {
            try core.processCatchup(store: db, provider: $0, owner: owner, now: clock(), wall: wall)
        } }
        for key in progress.arrivals {
            if state.delayed.count >= 100, let old = state.delayed.first { state.delayed.remove(old) }
            state.delayed.insert(key)
        }
        state.catchup = progress.active > 0 ? "Requesting bounded nearby history…" :
            "Catch-up: \(progress.complete) completed, \(progress.incomplete) limited or interrupted. This does not establish delivery or complete history."
    }
    private func event(_ id: UInt64, _ event: TransportEvent) { perform {
        let core = try requiredCore(), owner = try requiredOwner()
        switch event {
        case let .admitted(link, _, _):
            links[id] = link; announceAt = 0
            catchUp([id])
            apply(try identity.messaging { try core.prepareProof(link: link, provider: $0, now: clock()) })
        case let .closed(link):
            links.removeValue(forKey: id); try owner.disconnected(link: link, now: clock())
            for token in Array(pending.keys) { pending[token]?.1.remove(id); if pending[token]?.1.isEmpty == true { pending.removeValue(forKey: token) } }
        case let .finished(_, token, status):
            if let item = pending[token] {
                if status == .nativeComplete { state.sendStates[item.0] = "Handed to mesh · delivery unknown" }
                pending[token]?.1.remove(id)
                if pending[token]?.1.isEmpty == true { pending.removeValue(forKey: token) }
            }
        case let .received(link, bytes, intake):
            if intake != .duplicate && intake != .deferredSync {
                if intake == .pending { _ = try storage.operation { db in try identity.messaging { try core.authenticateMessage(store: db, provider: $0, link: link, bytes: bytes, now: clock(), wall: wall) } } }
                if bytes.count >= 26, ![UInt8(2), 3, 7].contains(bytes[1]) {
                    for other in links.keys where other != id { cookie += 1; _ = radio?.enqueue(other, bytes: bytes, traffic: .forwarded, cookie: cookie) }
                }
                if intake == .unverified, bytes.count >= 26, let name = try joined.first(where: { try channelInfo(name: $0).id == bytes.subdata(in: 20..<24) }) {
                    let added = try storage.operation { try owner.accept(store: $0, receipt: ChannelReceipt(link: link, bytes: bytes, intake: intake, own: false, wall: wall, now: clock())) }
                    if added {
                        state.previews[name] = try storage.operation { try owner.history(store: $0, name: name, ownNickname: state.nickname).last?.text } ?? "Quiet so far"
                        if name != state.selected && !muted.contains(name) { state.unread[name] = min(999, (state.unread[name] ?? 0) + 1) }
                    }
                }
            }
        }
        try refreshThrowing()
    } }
    @discardableResult private func submitProtected(_ work: (NativeTransport, UInt64) throws -> MessageSubmission) throws -> Bool {
        guard let radio, !links.isEmpty, pending.count < 32 else { throw MessagingError.Busy }
        cookie += 1; let token = cookie; var sent: MessageSubmission?
        var failure: Error?
        let applied = radio.operation { core in
            do { let result = try work(core, token); sent = result; return result.effects }
            catch { failure = error; return TransportEffects(sends: [], events: []) }
        }
        if let failure { throw failure }
        guard applied, let sent else { throw MessagingError.Busy }
        if state.sendStates.count >= 100, let key = state.sendStates.keys.first { state.sendStates.removeValue(forKey: key) }
        state.sendStates[sent.id] = sent.queued ? "Queued · delivery unknown" : "Stored locally · not sent"
        if sent.queued { pending[token] = (sent.id, Set(links.filter { sent.queuedLinks.contains($0.value) }.keys)) }
        return sent.queued
    }
    func send(_ text: String, pinExpiry: UInt32? = nil) { perform {
        if let direct = state.direct {
            guard !state.directArchived, let friend = state.friends.first(where: { $0.keys == direct.keys && !$0.replacing }) else { throw MessagingError.Stale }
            if try submitProtected({ core, token in try storage.operation { db in try identity.messaging { try core.sendDirect(store: db, provider: $0, friend: friend.handle, content: .chat(text: text), cookie: token, now: clock(), wall: wall) } } }) { state.posted += 1 }
        } else if let name = state.selected {
            if name == "#event updates" && state.events.contains(where: { $0.active }) {
                if try submitProtected({ core, token in try storage.operation { db in try staff.access(generation: identity.load().identity.generation) { uri in
                    let session = try core.importStaffKey(store: db, uri: uri, now: clock(), wall: wall); defer { try? session.invalidate() }
                    return try core.postEvent(store: db, session: session, nickname: state.nickname, text: text, avatar: state.avatar, pinExpiry: pinExpiry, cookie: token, now: clock(), wall: wall)
                } } }) { state.posted += 1 }
            } else {
                let bytes = try requiredOwner().compose(name: name, nickname: state.nickname, avatar: state.avatar, message: text, wall: UInt32(clamping: wall), now: clock())
                if try channelInfo(name: name).private {
                    if try submitProtected({ core, token in try storage.operation { db in try identity.messaging { try core.sendSigned(store: db, provider: $0, bytes: bytes, cookie: token, now: clock(), wall: wall) } } }) { state.posted += 1 }
                } else { try submitClear(bytes); state.posted += 1 }
            }
        }
        try refreshThrowing()
    } }
    private func submitClear(_ bytes: Data) throws {
        guard let radio, !links.isEmpty, pending.count < 32 else { throw MessagingError.Busy }
        cookie += 1; let token = cookie, id = bytes.subdata(in: 4..<12)
        let queued = Set(links.keys.filter { radio.enqueue($0, bytes: bytes, traffic: .own, cookie: token) })
        guard !queued.isEmpty else { throw MessagingError.Busy }
        pending[token] = (id, queued); state.sendStates[id] = "Queued · delivery unknown"
        _ = try storage.operation { try requiredOwner().accept(store: $0, receipt: ChannelReceipt(link: nil, bytes: bytes, intake: .unverified, own: true, wall: wall, now: clock())) }
    }
    func react(_ id: Data, code: UInt8, remove: Bool) { perform {
        if let direct = state.direct {
            guard !state.directArchived, let friend = state.friends.first(where: { $0.keys == direct.keys && !$0.replacing }) else { throw MessagingError.Stale }
            try submitProtected { core, token in try storage.operation { db in try identity.messaging { try core.sendDirect(store: db, provider: $0, friend: friend.handle, content: .reaction(target: id, remove: remove, code: code), cookie: token, now: clock(), wall: wall) } } }
        } else if let name = state.selected { try submitClear(requiredOwner().reaction(name: name, target: id, code: code, remove: remove, now: clock())) }
        try refreshThrowing()
    } }
    private func startPulse() {
        guard pulse == nil else { return }
        pulse = Task { @MainActor [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: 1_000_000_000)
                guard !Task.isCancelled, let self else { return }
                self.perform {
                    guard self.state.onboarded else { return }
                    let core = try self.requiredCore()
                    if !self.links.isEmpty {
                        _ = try self.storage.operation { db in try self.identity.messaging { try core.retryMessages(store: db, provider: $0, now: self.clock(), wall: self.wall) } }
                        let credential = self.state.staff.flatMap { self.wall >= Int64($0.notBefore) && self.wall <= Int64($0.notAfter) ? $0.credential : nil }
                        self.apply(try self.storage.operation { try core.organizerTick(store: $0, credential: credential, now: self.clock(), wall: self.wall) })
                        if self.clock() >= self.announceAt {
                            let bytes = try self.requiredOwner().announce(nickname: self.state.nickname, avatar: self.state.avatar, peers: UInt8(clamping: self.links.count), wall: UInt32(clamping: self.wall))
                            try self.submitProtected { core, token in try self.storage.operation { db in try self.identity.messaging { try core.sendSigned(store: db, provider: $0, bytes: bytes, cookie: token, now: self.clock(), wall: self.wall) } } }
                            self.announceAt = self.clock() + (self.radio?.appliedPower?.announceMs ?? 30_000)
                        }
                    }
                    try self.refreshThrowing()
                }
            }
        }
    }
}
