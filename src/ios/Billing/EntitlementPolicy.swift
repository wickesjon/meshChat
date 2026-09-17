enum StoreEvidence: Sendable {
    case owned, notOwned, pending, unavailable, disabled
}

enum EntitlementPolicy {
    static func active(previous: Bool, result: StoreEvidence) -> Bool {
        switch result {
        case .owned: return true
        case .notOwned: return false
        case .pending, .unavailable, .disabled: return previous
        }
    }
    static func canAddPrivate(active: Bool, privateCount: Int) -> Bool {
        privateCount < (active ? 30 : 5)
    }
}

/// The caller supplies protected persistence. Failed saves cannot acknowledge a
/// purchase or replace the last successfully cached entitlement.
@MainActor
final class EntitlementCache {
    private(set) var active: Bool
    private let save: (Bool) throws -> Void
    init(cached: Bool, save: @escaping (Bool) throws -> Void) { active = cached; self.save = save }
    func update(_ evidence: StoreEvidence) throws {
        let next = EntitlementPolicy.active(previous: active, result: evidence)
        try save(next)
        active = next
    }
}
