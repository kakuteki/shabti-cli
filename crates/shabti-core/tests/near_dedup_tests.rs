use shabti_core::near_dedup::NearDuplicateDetector;

#[test]
fn detect_near_duplicate_identical_vectors() {
    let detector = NearDuplicateDetector::new(0.98);
    let a = vec![1.0f32, 0.0, 0.0];
    assert!(
        detector.is_near_duplicate(&a, &a),
        "identical vectors should be near duplicates"
    );
}

#[test]
fn detect_near_duplicate_very_similar_vectors() {
    let detector = NearDuplicateDetector::new(0.98);
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.999f32, 0.01, 0.0];
    assert!(
        detector.is_near_duplicate(&a, &b),
        "very similar vectors should be near duplicates"
    );
}

#[test]
fn detect_not_near_duplicate_different_vectors() {
    let detector = NearDuplicateDetector::new(0.98);
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    assert!(
        !detector.is_near_duplicate(&a, &b),
        "orthogonal vectors should not be near duplicates"
    );
}

#[test]
fn detect_not_near_duplicate_somewhat_similar() {
    let detector = NearDuplicateDetector::new(0.98);
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.9, 0.4, 0.0]; // similarity ~0.91
    assert!(
        !detector.is_near_duplicate(&a, &b),
        "somewhat similar vectors (below threshold) should not be near duplicates"
    );
}

#[test]
fn custom_threshold() {
    let detector = NearDuplicateDetector::new(0.5);
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.8, 0.6, 0.0]; // similarity ~0.8
    assert!(
        detector.is_near_duplicate(&a, &b),
        "with threshold=0.5, moderately similar vectors should match"
    );
}

#[test]
fn default_threshold_is_098() {
    let detector = NearDuplicateDetector::default();
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.999f32, 0.01, 0.0];
    assert!(detector.is_near_duplicate(&a, &b));
}

#[test]
fn zero_vector_not_near_duplicate() {
    let detector = NearDuplicateDetector::new(0.98);
    let zero = vec![0.0f32, 0.0, 0.0];
    let a = vec![1.0, 0.0, 0.0];
    assert!(
        !detector.is_near_duplicate(&zero, &a),
        "zero vector should not be near duplicate"
    );
}

#[test]
fn group_tracking() {
    let mut detector = NearDuplicateDetector::new(0.98);
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.999f32, 0.01, 0.0]; // near-dup of a
    let c = vec![0.0, 1.0, 0.0]; // different

    let group_a = detector.assign_group(&a);
    let group_b = detector.assign_group(&b);
    let group_c = detector.assign_group(&c);

    assert_eq!(
        group_a, group_b,
        "near-duplicate vectors should be in the same group"
    );
    assert_ne!(
        group_a, group_c,
        "different vectors should be in different groups"
    );
}

#[test]
fn group_count() {
    let mut detector = NearDuplicateDetector::new(0.98);
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.999f32, 0.01, 0.0]; // near-dup of a
    let c = vec![0.0, 1.0, 0.0]; // different
    let d = vec![0.0, 0.999, 0.01]; // near-dup of c

    detector.assign_group(&a);
    detector.assign_group(&b);
    detector.assign_group(&c);
    detector.assign_group(&d);

    assert_eq!(detector.group_count(), 2, "should have 2 groups");
}

#[test]
fn deduplicate_results_keeps_one_per_group() {
    let detector = NearDuplicateDetector::new(0.98);
    let entries = vec![
        ("group1", 0.95f32, 5u32), // group, score, access_count
        ("group1", 0.90, 10),
        ("group2", 0.85, 3),
        ("group1", 0.80, 1),
    ];

    // Should keep: group1 entry with highest access (10), group2 entry
    let deduped = detector.deduplicate_by_group(&entries);
    assert_eq!(deduped.len(), 2);
    // The group1 representative should be the one with access_count=10
    let g1 = deduped.iter().find(|e| e.0 == "group1").unwrap();
    assert_eq!(g1.2, 10);
}
