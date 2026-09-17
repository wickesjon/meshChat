import Foundation
func statsTrace() throws {
    let db=try TransportSql();defer {db.finish()}
    let identity=try IdentityKeySession.importUnlocked(material:Data(repeating:44,count:64),generation:Data(repeating:44,count:16))
    let own=try identity.publicIdentity()
    let store=try EncryptedStore.open(db:db,generation:own.generation,create:true,now:200000)
    let core=try NativeTransport.newIos(store:store,identity:own,instanceNonce:44,monotonicMs:0)
    let initial=try core.contributionStats(now:0);precondition(initial.batteryPercent==nil && initial.completedFrames==0)
    _=try core.updatePower(setting:.saver,batteryPercent:32,charging:false,visiblePeers:0,now:1)
    var s=try core.contributionStats(now:2);precondition(s.batteryPercent==32 && s.powerMode=="Saver")
    s.receivedFrames=999;s.completedFrames=888;s.relayedChatCopies=777;s.powerMode="PRIVATE_SENTINEL"
    let text=contributionShareText(stats:s,received:false,sent:false,relayed:true)
    precondition(text.contains("777") && !text.contains("999") && !text.contains("888") && !text.contains("PRIVATE_SENTINEL") && text.contains("Delivery unknown"))
    try core.resetContributionStats(now:3);let reset=try core.contributionStats(now:3);precondition(reset.elapsedMs==0)
    let stale=try core.contributionStats(now:60002);precondition(stale.batteryPercent==nil)
    print("MC-042 Swift production stats, power freshness, reset and selected aggregate export PASS")
}
