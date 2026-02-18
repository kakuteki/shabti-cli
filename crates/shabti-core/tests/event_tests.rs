use shabti_core::event::{
    cosine_distance, cosine_similarity, consecutive_distances,
    detect_boundaries, detect_boundaries_adaptive, detect_time_gaps,
    merge_boundaries, segment_events, AdaptiveThreshold, Boundary,
};

// ============================================================
// 2-1: Cosine distance & boundary detection
// ============================================================

#[test]
fn cosine_similarity_identical_vectors() {
    let a = vec![1.0, 2.0, 3.0];
    let result = cosine_similarity(&a, &a);
    assert!((result - 1.0).abs() < 1e-6, "identical vectors should have similarity 1.0, got {result}");
}

#[test]
fn cosine_similarity_orthogonal_vectors() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let result = cosine_similarity(&a, &b);
    assert!(result.abs() < 1e-6, "orthogonal vectors should have similarity 0.0, got {result}");
}

#[test]
fn cosine_similarity_opposite_vectors() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![-1.0, 0.0, 0.0];
    let result = cosine_similarity(&a, &b);
    assert!((result + 1.0).abs() < 1e-6, "opposite vectors should have similarity -1.0, got {result}");
}

#[test]
fn cosine_similarity_zero_vector_returns_zero() {
    let a = vec![0.0, 0.0, 0.0];
    let b = vec![1.0, 2.0, 3.0];
    assert_eq!(cosine_similarity(&a, &b), 0.0);
    assert_eq!(cosine_similarity(&b, &a), 0.0);
}

#[test]
fn cosine_distance_identical_is_zero() {
    let a = vec![1.0, 0.0, 0.0];
    let d = cosine_distance(&a, &a);
    assert!(d.abs() < 1e-6, "distance of identical vectors should be 0, got {d}");
}

#[test]
fn cosine_distance_orthogonal_is_one() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let d = cosine_distance(&a, &b);
    assert!((d - 1.0).abs() < 1e-6, "distance of orthogonal vectors should be 1, got {d}");
}

#[test]
fn consecutive_distances_length() {
    let embeddings = vec![
        vec![1.0, 0.0, 0.0],
        vec![0.9, 0.1, 0.0],
        vec![0.0, 0.0, 1.0],
    ];
    let distances = consecutive_distances(&embeddings);
    assert_eq!(distances.len(), 2, "N embeddings should produce N-1 distances");
}

#[test]
fn consecutive_distances_empty_input() {
    let distances = consecutive_distances(&[]);
    assert!(distances.is_empty());
}

#[test]
fn consecutive_distances_single_input() {
    let distances = consecutive_distances(&[vec![1.0, 0.0]]);
    assert!(distances.is_empty());
}

#[test]
fn consecutive_distances_detects_topic_shift() {
    // Cluster A: similar vectors
    let a1 = vec![1.0, 0.0, 0.0];
    let a2 = vec![0.95, 0.05, 0.0];
    // Cluster B: very different direction
    let b1 = vec![0.0, 0.0, 1.0];
    let b2 = vec![0.0, 0.05, 0.95];

    let distances = consecutive_distances(&[a1, a2, b1, b2]);
    // The jump from a2 to b1 should be the largest distance
    assert!(distances[1] > distances[0], "topic shift distance should be larger than within-topic");
    assert!(distances[1] > distances[2], "topic shift distance should be larger than within-topic");
}

#[test]
fn detect_boundaries_with_threshold() {
    let distances = vec![0.05, 0.03, 0.8, 0.04, 0.9, 0.02];
    let boundaries = detect_boundaries(&distances, 0.5);
    assert_eq!(boundaries.len(), 2);
    assert_eq!(boundaries[0].index, 2);
    assert!((boundaries[0].distance - 0.8).abs() < 1e-6);
    assert_eq!(boundaries[1].index, 4);
    assert!((boundaries[1].distance - 0.9).abs() < 1e-6);
}

#[test]
fn detect_boundaries_none_when_below_threshold() {
    let distances = vec![0.1, 0.2, 0.15, 0.1];
    let boundaries = detect_boundaries(&distances, 0.5);
    assert!(boundaries.is_empty());
}

#[test]
fn detect_boundaries_empty_input() {
    let boundaries = detect_boundaries(&[], 0.5);
    assert!(boundaries.is_empty());
}

#[test]
fn segment_events_no_boundaries() {
    let segments = segment_events(5, &[]);
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0], vec![0, 1, 2, 3, 4]);
}

#[test]
fn segment_events_one_boundary() {
    let boundaries = vec![Boundary { index: 2, distance: 0.8 }];
    let segments = segment_events(5, &boundaries);
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0], vec![0, 1, 2]);
    assert_eq!(segments[1], vec![3, 4]);
}

#[test]
fn segment_events_multiple_boundaries() {
    let boundaries = vec![
        Boundary { index: 1, distance: 0.7 },
        Boundary { index: 3, distance: 0.9 },
    ];
    let segments = segment_events(6, &boundaries);
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0], vec![0, 1]);
    assert_eq!(segments[1], vec![2, 3]);
    assert_eq!(segments[2], vec![4, 5]);
}

#[test]
fn segment_events_zero_utterances() {
    let segments = segment_events(0, &[]);
    assert!(segments.is_empty());
}

// ============================================================
// 2-2: Adaptive threshold
// ============================================================

#[test]
fn adaptive_threshold_homogeneous_data_no_boundaries() {
    // All distances are similar — no peaks
    let distances = vec![0.1, 0.11, 0.09, 0.1, 0.12, 0.1, 0.11, 0.09, 0.1, 0.11];
    let config = AdaptiveThreshold {
        window_size: 5,
        sensitivity: 2.0,
    };
    let boundaries = detect_boundaries_adaptive(&distances, &config);
    assert!(boundaries.is_empty(), "homogeneous data should have no boundaries, got {:?}", boundaries);
}

#[test]
fn adaptive_threshold_clear_peak() {
    // One clear spike amid low values
    let distances = vec![0.05, 0.04, 0.06, 0.05, 0.8, 0.04, 0.05, 0.06];
    let config = AdaptiveThreshold {
        window_size: 3,
        sensitivity: 2.0,
    };
    let boundaries = detect_boundaries_adaptive(&distances, &config);
    assert_eq!(boundaries.len(), 1, "should detect one boundary at the peak");
    assert_eq!(boundaries[0].index, 4);
}

#[test]
fn adaptive_threshold_window_size_effect() {
    // With a larger window, context changes — let's just verify it doesn't panic
    let distances = vec![0.1, 0.1, 0.8, 0.1, 0.1];
    let small_window = AdaptiveThreshold { window_size: 2, sensitivity: 2.0 };
    let large_window = AdaptiveThreshold { window_size: 4, sensitivity: 2.0 };
    let b1 = detect_boundaries_adaptive(&distances, &small_window);
    let b2 = detect_boundaries_adaptive(&distances, &large_window);
    // Both should detect the peak at index 2
    assert!(b1.iter().any(|b| b.index == 2), "small window should detect peak");
    assert!(b2.iter().any(|b| b.index == 2), "large window should detect peak");
}

#[test]
fn adaptive_threshold_fallback_for_small_data() {
    // Less than 3 distances — should use fixed threshold fallback
    let distances = vec![0.8, 0.1];
    let config = AdaptiveThreshold {
        window_size: 5,
        sensitivity: 2.0,
    };
    let boundaries = detect_boundaries_adaptive(&distances, &config);
    // Should still detect the high distance even with fallback
    assert!(boundaries.iter().any(|b| b.index == 0), "should detect high distance with fallback");
}

#[test]
fn adaptive_threshold_empty_input() {
    let config = AdaptiveThreshold { window_size: 3, sensitivity: 2.0 };
    let boundaries = detect_boundaries_adaptive(&[], &config);
    assert!(boundaries.is_empty());
}

// ============================================================
// 2-3: Time gap detection
// ============================================================

#[test]
fn detect_time_gaps_long_gap() {
    // Timestamps with a large gap between index 2 and 3
    let timestamps = vec![1000, 1010, 1020, 5000, 5010, 5020];
    let gap_threshold = 3600; // 1 hour
    let gaps = detect_time_gaps(&timestamps, gap_threshold);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0], 2, "gap should be between index 2 and 3");
}

#[test]
fn detect_time_gaps_no_gap() {
    let timestamps = vec![1000, 1010, 1020, 1030];
    let gaps = detect_time_gaps(&timestamps, 3600);
    assert!(gaps.is_empty());
}

#[test]
fn detect_time_gaps_multiple_gaps() {
    let timestamps = vec![1000, 1010, 5000, 5010, 9000, 9010];
    let gaps = detect_time_gaps(&timestamps, 3600);
    assert_eq!(gaps.len(), 2);
    assert_eq!(gaps[0], 1);
    assert_eq!(gaps[1], 3);
}

#[test]
fn detect_time_gaps_empty_input() {
    let gaps = detect_time_gaps(&[], 3600);
    assert!(gaps.is_empty());
}

#[test]
fn detect_time_gaps_single_entry() {
    let gaps = detect_time_gaps(&[1000], 3600);
    assert!(gaps.is_empty());
}

#[test]
fn merge_boundaries_deduplicates() {
    let semantic = vec![
        Boundary { index: 2, distance: 0.8 },
        Boundary { index: 5, distance: 0.7 },
    ];
    let temporal = vec![2, 4]; // index 2 overlaps with semantic
    let merged = merge_boundaries(&semantic, &temporal);
    // Should have 3 unique indices: 2, 4, 5
    let indices: Vec<usize> = merged.iter().map(|b| b.index).collect();
    assert_eq!(indices.len(), 3);
    assert!(indices.contains(&2));
    assert!(indices.contains(&4));
    assert!(indices.contains(&5));
}

#[test]
fn merge_boundaries_preserves_semantic_distance() {
    let semantic = vec![Boundary { index: 3, distance: 0.9 }];
    let temporal = vec![3]; // same index
    let merged = merge_boundaries(&semantic, &temporal);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].index, 3);
    // Should preserve the semantic distance since it has real distance info
    assert!((merged[0].distance - 0.9).abs() < 1e-6);
}

#[test]
fn merge_boundaries_sorted_by_index() {
    let semantic = vec![Boundary { index: 5, distance: 0.7 }];
    let temporal = vec![2];
    let merged = merge_boundaries(&semantic, &temporal);
    assert_eq!(merged[0].index, 2);
    assert_eq!(merged[1].index, 5);
}

// ============================================================
// Evaluation metrics (2-10 prep)
// ============================================================

#[test]
fn evaluate_boundaries_perfect_match() {
    let detected = vec![
        Boundary { index: 3, distance: 0.8 },
        Boundary { index: 7, distance: 0.9 },
    ];
    let ground_truth = vec![3, 7];
    let (precision, recall) = shabti_core::event::evaluate_boundaries(&detected, &ground_truth, 0);
    assert!((precision - 1.0).abs() < 1e-6);
    assert!((recall - 1.0).abs() < 1e-6);
}

#[test]
fn evaluate_boundaries_with_tolerance() {
    let detected = vec![Boundary { index: 4, distance: 0.8 }]; // off by 1
    let ground_truth = vec![3];
    let (precision, recall) = shabti_core::event::evaluate_boundaries(&detected, &ground_truth, 1);
    assert!((precision - 1.0).abs() < 1e-6, "within tolerance should match");
    assert!((recall - 1.0).abs() < 1e-6);
}

#[test]
fn evaluate_boundaries_missed_detection() {
    let detected = vec![Boundary { index: 3, distance: 0.8 }];
    let ground_truth = vec![3, 7]; // 7 is missed
    let (precision, recall) = shabti_core::event::evaluate_boundaries(&detected, &ground_truth, 0);
    assert!((precision - 1.0).abs() < 1e-6, "detected boundary is correct");
    assert!((recall - 0.5).abs() < 1e-6, "only 1 of 2 ground truth found");
}

#[test]
fn evaluate_boundaries_false_positive() {
    let detected = vec![
        Boundary { index: 3, distance: 0.8 },
        Boundary { index: 5, distance: 0.6 }, // false positive
    ];
    let ground_truth = vec![3];
    let (precision, recall) = shabti_core::event::evaluate_boundaries(&detected, &ground_truth, 0);
    assert!((precision - 0.5).abs() < 1e-6, "1 of 2 detected is correct");
    assert!((recall - 1.0).abs() < 1e-6, "ground truth boundary found");
}

#[test]
fn evaluate_boundaries_empty_detected() {
    let detected: Vec<Boundary> = vec![];
    let ground_truth = vec![3];
    let (precision, recall) = shabti_core::event::evaluate_boundaries(&detected, &ground_truth, 0);
    assert_eq!(precision, 0.0);
    assert_eq!(recall, 0.0);
}

#[test]
fn evaluate_boundaries_empty_ground_truth() {
    let detected = vec![Boundary { index: 3, distance: 0.8 }];
    let ground_truth: Vec<usize> = vec![];
    let (precision, recall) = shabti_core::event::evaluate_boundaries(&detected, &ground_truth, 0);
    assert_eq!(precision, 0.0);
    assert_eq!(recall, 0.0);
}
