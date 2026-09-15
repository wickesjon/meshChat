// Session component fixtures still enter through real outer ingress. Serving
// state tests use a fresh pre-admitted ingress instance per supplied request to
// isolate cursor/ID policy; exchange.rs exercises persistent ingress budgets.
use meshchat_core::{
    LinkHandle,
    framing::Encoder,
    ingress::{Ingress, SyncAdmission},
    sync::{Cache, session::*},
};
pub fn admission<'a>(
    i: &mut Ingress,
    h: &LinkHandle,
    raw: &[u8],
    transport: bool,
    now: u64,
    out: &'a mut [u8],
) -> SyncAdmission<'a> {
    let enc = if transport {
        Encoder::transport(1, raw, 512, 1)
    } else {
        Encoder::logical(raw, 512, 1)
    }
    .unwrap();
    let mut frame = [0; 512];
    for f in 0..enc.frame_count() - 1 {
        let n = enc.frame(f, &mut frame).unwrap();
        assert!(
            i.receive_deferred_sync(h, n as u64, &frame[..n], now, out)
                .unwrap()
                .sync
                .is_none()
        );
    }
    let n = enc.frame(enc.frame_count() - 1, &mut frame).unwrap();
    i.receive_deferred_sync(h, n as u64, &frame[..n], now, out)
        .unwrap()
        .sync
        .unwrap()
}
pub trait RawSessions {
    fn accept_request(
        &mut self,
        h: &LinkHandle,
        raw: &[u8],
        cache: &mut Cache,
        now: u64,
        emit: &mut impl FnMut(Event),
    ) -> Result<RequestOutcome, Error>;
    fn receive(
        &mut self,
        h: &LinkHandle,
        raw: &[u8],
        ingress: &mut Ingress,
        now: u64,
        sink: &mut Sink<'_>,
    ) -> Result<Received, Error>;
}
impl RawSessions for Sessions {
    fn accept_request(
        &mut self,
        h: &LinkHandle,
        raw: &[u8],
        cache: &mut Cache,
        now: u64,
        emit: &mut impl FnMut(Event),
    ) -> Result<RequestOutcome, Error> {
        let mut i = Ingress::new(h.instance_nonce, 8, now).unwrap();
        i.register(h, 512, now).unwrap();
        let mut out = [0; 1035];
        let token = admission(&mut i, h, raw, false, now, &mut out);
        self.accept_admitted_request(h, token, cache, now, emit)
    }
    fn receive(
        &mut self,
        h: &LinkHandle,
        raw: &[u8],
        ingress: &mut Ingress,
        now: u64,
        sink: &mut Sink<'_>,
    ) -> Result<Received, Error> {
        let mut out = [0; 1035];
        let token = admission(ingress, h, raw, true, now, &mut out);
        self.receive_admitted(h, token, ingress, now, sink)
    }
}
