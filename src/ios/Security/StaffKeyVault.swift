import Foundation

/// Main-actor serialization includes the final native submission. No await or
/// retained private session can cross the foreground/lock/reset boundary.
@MainActor final class StaffKeyVault: IdentityResetStore {
    private let files: IdentityStorage
    private let protection: IdentityProtection
    private(set) var foreground = true
    init(files: IdentityStorage, protection: IdentityProtection) {
        self.files = files; self.protection = protection
    }
    static func native() throws -> StaffKeyVault {
        StaffKeyVault(files: try AppleIdentityStorage(name: "meshchat-staff-v1"),
                      protection: AppleIdentityProtection(tag: "org.meshchat.staff.wrapping.v1", payloadRange: 1...512))
    }
    func setForeground(_ value: Bool) { foreground = value }
    func present() throws -> Bool { try files.exists(.envelope) || files.exists(.journal) || protection.exists() }
    private func header(_ generation: Data) throws -> Data {
        guard generation.count == 16 else { throw IdentityFailure.invalidInput }
        return Data([1]) + generation
    }
    private func requireForeground() throws {
        try protection.requireUnlocked()
        guard foreground else { throw OrganizerError.Authority }
    }
    func importConfirmed(generation: Data, candidate: inout Data, validate: (String) throws -> Void) throws {
        defer { candidate.resetBytes(in: 0..<candidate.count); candidate.removeAll() }
        try requireForeground()
        guard (1...512).contains(candidate.count), let uri = String(data: candidate, encoding: .utf8) else { throw IdentityFailure.invalidInput }
        let aad = try header(generation)
        guard !files.exists(.journal) else { throw IdentityFailure.recoveryRequired }
        try validate(uri)
        let existing = files.exists(.envelope)
        if existing { guard try protection.exists() else { throw IdentityFailure.invalidated } }
        else {
            guard try !protection.exists() else { throw IdentityFailure.recoveryRequired }
            try files.write(.journal, Data([1])); try protection.create()
        }
        try files.write(.envelope, aad + protection.seal(candidate, aad: aad))
        if !existing { try files.delete(.journal) }
    }
    func access<T>(generation: Data, _ work: (String) throws -> T) throws -> T {
        try requireForeground()
        guard !files.exists(.journal) else { throw IdentityFailure.recoveryRequired }
        guard files.exists(.envelope) else { throw IdentityFailure.missing }
        guard try protection.exists() else { throw IdentityFailure.invalidated }
        let aad = try header(generation), encoded = try files.read(.envelope)
        guard (18...1024).contains(encoded.count), encoded.prefix(17) == aad else { throw IdentityFailure.invalidInput }
        var plain = try protection.open(Data(encoded.dropFirst(17)), aad: aad)
        defer { plain.resetBytes(in: 0..<plain.count) }
        guard (1...512).contains(plain.count), let uri = String(data: plain, encoding: .utf8) else { throw IdentityFailure.invalidInput }
        let result = try work(uri); try requireForeground(); return result
    }
    func forget() throws {
        try protection.requireUnlocked()
        try files.write(.journal, Data([2])); try protection.delete()
        try files.delete(.envelope); try files.delete(.journal)
    }
    func clearIdentityState() throws { try IdentityProvider.requireResetInProgress(); try forget() }
}

@MainActor final class FeatureResetStore: IdentityResetStore {
    private let storage: StorageVault
    private let staff: StaffKeyVault
    init(storage: StorageVault, staff: StaffKeyVault) { self.storage = storage; self.staff = staff }
    func clearIdentityState() throws { try staff.clearIdentityState(); try storage.clearIdentityState() }
}
