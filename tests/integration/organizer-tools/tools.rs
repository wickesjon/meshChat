use meshchat_organizer::*;
#[path = "../friends/database.rs"]
#[allow(dead_code)]
mod database;
mod rehearsal;
use meshchat_core::{
    identity::IdentityKeySession, security_probe::probe_organizer_import, storage::EncryptedStore,
};

fn mobile_import(event: &str, staff: &str, wall: i64) -> bool {
    let identity = IdentityKeySession::import_unlocked(vec![41; 64], vec![41; 16]).unwrap();
    let own = identity.public_identity().unwrap();
    let store = EncryptedStore::open(
        Box::new(database::Database::new()),
        own.generation.clone(),
        true,
        Some(wall),
    )
    .unwrap();
    probe_organizer_import(store, own, event.into(), staff.into(), wall).unwrap_or(false)
}

#[test]
fn encrypted_root_and_canonical_staff_roundtrip() {
    let created = create_root("Test event", 202000, 201000, 200000).unwrap();
    let root = OpenRoot::open(&created.vault, &created.unlock, 200000).unwrap();
    assert_eq!(root.event().unwrap(), created.event);
    let staff = root.issue("Ops", 199990, 201000, 200000).unwrap();
    validate_staff(&created.event, &staff, 200000).unwrap();
    assert!(mobile_import(&created.event, &staff, 200000));
    assert!(validate_staff(&created.event, &staff, 201001).is_err());
    assert!(!mobile_import(&created.event, &staff, 201001));
    assert!(validate_staff(&created.event, &staff, 199989).is_err());
    assert!(OpenRoot::open(&created.vault, &"00".repeat(32), 200000).is_err());
    assert!(OpenRoot::open(&created.vault, &created.unlock, 202001).is_err());
    let other = create_root("Other", 202000, 201000, 200000).unwrap();
    assert!(validate_staff(&other.event, &staff, 200000).is_err());
    assert!(!mobile_import(&other.event, &staff, 200000));
    let mut damaged = created.vault.clone();
    damaged.replace_range(..2, "ff");
    assert!(OpenRoot::open(&damaged, &created.unlock, 200000).is_err());
    let raw = unhex(&created.vault).unwrap();
    for offset in [
        b"meshchat/offline-root/v1\0".len(),
        raw.len() - 17,
        raw.len() - 1,
    ] {
        let mut modified = raw.to_vec();
        modified[offset] ^= 1;
        assert!(OpenRoot::open(&hex(&modified), &created.unlock, 200000).is_err());
    }
    assert!(OpenRoot::open(&hex(&raw[..raw.len() - 1]), &created.unlock, 200000).is_err());
    let qr = matrix(&staff).unwrap();
    assert!(qr.len() > 21);
    assert!(qr.iter().all(|row| row.len() == qr.len()));
}

#[test]
fn generate_disposable_native_import_fixtures() {
    let root = create_root("Synthetic event", 202000, 201000, 200000).unwrap();
    let open = OpenRoot::open(&root.vault, &root.unlock, 200000).unwrap();
    let a = open.issue("Stage A", 199990, 201000, 200000).unwrap();
    let b = open.issue("Stage B", 199990, 201000, 200000).unwrap();
    let other = create_root("Other event", 202000, 201000, 200000).unwrap();
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/mc041");
    std::fs::create_dir_all(&folder).unwrap();
    // Disposable randomly generated synthetic staff material, never a real root
    // seed/unlock code. These ignored files are native test inputs, never logs.
    std::fs::write(
        folder.join("fixtures.tsv"),
        format!(
            "event\t{}\nstaff_a\t{}\nstaff_b\t{}\nother\t{}\nmatrix\t{}\n",
            root.event,
            &*a,
            &*b,
            other.event,
            matrix(&a).unwrap().join("/")
        ),
    )
    .unwrap();
    assert!(mobile_import(&root.event, &a, 200000));
    assert!(mobile_import(&root.event, &b, 200000));
}

#[test]
fn explicit_validity_labels_and_malformed_candidates_fail_closed() {
    assert!(create_root("", 202000, 201000, 200000).is_err());
    assert!(create_root("Name", 200000, 201000, 200000).is_err());
    let created = create_root("Event", 202000, 201000, 200000).unwrap();
    let root = OpenRoot::open(&created.vault, &created.unlock, 200000).unwrap();
    for (label, before, after) in [
        ("", 199990, 201000),
        ("abcdefghijklmnopq", 199990, 201000),
        ("Ops", 201001, 201000),
        ("Ops", 199990, 202001),
        ("Ops", 199990, 200000),
        ("A\nB", 199990, 201000),
    ] {
        assert!(root.issue(label, before, after, 200000).is_err());
    }
    assert!(validate_staff(&created.event, "meshfest://staff/no/key", 200000).is_err());
}

#[test]
fn event_end_plus_one_day_is_a_mandatory_authenticated_limit() {
    let end = 201000;
    assert!(create_root("Event", end + 86401, end, 200000).is_err());
    assert!(create_root("Event", end + 86400, 0, 200000).is_err());
    let created = create_root("Event", end + 86400, end, 200000).unwrap();
    let opened = OpenRoot::open(&created.vault, &created.unlock, 200000).unwrap();
    assert_eq!(opened.event_end, end);
    assert!(opened.issue("Ops", 200000, end + 86400, 200000).is_ok());
    assert!(opened.issue("Ops", 200000, end + 86401, 200000).is_err());
    // The u32 wire endpoint does not wrap the local +24h comparison.
    assert!(create_root("Edge", u32::MAX, u32::MAX - 1, u32::MAX - 100).is_ok());
}
