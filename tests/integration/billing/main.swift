import Foundation

for result in [StoreEvidence.pending, .unavailable, .disabled] {
    precondition(!EntitlementPolicy.active(previous: false, result: result))
    precondition(EntitlementPolicy.active(previous: true, result: result))
}
precondition(EntitlementPolicy.active(previous: false, result: .owned))
precondition(!EntitlementPolicy.active(previous: true, result: .notOwned))
precondition(EntitlementPolicy.canAddPrivate(active: false, privateCount: 4))
precondition(!EntitlementPolicy.canAddPrivate(active: false, privateCount: 5))
precondition(EntitlementPolicy.canAddPrivate(active: true, privateCount: 29))
precondition(!EntitlementPolicy.canAddPrivate(active: true, privateCount: 30))
precondition(!EntitlementPolicy.canAddPrivate(active: false, privateCount: 30))
print("MC-032 entitlement transition and slot checks passed")

@MainActor
func cacheChecks() async throws {
    enum SaveError: Error { case unavailable }
    var disk = false
    let cache = EntitlementCache(cached: disk) { disk = $0 }
    try cache.update(.pending); precondition(!disk)
    try cache.update(.owned); precondition(disk && cache.active)
    let reopened = EntitlementCache(cached: disk) { disk = $0 }
    try reopened.update(.unavailable); precondition(reopened.active)
    try reopened.update(.notOwned); precondition(!disk && !reopened.active)
    let failure = EntitlementCache(cached: false) { _ in throw SaveError.unavailable }
    do { try failure.update(.owned); preconditionFailure("Failed save acknowledged") } catch SaveError.unavailable { }
    precondition(!failure.active)
    let disabled = StoreEntitlements(cached: true) { disk = $0 }
    let refreshed = await disabled.refresh()
    precondition(refreshed)
    await disabled.purchase(); await disabled.restore(); await disabled.loadPrice()
    precondition(disabled.active && disabled.price == nil && disk)
}
try await cacheChecks()
for theme in ThemeTokens.all {
    for color in [theme.text, theme.secondary, theme.accent, theme.error] {
        precondition(ThemeTokens.contrast(color, theme.surface) >= 4.5)
        precondition(ThemeTokens.contrast(color, theme.background) >= 4.5)
    }
    precondition(ThemeTokens.contrast(theme.onAccent, theme.accent) >= 4.5)
    for color in [UInt32(0), 0xffffff, 0x777777, 0x00ff00, 0xff0000, 0x0000ff] {
        precondition(ThemeTokens.contrast(theme.readableNickname(color), theme.surface) >= 4.5)
    }
}
precondition(ThemeTokens.selected("violet", active: false).id == "afterhours")
precondition(ThemeTokens.selected("violet", active: true).id == "violet")
print("MC-032 native cache, disabled StoreKit adapter and theme checks passed")
