//! Deterministic lifecycle/intake mutation smoke test, not a full fuzz campaign.
use meshchat_core::{
    LinkHandle,
    ingress::{Ingress, Intake},
};

#[test]
fn hostile_sizes_staging_tokens_and_reconnects_remain_bounded() {
    let mut ingress = Ingress::new(9, 8, 0).unwrap();
    let mut handle = LinkHandle {
        instance_nonce: 9,
        generation: 1,
    };
    ingress.register(&handle, 512, 0).unwrap();
    let mut random = 43u64;
    let mut tokens: Vec<Intake> = Vec::new();
    for step in 0..12000u64 {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let now = step * 50;
        if step > 0 && step % 23 == 0 {
            ingress.disconnect(&handle, now).unwrap();
            handle.generation += 1;
            ingress.register(&handle, 512, now).unwrap();
        }
        let mut raw = [0; 513];
        for (i, byte) in raw.iter_mut().enumerate() {
            *byte = random.rotate_left(i as u32) as u8;
        }
        let len = (random as usize) % 514;
        let reported = if step % 97 == 0 { u64::MAX } else { len as u64 };
        if let Ok(token) = ingress
            .enqueue(&handle, reported, &raw[..len], now)
            .unwrap()
        {
            if tokens.len() < 16 {
                tokens.push(token);
            }
        }
        if step % 3 == 0 {
            if let Some(token) = tokens.pop() {
                let _ = ingress.process(token, now, &mut [0; 1035]);
                assert!(ingress.process(token, now, &mut [0; 1035]).is_err());
            }
        }
        let r = ingress.reservations();
        assert!(r.staging <= 16 && r.pending <= 32 && r.rejected <= 512 && r.accepted <= 4096);
        assert_eq!(ingress.counters().reserved_work_units, 0);
    }
}
