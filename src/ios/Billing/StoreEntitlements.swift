import Combine
import StoreKit

/// MC-035 supplies the protected settings cache and settings UI. No product ID
/// is configured by this ticket; an unconfigured adapter never contacts StoreKit.
@MainActor
final class StoreEntitlements: ObservableObject {
    @Published private(set) var active: Bool
    @Published private(set) var status = "Purchases are not configured in this build."
    @Published private(set) var price: String?
    private let productID: String?
    private let cache: EntitlementCache
    private var updates: Task<Void, Never>?
    private var busy = false
    private var refreshRevision: UInt = 0

    init(productID: String? = nil, cached: Bool, save: @escaping (Bool) throws -> Void) {
        self.productID = productID?.isEmpty == false ? productID : nil
        self.active = cached
        self.cache = EntitlementCache(cached: cached, save: save)
        guard self.productID != nil else { return }
        updates = Task { [weak self] in
            for await result in Transaction.updates {
                guard let self else { return }
                if case .verified(let transaction) = result, transaction.productID == self.productID {
                    if await self.refresh() { await transaction.finish() }
                }
            }
        }
    }
    deinit { updates?.cancel() }

    @discardableResult
    func refresh() async -> Bool {
        guard let productID else { return apply(.disabled) }
        refreshRevision &+= 1
        let revision = refreshRevision
        var owned = false
        var unknown = false
        for await result in Transaction.currentEntitlements {
            switch result {
            case .verified(let transaction):
                if transaction.productID == productID,
                   transaction.productType == .nonConsumable,
                   transaction.revocationDate == nil { owned = true }
            case .unverified(let transaction, _):
                if transaction.productID == productID { unknown = true }
            }
        }
        guard revision == refreshRevision else { return false }
        return apply(owned ? .owned : (unknown ? .unavailable : .notOwned))
    }

    func loadPrice() async {
        guard let productID else { return }
        do {
            let product = try await Product.products(for: [productID]).first { $0.type == .nonConsumable }
            price = product?.displayPrice
            if product == nil { status = "Supporter is unavailable from the store." }
        } catch { price = nil; status = "Store unavailable. Saved Supporter access is unchanged." }
    }

    func purchase() async {
        guard let productID, !busy else { return }
        busy = true
        defer { busy = false }
        do {
            guard let product = try await Product.products(for: [productID]).first,
                  product.type == .nonConsumable else { _ = apply(.unavailable); return }
            switch try await product.purchase() {
            case .success(let result):
                if case .verified(let transaction) = result,
                   transaction.productID == productID,
                   transaction.productType == .nonConsumable,
                   transaction.revocationDate == nil {
                    refreshRevision &+= 1
                    if apply(.owned) { await transaction.finish() }
                } else { _ = apply(.unavailable) }
            case .pending: _ = apply(.pending)
            case .userCancelled: status = "Purchase cancelled. Saved access is unchanged."
            @unknown default: _ = apply(.unavailable)
            }
        } catch { _ = apply(.unavailable) }
    }

    func restore() async {
        guard productID != nil, !busy else { return }
        busy = true
        defer { busy = false }
        do { try await AppStore.sync(); await refresh() }
        catch { _ = apply(.unavailable) }
    }

    private func apply(_ evidence: StoreEvidence) -> Bool {
        do { try cache.update(evidence) }
        catch { status = "Unable to save Supporter access. Try Restore when storage reopens."; return false }
        active = cache.active
        switch evidence {
        case .owned: status = "Supporter restored. Ready for offline use."
        case .notOwned: status = "No current Supporter purchase. Existing channels stay available."
        case .pending: status = "Purchase pending. Saved access is unchanged."
        case .unavailable: status = "Store unavailable. Saved access is unchanged."
        case .disabled: status = "Purchases are not configured in this build."
        }
        return true
    }
}
