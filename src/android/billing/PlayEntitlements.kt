package org.meshchat.billing

import android.app.Activity
import android.content.Context
import android.os.Handler
import android.os.Looper
import com.android.billingclient.api.*

/** Empty release configuration deliberately disables all store contact. Activate
 * only after MC-039 checks the real non-consumable product and license key. */
data class PlayConfiguration(val productId:String="",val publicKey:String="")

class PlayEntitlements(context:Context,configuration:PlayConfiguration=PlayConfiguration(),
    private val report:(StoreResult,String,(Boolean)->Unit)->Unit,
    private val priceChanged:(String?)->Unit):AutoCloseable {
    private val main=Handler(Looper.getMainLooper())
    private val packageName=context.packageName
    private val productId=configuration.productId
    private val verifier=ReceiptVerifier(configuration.publicKey)
    val configured=productId.isNotBlank() && verifier.configured
    private var closed=false
    private var connecting=false
    private var querying=false
    private var refreshAgain=false
    private var buying=false
    private val client=if(configured)BillingClient.newBuilder(context)
        .setListener { result,_ -> onMain {
            buying=false
            if(result.responseCode==BillingClient.BillingResponseCode.OK || result.responseCode==BillingClient.BillingResponseCode.ITEM_ALREADY_OWNED)refresh()
            else emit(StoreResult.UNAVAILABLE,"Purchase not completed. Saved access is unchanged.")
        } }
        .enablePendingPurchases(PendingPurchasesParams.newBuilder().enableOneTimeProducts().build())
        .enableAutoServiceReconnection().build() else null

    private fun onMain(block:()->Unit) { main.post { if(!closed)block() } }
    private fun emit(result:StoreResult,message:String,afterSaved:()->Unit={}) {
        report(result,message) { saved -> onMain { if(saved)afterSaved() } }
    }
    fun refresh() {
        if(closed)return
        val store=client ?: run { emit(StoreResult.DISABLED,"Purchases are not configured in this build.");return }
        if(querying) { refreshAgain=true;return }
        if(connecting)return
        if(!store.isReady) {
            connecting=true
            store.startConnection(object:BillingClientStateListener {
                override fun onBillingSetupFinished(result:BillingResult)=onMain {
                    connecting=false
                    if(result.responseCode==BillingClient.BillingResponseCode.OK) { refresh();loadPrice() }
                    else emit(StoreResult.UNAVAILABLE,"Store unavailable. Saved access is unchanged.")
                }
                override fun onBillingServiceDisconnected()=onMain { connecting=false }
            })
            return
        }
        querying=true
        store.queryPurchasesAsync(QueryPurchasesParams.newBuilder().setProductType(BillingClient.ProductType.INAPP).build()) { result,purchases -> onMain {
            querying=false
            if(refreshAgain) { refreshAgain=false;refresh();return@onMain }
            if(result.responseCode!=BillingClient.BillingResponseCode.OK) {
                emit(StoreResult.UNAVAILABLE,"Store unavailable. Saved access is unchanged.");return@onMain
            }
            val matching=purchases.filter { productId in it.products }
            val owned=matching.filter { it.purchaseState==Purchase.PurchaseState.PURCHASED &&
                it.packageName==packageName && verifier.valid(it.originalJson,it.signature) }
            when {
                owned.isNotEmpty() -> emit(StoreResult.OWNED,"Supporter restored. Ready for offline use.") {
                    owned.filterNot { it.isAcknowledged }.forEach { purchase ->
                        store.acknowledgePurchase(AcknowledgePurchaseParams.newBuilder().setPurchaseToken(purchase.purchaseToken).build()) { acknowledgement ->
                            if(acknowledgement.responseCode!=BillingClient.BillingResponseCode.OK)onMain {
                                emit(StoreResult.UNAVAILABLE,"Purchase saved; store confirmation will retry when reopened.")
                            }
                        }
                    }
                }
                matching.any { it.purchaseState==Purchase.PurchaseState.PENDING } -> emit(StoreResult.PENDING,"Purchase pending. Saved access is unchanged.")
                matching.isNotEmpty() -> emit(StoreResult.UNAVAILABLE,"Unable to verify purchase. Saved access is unchanged.")
                else -> emit(StoreResult.NOT_OWNED,"No current Supporter purchase. Existing channels stay available.")
            }
        } }
    }
    private fun product(failed:()->Unit={},ready:(ProductDetails)->Unit) {
        val store=client ?: return
        val params=QueryProductDetailsParams.newBuilder().setProductList(listOf(
            QueryProductDetailsParams.Product.newBuilder().setProductId(productId).setProductType(BillingClient.ProductType.INAPP).build())).build()
        store.queryProductDetailsAsync(params) { result,details -> onMain {
            val item=details.productDetailsList.singleOrNull()
            // Exactly one permanent purchase offer is required by release setup.
            val offer=item?.oneTimePurchaseOfferDetailsList?.singleOrNull()
            if(result.responseCode==BillingClient.BillingResponseCode.OK && item?.productId==productId && offer!=null && offer.rentalDetails==null) {
                priceChanged(offer.formattedPrice);ready(item)
            } else { failed();priceChanged(null);emit(StoreResult.UNAVAILABLE,"Supporter is unavailable from the store.") }
        } }
    }
    fun loadPrice() { if(!closed && client?.isReady==true)product { } }
    fun purchase(activity:Activity) {
        if(closed || buying)return
        val store=client ?: return
        if(!store.isReady) { refresh();return }
        buying=true
        // Requery instead of persisting ProductDetails/offer tokens across sessions.
        product(failed={buying=false}) { details ->
            val offer=details.oneTimePurchaseOfferDetailsList!!.single()
            val params=BillingFlowParams.newBuilder().setProductDetailsParamsList(listOf(
                BillingFlowParams.ProductDetailsParams.newBuilder().setProductDetails(details)
                    .apply { offer.offerToken?.let { setOfferToken(it) } }.build())).build()
            val result=store.launchBillingFlow(activity,params)
            if(result.responseCode!=BillingClient.BillingResponseCode.OK) {
                buying=false;emit(StoreResult.UNAVAILABLE,"Unable to open the store. Saved access is unchanged.")
            }
        }
    }
    override fun close() { closed=true;client?.endConnection();main.removeCallbacksAndMessages(null) }
}
