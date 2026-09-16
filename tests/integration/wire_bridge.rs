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

#[uniffi::export]
pub fn check_wire_vectors(
    friend_vectors: HashMap<String, Vec<u8>>,
    dm_vectors: HashMap<String, Vec<u8>>,
    organizer_vectors: HashMap<String, Vec<u8>>,
) -> Result<Vec<String>, FixtureError> {
    std::panic::catch_unwind(|| {
        assert_eq!(friend_vectors.len(), 8);
        assert_eq!(dm_vectors.len(), 17);
        assert_eq!(organizer_vectors.len(), 8);
        friends::check_binding_vectors(friend_vectors);
        dm::check_binding_vectors(dm_vectors);
        organizer::check_binding_vectors(organizer_vectors);
        vec!["friend:8".into(), "dm:17".into(), "organizer:8".into()]
    })
    .map_err(|_| FixtureError::Mismatch)
}
