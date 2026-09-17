import SwiftUI
import UIKit

private let animals = ["🐈", "🦊", "🐼", "🐸", "🦉", "🐰", "🐢", "🦋"]
private let reactions = ["👍", "❤️", "😂", "🎉", "🔥", "👀", "🙏", "👎"]

private struct ConfirmAction: Identifiable {
    let id = UUID(), title: String, explanation: String, action: () -> Void
}
private struct ShareItem: Identifiable {
    let id = UUID(), uri: String, title: String, explanation: String
}

struct MeshView: View {
    @ObservedObject var model: MeshModel
    @Environment(\.scenePhase) private var phase
    @State private var tab = 0, settings = false, join = false, scan = false, input = false, eventSettings = false
    @State private var shared: ShareItem?
    @State private var confirmation: ConfirmAction?
    private var state: MeshState { model.state }
    private var theme: ThemeTokens { ThemeTokens.selected(state.theme, active: model.store?.active == true) }
    var body: some View {
        Group {
            if state.loading { ProgressView("Opening protected data…") }
            else if state.locked { VStack(spacing: 20) {
                Text("Protected data unavailable").font(.title2)
                Text(state.notice ?? "Unlock and reopen the app.")
                Button("Reopen protected data") { model.load() }
                Button("Reset identity and local data", role: .destructive) { resetConfirmation() }
            }.padding() }
            else if !state.onboarded { OnboardingView(model: model) }
            else {
                NavigationView {
                    VStack(spacing: 0) {
                        HStack {
                            Image(systemName: state.peers > 0 ? "circle.fill" : "circle")
                            Text(state.peers > 0 ? "\(state.peers) direct connections · \(state.status)" : state.status).font(.caption)
                            Spacer()
                            Button("Connect") { model.connect() }
                        }.padding().accessibilityElement(children: .combine)
                        if let notice = state.notice { Text(notice).font(.callout).foregroundColor(ThemeTokens.color(theme.error)).padding() }
                        if state.selected != nil || state.direct != nil {
                            ChatView(model: model, theme: theme, confirm: { confirmation = $0 }, share: { shared = $0 })
                        } else {
                            TabView(selection: $tab) {
                                channelList.tabItem { Label("Channels", systemImage: "lock.open") }.tag(0)
                                messages.tabItem { Label("Messages", systemImage: "lock") }.tag(1)
                                friends.tabItem { Label("Friends", systemImage: "person.2") }.tag(2)
                            }
                        }
                    }
                    .background(ThemeTokens.color(theme.background))
                    .navigationTitle("MeshChat").navigationBarTitleDisplayMode(.inline)
                    .toolbar {
                        ToolbarItem(placement: .navigationBarLeading) { if state.selected != nil || state.direct != nil { Button("Back") { model.select(nil) } } }
                        ToolbarItem(placement: .navigationBarTrailing) { Button("Settings") { settings = true } }
                    }
                }.navigationViewStyle(.stack)
            }
        }
        .tint(ThemeTokens.color(theme.accent))
        .preferredColorScheme(theme.light ? .light : .dark)
        .onAppear { model.load() }
        .onChange(of: phase) { model.foreground($0 == .active) }
        .onReceive(NotificationCenter.default.publisher(for: UIApplication.protectedDataWillBecomeUnavailableNotification)) { _ in model.protectedDataUnavailable() }
        .onOpenURL { model.shareInput($0.absoluteString) }
        .sheet(isPresented: $settings) { SettingsView(model: model) }
        .sheet(isPresented: $join) { JoinView(model: model) }
        .sheet(isPresented: $scan) { NavigationView { CodeScanner { value in scan = false; model.shareInput(value, scanned: true) }.navigationTitle("Scan a code").toolbar { Button("Cancel") { scan = false } } } }
        .sheet(isPresented: $input) { LinkInput { model.shareInput($0) } }
        .sheet(isPresented: $eventSettings) { EventSettings(model: model) }
        .sheet(item: $shared) { PublicShareView(uri: $0.uri, title: $0.title, explanation: $0.explanation) }
        .sheet(item: $confirmation) { item in ConfirmationView(title: item.title, explanation: item.explanation, destructive: true) { item.action(); confirmation = nil } }
        .sheet(isPresented: Binding(get: { state.channelProposal != nil || state.friendProposal != nil || state.eventProposal != nil }, set: { if !$0 { model.cancelProposal() } })) {
            ProposalView(model: model)
        }
    }
    private var channelList: some View {
        List {
            Text("Open conversations. Readable on air.").font(.callout)
            ForEach(state.channels, id: \.name) { channel in Button { model.select(channel.name) } label: {
                HStack {
                    Image(systemName: "lock.open")
                    VStack(alignment: .leading, spacing: 5) {
                        Text(verbatim: channel.name.replacingOccurrences(of: "|", with: " ")).font(.headline)
                        Text(channel.anonymous ? "Fresh identity for every post" : channel.private ? "Anyone with the words can read" : "Open to everyone").font(.caption)
                        Text(verbatim: state.previews[channel.name] ?? "Quiet so far").lineLimit(1)
                    }
                    Spacer(); if let unread = state.unread[channel.name], unread > 0 { Text("\(unread) unread") }
                }
            }.listRowBackground(ThemeTokens.color(theme.surface)) }
            Button("Join a word-triple channel") { join = true }
            Button("Scan QR") { scan = true }; Button("Paste a shared link") { input = true }
            Button("Event trust and staff") { eventSettings = true }
        }
    }
    private var messages: some View {
        List {
            Text("End-to-end encrypted conversations with pinned friends. There is no plaintext fallback.")
            ForEach(state.friends, id: \.keys) { friend in Button { model.openDirect(friend) } label: {
                Label { VStack(alignment: .leading) { Text(verbatim: friend.petname); Text(friend.replacing ? "Replacement pending · sending stopped" : "Encrypted for this verified device").font(.caption) } } icon: { Image(systemName: "lock.fill") }
            } }
            if state.friends.isEmpty { Text("Add and confirm a friend's code before starting a message.") }
            Section("Old identities · read only") {
                ForEach(state.archives, id: \.keys) { old in Button { model.openDirect(old, archived: true) } label: { Text(verbatim: old.nickname + " · " + old.fingerprint) } }
            }
        }
    }
    private var friends: some View {
        List {
            if let code = state.myCode { Button("My friend code") { shared = ShareItem(uri: code.uri, title: code.nickname, explanation: "Compare the fingerprint in person. Show your code so verification is mutual.") } }
            Button("Scan a friend's code") { scan = true }; Button("Paste a friend link") { input = true }
            Text("An authenticated response does not prove distance or direct radio range.")
            ForEach(state.friends, id: \.keys) { friend in VStack(alignment: .leading, spacing: 10) {
                Text(verbatim: friend.petname).font(.headline)
                Label("Pinned identity", systemImage: "checkmark.shield.fill")
                Text(verbatim: friend.fingerprint).font(.system(.caption, design: .monospaced))
                Text(friend.fresh ? "Recent authenticated response" : friend.responseAgeMs.map { "Last authenticated response: \($0 / 1000) seconds ago" } ?? "No authenticated response yet")
                Button("Message") { model.openDirect(friend) }
                Button("Replace this device") {
                    confirmation = ConfirmAction(title: "Replace pinned device?", explanation: "Sending to this identity stops after confirmation. Its old history remains separate. Scan the new device and compare both fingerprints.") { model.changeFriend(friend, replace: true); scan = true }
                }
                Button("Remove friend", role: .destructive) {
                    confirmation = ConfirmAction(title: "Remove this friend?", explanation: "The pin is removed. Old encrypted history stays under the old identity and cannot send.") { model.changeFriend(friend, replace: false) }
                }
            } }
        }
    }
    private func resetConfirmation() {
        confirmation = ConfirmAction(title: "Reset identity and local data?", explanation: "This permanently removes local history, friend pins, event trust and staff credentials. Friends must verify your new code. A lost key cannot recover old data.") { model.resetIdentity() }
    }
}

private struct OnboardingView: View {
    @ObservedObject var model: MeshModel
    @State private var step = 0, nickname = "", avatar: UInt8 = 1
    var body: some View {
        VStack(alignment: .leading, spacing: 22) {
            Text("Nearby, together").font(.caption); Text("Welcome to MeshChat").font(.largeTitle)
            if step == 0 {
                TextField("Nickname", text: $nickname).textInputAutocapitalization(.never).disableAutocorrection(true)
                Picker("Animal avatar", selection: $avatar) { ForEach(0..<animals.count, id: \.self) { Text(animals[$0]).tag(UInt8($0 + 1)) } }
                Text("Nicknames are public claims. A nickname never proves that someone is staff or a verified friend.")
            } else if step == 1 {
                Text("Bluetooth connects nearby phones. Tap Connect when you are ready to grant access. Camera access is optional and requested only when scanning.")
                Text("In airplane mode, turn Bluetooth back on. iOS background discovery and relay are limited; open the app to reconnect and catch up.")
            } else {
                Text("Channels are readable on air. Only Messages with pinned friends are end-to-end encrypted. Compare friend fingerprints in person.")
                Text("Delivery and range are not guaranteed. Handed to mesh means a native radio handoff, never delivery. No account or internet is needed to chat nearby.")
            }
            if let notice = model.state.notice { Text(notice) }
            if step < 2 { Button("Continue") { step += 1 }.disabled(step == 0 && (try? channelNickname(raw: nickname)) == nil) }
            else { Button("Create my local identity") { model.create(nickname: nickname, avatar: avatar) } }
            if step > 0 { Button("Back") { step -= 1 } }
        }.padding(28)
    }
}

private struct ChatView: View {
    @ObservedObject var model: MeshModel
    let theme: ThemeTokens
    let confirm: (ConfirmAction) -> Void
    let share: (ShareItem) -> Void
    @State private var text = "", pin = false
    private var state: MeshState { model.state }
    private var isDirect: Bool { state.direct != nil }
    var body: some View {
        VStack {
            HStack {
                Image(systemName: isDirect ? "lock.fill" : "lock.open")
                VStack(alignment: .leading) {
                    Text(verbatim: state.direct?.nickname ?? state.selected ?? "").font(.headline)
                    Text(isDirect ? "Encrypted for this pinned device" : "Anyone in this channel can read · not encrypted").font(.caption)
                }
                Spacer()
                Menu("Conversation options") {
                    if let name = state.selected, let channel = try? channelInfo(name: name), channel.private {
                        Button("Share channel") { if let uri = try? Sharing.channelURI(name) { share(ShareItem(uri: uri, title: name.replacingOccurrences(of: "|", with: " "), explanation: "Anyone with these words can read. This channel is not encrypted.")) } }
                        Button("Leave channel") { confirm(ConfirmAction(title: "Leave channel?", explanation: "You can join again with the same words.") { model.leave() }) }
                    }
                    if state.selected != nil { Button(state.muted ? "Unmute" : "Mute") { model.mute() } }
                    Button("Clear local history", role: .destructive) { confirm(ConfirmAction(title: "Clear local history?", explanation: "This removes the local conversation. Copies on other phones are unchanged.") { model.clearHistory() }) }
                    if state.directArchived { Button("Delete old conversation", role: .destructive) { confirm(ConfirmAction(title: "Delete old conversation?", explanation: "This permanently removes the retained local history and its old identity entry.") { model.deleteArchive() }) } }
                }
            }.padding()
            ScrollView { LazyVStack(alignment: .leading, spacing: 18) {
                if isDirect {
                    ForEach(state.directRows, id: \.id) { row in VStack(alignment: .leading, spacing: 6) {
                        Text(row.own ? "You" : state.direct?.nickname ?? "Pinned friend").font(.caption)
                        Text(verbatim: row.text)
                        if let status = state.sendStates[row.id] { Text(status).font(.caption) }
                        reactionRow(id: row.id, counts: row.reactions.map(UInt16.init), own: row.ownReaction)
                    }.padding().background(ThemeTokens.color(theme.surface)).cornerRadius(12) }
                } else if state.selected == "#event updates" {
                    Text(state.events.contains(where: { $0.active }) ? "Trusted event updates are marked separately. Names alone prove nothing." : "No active event root. Posts are unverified until you scan and confirm an official event code.")
                    ForEach(state.eventRows, id: \.contentKey) { row in VStack(alignment: .leading, spacing: 6) {
                        if let label = row.staffLabel { Label("Event Staff · " + label, systemImage: "checkmark.shield.fill").padding(6).background(Color.yellow.opacity(0.2)) }
                        else { Text("Unverified broadcast").font(.caption) }
                        if row.pinned { Label("Pinned until its signed expiry", systemImage: "pin.fill") }
                        Text(verbatim: row.nickname).lineLimit(1); Text(verbatim: row.text)
                    }.padding().background(ThemeTokens.color(theme.surface)).cornerRadius(12) }
                } else {
                    ForEach(Array(state.rows.enumerated()), id: \.offset) { _, row in VStack(alignment: .leading, spacing: 6) {
                        if let petname = row.verifiedPetname { Label { Text(verbatim: petname) } icon: { Image(systemName: "checkmark.shield.fill") }.padding(5).background(Color.green.opacity(0.15)) }
                        HStack {
                            Text(animals[Int(max(1, min(8, row.avatar))) - 1])
                            Text(verbatim: row.nickname).foregroundColor(ThemeTokens.color(theme.readableNickname(row.nicknameRgb ?? theme.text))).lineLimit(1)
                            Text(String(row.sender.map { String(format: "%02x", $0) }.joined().suffix(4))).font(.caption)
                        }
                        if row.confusable { Text("Nickname resembles yours · identity is unverified").font(.caption) }
                        if let warning = row.claimWarning { Text(verbatim: warning).font(.caption) }
                        Text(verbatim: row.text)
                        if let status = state.sendStates[row.id] { Text(status).font(.caption) }
                        reactionRow(id: row.id, counts: row.reactions, own: row.ownReaction)
                    }.padding().background(ThemeTokens.color(theme.surface)).cornerRadius(12) }
                }
                if (isDirect && state.directRows.isEmpty) || (!isDirect && state.rows.isEmpty && state.eventRows.isEmpty) { Text("Quiet so far. Messages appear when people within mesh range post.") }
            }.padding() }
            if state.directArchived { Text("Old identity · read only. No plaintext fallback.").padding() }
            else {
                if state.selected == "#event updates", state.staff != nil { Toggle("Pin for 15 minutes", isOn: $pin).padding(.horizontal) }
                HStack {
                    TextField("Message", text: $text).textInputAutocapitalization(.sentences)
                    Button("Send") { model.send(text, pinExpiry: pin ? UInt32(Date().timeIntervalSince1970) + 900 : nil) }
                        .disabled(text.utf8.isEmpty || text.utf8.count > 280 || state.waitSeconds > 0)
                }.padding()
                Text(state.waitSeconds > 0 ? "Wait \(state.waitSeconds) seconds" : "\(text.utf8.count)/280 UTF-8 bytes · delivery unknown").font(.caption)
            }
        }.onChange(of: state.posted) { _ in text = "" }.onChange(of: state.selected) { _ in text = "" }
    }
    private func reactionRow(id: Data, counts: [UInt16], own: UInt8?) -> some View {
        ScrollView(.horizontal) { HStack {
            ForEach(0..<8, id: \.self) { index in Button {
                model.react(id, code: UInt8(index), remove: own == UInt8(index))
            } label: { Text("\(reactions[index]) \(index < counts.count ? counts[index] : 0)") }
                .accessibilityLabel("Reaction \(reactions[index]); tap again to remove your reaction")
                .disabled(state.directArchived)
            }
        } }
    }
}

private struct JoinView: View {
    @ObservedObject var model: MeshModel
    @Environment(\.dismiss) private var dismiss
    private let words = channelWords()
    @State private var first = "", second = "", third = ""
    private var name: String { [first, second, third].joined(separator: "|") }
    var body: some View {
        NavigationView { Form {
            Text("Anyone with these words can read this channel. It is not encrypted.")
            Picker("First word", selection: $first) { ForEach(words.descriptors, id: \.self) { Text($0).tag($0) } }
            Picker("Second word", selection: $second) { ForEach(words.genres, id: \.self) { Text($0).tag($0) } }
            Picker("Third word", selection: $third) { ForEach(words.locations, id: \.self) { Text($0).tag($0) } }
            Text(verbatim: name.replacingOccurrences(of: "|", with: " "))
            Button("Randomize") { randomize(); UISelectionFeedbackGenerator().selectionChanged() }
            Button("Join channel") { model.join(name); if model.state.selected == name { dismiss() } }
            if let notice = model.state.notice { Text(notice) }
        }.navigationTitle("Join with three words").toolbar { Button("Cancel") { dismiss() } } }
            .onAppear { if first.isEmpty { randomize() } }
    }
    private func randomize() { first = words.descriptors.randomElement() ?? ""; second = words.genres.randomElement() ?? ""; third = words.locations.randomElement() ?? "" }
}

private struct ProposalView: View {
    @ObservedObject var model: MeshModel
    @State private var petname = ""
    var body: some View {
        NavigationView { Form {
            if let channel = model.state.channelProposal {
                Text(verbatim: channel.name.replacingOccurrences(of: "|", with: " "))
                Text("Anyone with these words can read. Nothing is joined until you confirm.")
                Button("Join channel") { model.join(channel.name) }
            } else if let friend = model.state.friendProposal {
                Text(verbatim: "They broadcast as: " + friend.nickname)
                Text(verbatim: friend.fingerprint).font(.system(.body, design: .monospaced))
                Text("Compare this fingerprint with the code on their screen.")
                if let previous = model.state.replacement { Text(verbatim: "Old fingerprint: " + previous.fingerprint); Text("Old history remains under its old identity.") }
                Text(model.state.scanned ? "Confirm you scanned this person's own device in person." : "This link arrived without an in-person scan. It could be someone else's code. Verify it in person before pinning.")
                TextField("Petname — only you see this", text: $petname)
                Text("Show them your code so verification is mutual.")
                Button("Confirm friend pin") { model.confirmFriend(petname) }.disabled(petname.isEmpty)
            } else if let event = model.state.eventProposal {
                Text(verbatim: event.name); Text(verbatim: event.fingerprint).font(.system(.body, design: .monospaced))
                Text("Compare with the official organizer's published fingerprint. This changes which updates receive trusted staff labels.")
                Text("Expires: \(Date(timeIntervalSince1970: TimeInterval(event.expiry)).formatted())")
                Button("Adopt event root") { model.adoptEvent() }
            }
            if let notice = model.state.notice { Text(notice) }
        }.navigationTitle("Review before confirming").toolbar { Button("Cancel") { model.cancelProposal() } } }
    }
}

private struct ConfirmationView: View {
    let title: String, explanation: String, destructive: Bool, action: () -> Void
    @Environment(\.dismiss) private var dismiss
    var body: some View { NavigationView { VStack(alignment: .leading, spacing: 24) {
        Text(title).font(.title2); Text(explanation)
        Button("Confirm", role: destructive ? .destructive : nil) { action(); dismiss() }
        Button("Cancel") { dismiss() }
    }.padding() } }
}

private struct LinkInput: View {
    let result: (String) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var value = ""
    var body: some View { NavigationView { Form {
        Text("Paste a public channel, friend or event link. You will review it before anything changes.")
        TextField("Shared link", text: $value).textInputAutocapitalization(.never).disableAutocorrection(true)
        Button("Review link") { let uri = value; value = ""; dismiss(); result(uri) }.disabled(value.isEmpty || value.utf8.count > 2048)
    }.navigationTitle("Open shared link").toolbar { Button("Cancel") { dismiss() } } } }
}

private struct EventSettings: View {
    @ObservedObject var model: MeshModel
    @Environment(\.dismiss) private var dismiss
    @Environment(\.scenePhase) private var phase
    @State private var staffInput = "", scanStaff = false
    @State private var confirmation: ConfirmAction?
    var body: some View {
        NavigationView { Form {
            Section("Adopted event roots") {
                ForEach(model.state.events, id: \.event.key) { card in
                    Text(verbatim: card.event.name); Text(verbatim: card.event.fingerprint).font(.caption)
                    Text(card.active ? "Active adopted root" : "Expired or unavailable root")
                    Button("Remove event root", role: .destructive) { confirmation = ConfirmAction(title: "Remove event root?", explanation: "Its updates lose trusted staff labels. Re-adoption requires your confirmation.") { model.removeEvent(card.event.key) } }
                }
                Text("Scan or paste a public event code from Channels to review an adoption. Heard event names never adopt a root.")
                ForEach(model.state.discoveries, id: \.self) { Text(verbatim: "Heard, unverified: " + $0) }
            }
            Section("Staff provisioning · secret") {
                Text("Import only your own credential from the organizer. It is encrypted for this identity, expires as signed, and cannot sign while the app is in the background.")
                SecureField("Staff provisioning code", text: $staffInput).textInputAutocapitalization(.never).disableAutocorrection(true)
                Button("Review staff credential") { model.staffInput(staffInput); staffInput = "" }
                Button("Scan staff provisioning QR") { scanStaff = true }
                if let proposal = model.state.staffProposal {
                    Text(verbatim: proposal.label); Text("Expires: \(Date(timeIntervalSince1970: TimeInterval(proposal.notAfter)).formatted())")
                    Button("Confirm staff import") { model.confirmStaff() }
                    Button("Cancel import") { model.cancelProposal() }
                }
                if model.state.staffPresent {
                    Text(model.state.staff?.label ?? "Stored staff credential · signing currently unavailable")
                    Button("Forget staff key", role: .destructive) { confirmation = ConfirmAction(title: "Forget staff key?", explanation: "You must be provisioned again to post trusted updates.") { model.forgetStaff() } }
                }
            }
            if let notice = model.state.notice { Text(notice) }
        }.navigationTitle("Event trust and staff").toolbar { Button("Done") { dismiss() } }
            .sheet(isPresented: $scanStaff) { NavigationView { CodeScanner { value in scanStaff = false; model.staffInput(value) }.toolbar { Button("Cancel") { scanStaff = false } } } }
            .sheet(item: $confirmation) { item in ConfirmationView(title: item.title, explanation: item.explanation, destructive: true, action: item.action) }
        }.onChange(of: phase) { if $0 != .active { staffInput = ""; scanStaff = false; model.cancelProposal() } }
            .onDisappear { staffInput = ""; model.cancelProposal() }
    }
}

private struct SettingsView: View {
    @ObservedObject var model: MeshModel
    @Environment(\.dismiss) private var dismiss
    @State private var nickname = "", avatar: UInt8 = 1, theme = "afterhours", color: UInt32?
    @State private var contribution = false, reset = false
    var body: some View {
        NavigationView { Form {
            Section("Make it yours") {
                TextField("Nickname", text: $nickname).disableAutocorrection(true)
                Picker("Animal avatar", selection: $avatar) { ForEach(0..<8, id: \.self) { Text(animals[$0]).tag(UInt8($0 + 1)) } }
            }
            if let store = model.store { SupporterSettings(store: store, theme: $theme, nicknameRGB: $color) }
            Section("Nearby connection power") {
                Picker("Power preference", selection: Binding(get: { model.state.power }, set: model.power)) {
                    Text("Auto").tag(TransportPowerSetting.auto); Text("Normal").tag(TransportPowerSetting.normal); Text("Saver").tag(TransportPowerSetting.saver)
                }
                Button("Beacon Mode · unavailable on iOS") {}.disabled(true)
                Text("iOS cannot provide the sustained background relay required for Beacon Mode. Keep the app open to participate; use an Android phone for a powered beacon.")
                Button("Contribution & power") { contribution = true }
                Button("Stop nearby connection") { model.stop() }
            }
            Section("Identity and data") {
                Button("Reset identity and local data", role: .destructive) { reset = true }
                Text("Protected data stays local. Reset removes history, pins, event trust and staff keys. Friends must verify your new identity.")
            }
            Section("Licenses") { Text("MeshChat uses Rust, UniFFI and SQLCipher. Dependency notices and licenses are provided with the source distribution.") }
            if let notice = model.state.notice { Text(notice) }
        }.navigationTitle("Settings").toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) { Button("Save") { model.updateProfile(nickname: nickname, avatar: avatar, theme: theme, color: color); if model.state.nickname == nickname { dismiss() } } }
        }
        .sheet(isPresented: $contribution) { NavigationView { if let stats = model.state.contribution { ContributionPanel(stats: stats, reset: model.resetContribution).toolbar { Button("Done") { contribution = false } } } } }
        .sheet(isPresented: $reset) { ConfirmationView(title: "Reset identity and local data?", explanation: "This permanently removes local history, friend pins, event trust and staff credentials. Friends must verify your new code. A lost key cannot recover old data.", destructive: true) { model.resetIdentity(); dismiss() } }
        }.onAppear { nickname = model.state.nickname; avatar = model.state.avatar; theme = model.state.theme; color = model.state.nicknameRGB }
    }
}
