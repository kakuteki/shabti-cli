use shabti_core::dedup::{DedupChecker, StoreResult};
use uuid::Uuid;

#[test]
fn new_checker_is_empty() {
    let checker = DedupChecker::new();
    assert!(checker.is_empty());
    assert_eq!(checker.len(), 0);
}

#[test]
fn first_insert_returns_stored() {
    let mut checker = DedupChecker::new();
    let id = Uuid::new_v4();
    let result = checker.check_and_insert("abc123", id);
    assert_eq!(result, StoreResult::Stored(id));
}

#[test]
fn duplicate_insert_returns_skipped() {
    let mut checker = DedupChecker::new();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    checker.check_and_insert("abc123", id1);
    let result = checker.check_and_insert("abc123", id2);

    assert_eq!(result, StoreResult::Skipped { existing_id: id1 });
}

#[test]
fn different_hashes_are_not_duplicates() {
    let mut checker = DedupChecker::new();
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    let r1 = checker.check_and_insert("hash1", id1);
    let r2 = checker.check_and_insert("hash2", id2);

    assert_eq!(r1, StoreResult::Stored(id1));
    assert_eq!(r2, StoreResult::Stored(id2));
}

#[test]
fn contains_reports_correctly() {
    let mut checker = DedupChecker::new();
    let id = Uuid::new_v4();

    assert!(!checker.contains("hash1"));
    checker.check_and_insert("hash1", id);
    assert!(checker.contains("hash1"));
    assert!(!checker.contains("hash2"));
}

#[test]
fn len_tracks_unique_entries() {
    let mut checker = DedupChecker::new();

    checker.check_and_insert("h1", Uuid::new_v4());
    assert_eq!(checker.len(), 1);

    checker.check_and_insert("h2", Uuid::new_v4());
    assert_eq!(checker.len(), 2);

    // Duplicate doesn't increase len
    checker.check_and_insert("h1", Uuid::new_v4());
    assert_eq!(checker.len(), 2);
}

#[test]
fn handles_many_entries() {
    let mut checker = DedupChecker::new();
    for i in 0..1000 {
        let hash = format!("hash_{i}");
        let result = checker.check_and_insert(&hash, Uuid::new_v4());
        assert!(matches!(result, StoreResult::Stored(_)));
    }
    assert_eq!(checker.len(), 1000);

    // All should be detected as duplicates
    for i in 0..1000 {
        let hash = format!("hash_{i}");
        assert!(checker.contains(&hash));
    }
}
