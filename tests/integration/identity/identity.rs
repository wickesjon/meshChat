use ed25519_dalek::{Signature, VerifyingKey};
use meshchat_core::identity::*;
use sha2::{Digest, Sha256};
#[test]
fn public_identity_signing_agreement_and_invalidation() {
    let a = IdentityKeySession::import_unlocked((0..64).collect(), vec![1; 16]).unwrap();
    let b = IdentityKeySession::import_unlocked((64..128).collect(), vec![2; 16]).unwrap();
    let public = a.public_identity().unwrap();
    let peer = b.public_identity().unwrap();
    assert_eq!(public.sender_id, Sha256::digest(&public.signing_key)[..8]);
    assert_eq!(
        public.agreement_hint,
        Sha256::digest(&public.agreement_key)[..8]
    );
    let message = b"synthetic identity contract".to_vec();
    let sig = a.sign(message.clone()).unwrap();
    VerifyingKey::from_bytes(&public.signing_key.try_into().unwrap())
        .unwrap()
        .verify_strict(&message, &Signature::from_slice(&sig).unwrap())
        .unwrap();
    assert_eq!(
        a.agree(peer.agreement_key).unwrap(),
        b.agree(public.agreement_key).unwrap()
    );
    assert_eq!(a.sign(vec![0; 2049]), Err(IdentityError::InvalidInput));
    assert_eq!(a.agree(vec![0; 32]), Err(IdentityError::InvalidPeer));
    assert_eq!(a.agree(vec![255; 32]), Err(IdentityError::InvalidPeer));
    a.invalidate().unwrap();
    a.invalidate().unwrap();
    assert_eq!(a.public_identity(), Err(IdentityError::Unavailable));
    assert_eq!(a.sign(message), Err(IdentityError::Unavailable));
    assert_eq!(
        a.agree(b.public_identity().unwrap().agreement_key),
        Err(IdentityError::Unavailable)
    );
}
#[test]
fn malformed_imports_refuse_without_material_export() {
    for len in [0, 32, 63, 65, 4096] {
        assert!(IdentityKeySession::import_unlocked(vec![1; len], vec![1; 16]).is_err());
    }
    assert!(IdentityKeySession::import_unlocked(vec![1; 64], vec![0; 16]).is_err());
    assert!(IdentityKeySession::import_unlocked(vec![1; 64], vec![1; 15]).is_err());
}
