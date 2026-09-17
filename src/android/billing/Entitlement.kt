package org.meshchat.billing

/** Store results are local payment evidence only. Never feed this to trust or
 * transport policy. Unknown/pending results cannot create a paid entitlement. */
enum class StoreResult { OWNED, NOT_OWNED, PENDING, UNAVAILABLE, DISABLED }

object EntitlementPolicy {
    const val FREE_PRIVATE_SLOTS=5
    const val SUPPORTER_PRIVATE_SLOTS=30
    fun active(previous:Boolean,result:StoreResult):Boolean=when(result) {
        StoreResult.OWNED -> true
        StoreResult.NOT_OWNED -> false
        else -> previous
    }
    fun canAddPrivate(active:Boolean,privateCount:Int)=privateCount <
        if(active)SUPPORTER_PRIVATE_SLOTS else FREE_PRIVATE_SLOTS
    // Downgrade keeps existing subscriptions. The cap applies only to new slots.
}
