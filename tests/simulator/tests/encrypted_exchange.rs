// The deterministic topology uses production HELLO admission, framing, opaque
// cache/SYNC and HPKE/persistence at endpoints. It makes no radio/device claim.
#[path = "../../integration/dm/dm.rs"]
mod encrypted_exchange;
