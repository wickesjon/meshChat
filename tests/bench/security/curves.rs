use meshchat_core::security_probe::*;

fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn rfc8032_empty_message_and_negative_signatures() {
    // RFC 8032 section 7.1, TEST 1. Published dummy key material only.
    let seed = hex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60");
    let public = hex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a");
    let signature = hex(concat!(
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555f",
        "b8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
    ));
    assert_eq!(probe_signing_public(seed.clone()).unwrap(), public);
    assert_eq!(probe_sign(seed, vec![]).unwrap(), signature);
    assert!(probe_verify(public.clone(), vec![], signature.clone()));
    assert!(!probe_verify(public.clone(), vec![1], signature.clone()));
    let mut corrupt = signature;
    corrupt[0] ^= 1;
    assert!(!probe_verify(public.clone(), vec![], corrupt));
    assert!(!probe_verify(public, vec![], vec![0; 63]));
}

#[test]
fn rfc7748_agreement_and_noncontributory_peer() {
    // RFC 7748 section 6.1.
    let alice = hex("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
    let alice_public = hex("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a");
    let bob = hex("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");
    let bob_public = hex("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f");
    let shared = hex("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");
    assert_eq!(probe_agreement_public(alice.clone()).unwrap(), alice_public);
    assert!(probe_agreement_matches(alice.clone(), bob_public.clone(), shared.clone()).unwrap());
    assert!(probe_agreement_matches(bob, alice_public, shared).unwrap());
    assert!(!probe_agreement_matches(alice.clone(), bob_public, vec![0; 32]).unwrap());
    assert!(matches!(
        probe_agreement_matches(alice, vec![0; 32], vec![0; 32]),
        Err(ProbeError::InvalidPeer)
    ));
}

#[test]
fn bounded_inputs_fail_closed() {
    assert!(probe_sign(vec![0; 31], vec![]).is_err());
    assert!(probe_sign(vec![0; 32], vec![0; 1025]).is_err());
    assert!(probe_agreement_matches(vec![0; 32], vec![1; 31], vec![0; 32]).is_err());
    assert!(probe_agreement_matches(vec![0; 32], vec![1; 32], vec![0; 31]).is_err());
}
