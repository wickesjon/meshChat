//! Test-only FFI facade. Never linked into a shipping application.
//! Foreign callers supply the public reference bytes; the existing admitted
//! production verifier harnesses check them. SQLite here is a test double.
#![allow(dead_code, unused_imports)] // Shared harness helpers also serve Rust-only tests.
#![allow(clippy::duplicate_mod)] // Each existing harness owns a namespaced database helper.

#[path = "dm/dm.rs"]
mod dm;
#[path = "friends/friends.rs"]
mod friends;
#[path = "organizer/organizer.rs"]
mod organizer;
use std::collections::HashMap;

uniffi::setup_scaffolding!();

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FixtureError {
    #[error("reference fixture check failed")]
    Mismatch,
}

fn require_names(vectors: &HashMap<String, Vec<u8>>, names: &[&str]) {
    assert_eq!(vectors.len(), names.len());
    assert!(names.iter().all(|name| vectors.contains_key(*name)));
}

#[uniffi::export]
pub fn check_wire_vectors(
    friend_vectors: HashMap<String, Vec<u8>>,
    dm_vectors: HashMap<String, Vec<u8>>,
    organizer_vectors: HashMap<String, Vec<u8>>,
) -> Result<Vec<String>, FixtureError> {
    std::panic::catch_unwind(|| {
        require_names(
            &friend_vectors,
            &[
                "peer_public",
                "own_public",
                "peer_chat",
                "own_chat",
                "local_hello",
                "remote_hello",
                "peer_proof",
                "own_proof",
            ],
        );
        require_names(
            &dm_vectors,
            &[
                "chat_2_1",
                "chat_2_58",
                "chat_2_59",
                "chat_2_138",
                "chat_2_139",
                "chat_2_280",
                "chat_3_1",
                "chat_3_58",
                "chat_3_59",
                "chat_3_138",
                "chat_3_139",
                "chat_3_280",
                "reaction",
                "bad_padding",
                "nonminimal",
                "bad_utf8",
                "sender_forgery",
            ],
        );
        require_names(
            &organizer_vectors,
            &[
                "root_bundle",
                "credential_a",
                "credential_b",
                "chat_a",
                "chat_b",
                "omitted_a",
                "pinned_a",
                "wrong_domain",
            ],
        );
        friends::check_binding_vectors(friend_vectors);
        dm::check_binding_vectors(dm_vectors);
        organizer::check_binding_vectors(organizer_vectors);
        vec!["friend:8".into(), "dm:17".into(), "organizer:8".into()]
    })
    .map_err(|_| FixtureError::Mismatch)
}
