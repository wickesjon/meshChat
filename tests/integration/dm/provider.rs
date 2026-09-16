use super::*;
fn hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|x| u8::from_str_radix(std::str::from_utf8(x).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn published_auth_kat_and_one_use_entropy_contract() {
    let v: std::collections::HashMap<_, _> =
        include_str!("../../vectors/crypto/hpke-auth-rfc9180.tsv")
            .lines()
            .map(|l| {
                let (k, v) = l.split_once('\t').unwrap();
                (k, hex(v))
            })
            .collect();
    let mut rng = OneUse {
        bytes: Zeroizing::new(v["ikmE"].as_slice().try_into().unwrap()),
        used: false,
        failed: false,
    };
    let private = <Kem as KemTrait>::PrivateKey::from_bytes(&v["skSm"]).unwrap();
    let (enc, mut ctx) = hpke::setup_sender_with_rng::<Aead, Kdf, Kem>(
        &hpke::OpModeS::Auth((private, key(&v["pkSm"]).unwrap())),
        &key(&v["pkRm"]).unwrap(),
        &v["info"],
        &mut rng,
    )
    .unwrap();
    assert!(rng.used && !rng.failed);
    assert_eq!(enc.to_bytes().as_slice(), v["enc"]);
    assert_eq!(ctx.seal(&v["pt"], &v["aad"]).unwrap(), v["ct"]);
    let mut bytes = [9; 32];
    rng.try_fill_bytes(&mut bytes).unwrap();
    assert!(rng.failed);
    assert_eq!(bytes, [0; 32]);
    for length in [0, 16, 33] {
        let mut rng = OneUse {
            bytes: Zeroizing::new([1; 32]),
            used: false,
            failed: false,
        };
        rng.try_fill_bytes(&mut vec![0; length]).unwrap();
        assert!(rng.failed && !rng.used);
    }
}
#[test]
fn protected_sender_matches_independent_profile_vector() {
    let v: std::collections::HashMap<_, _> = include_str!("../../vectors/crypto/dm-v1.tsv")
        .lines()
        .map(|l| {
            let (k, v) = l.split_once('\t').unwrap();
            (k, hex(v))
        })
        .collect();
    let provider = IdentityKeySession::import_unlocked(vec![2; 64], vec![2; 16]).unwrap();
    let recipient = IdentityKeySession::import_unlocked(vec![3; 64], vec![3; 16]).unwrap();
    let tuple = |p: PublicIdentity| {
        let mut t = [0; 64];
        t[..32].copy_from_slice(&p.signing_key);
        t[32..].copy_from_slice(&p.agreement_key);
        t
    };
    let own = tuple(provider.public_identity().unwrap());
    let peer = tuple(recipient.public_identity().unwrap());
    let mut header = v["chat_2_1"][..26].try_into().unwrap();
    let mut plain = vec![0; 64];
    plain[..4].copy_from_slice(&200000u32.to_be_bytes());
    plain[5] = 1;
    plain[6] = b'a';
    let raw = provider
        .dm_seal_buffered(
            &own,
            &peer,
            &mut header,
            &plain,
            200000 / 3600,
            Zeroizing::new([7; 32]),
        )
        .unwrap();
    assert_eq!(raw, v["chat_2_1"]);
    assert_eq!(
        recipient
            .dm_open(&peer, &own, &raw, 200000 / 3600)
            .unwrap()
            .as_slice(),
        plain
    );
    for x in [[0; 32], [255; 32]] {
        let mut bad = peer;
        bad[32..].copy_from_slice(&x);
        assert!(
            provider
                .dm_seal_buffered(&own, &bad, &mut header, &plain, 55, Zeroizing::new([7; 32]))
                .is_err()
        );
    }
    provider.invalidate().unwrap();
    assert!(
        provider
            .dm_seal(&own, &peer, &mut header, &plain, 55)
            .is_err()
    );
}

#[test]
fn os_entropy_failure_is_propagated_before_setup() {
    let provider = IdentityKeySession::import_unlocked(vec![2; 64], vec![2; 16]).unwrap();
    let result =
        provider.dm_seal_with_entropy(&[0; 64], &[0; 64], &mut [0; 26], &[0; 64], 0, |_| {
            Err(Error::Entropy)
        });
    assert!(matches!(result, Err(Error::Entropy)));
}
