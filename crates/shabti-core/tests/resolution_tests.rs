use shabti_core::resolution::{SessionSummary, aggregate_embeddings, normalize_l2};

// ============================================================
// L2 normalization
// ============================================================

#[test]
fn normalize_l2_unit_vector_unchanged() {
    let v = vec![1.0f32, 0.0, 0.0];
    let n = normalize_l2(&v);
    for (a, b) in n.iter().zip(v.iter()) {
        assert!((a - b).abs() < 1e-6);
    }
}

#[test]
fn normalize_l2_result_has_unit_norm() {
    let v = vec![3.0f32, 4.0, 0.0];
    let n = normalize_l2(&v);
    let norm: f32 = n.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 1e-5,
        "normalized vector should have norm ~1.0, got {norm}"
    );
}

#[test]
fn normalize_l2_zero_vector_returns_zero() {
    let v = vec![0.0f32, 0.0, 0.0];
    let n = normalize_l2(&v);
    assert!(n.iter().all(|&x| x == 0.0));
}

// ============================================================
// Embedding aggregation
// ============================================================

#[test]
fn aggregate_single_embedding_returns_normalized() {
    let embeddings = vec![vec![3.0f32, 4.0, 0.0]];
    let weights = vec![1.0f32];
    let result = aggregate_embeddings(&embeddings, &weights);
    let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 1e-5,
        "single embedding should be L2-normalized"
    );
    // Direction should be preserved: [3,4,0] → [0.6, 0.8, 0]
    assert!(result[1] > result[0]);
}

#[test]
fn aggregate_equal_weights_gives_mean_direction() {
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let result = aggregate_embeddings(&[a, b], &[1.0, 1.0]);
    // Mean of [1,0,0] and [0,1,0] = [0.5, 0.5, 0], normalized = [1/√2, 1/√2, 0]
    let expected = 1.0 / 2.0f32.sqrt();
    assert!((result[0] - expected).abs() < 1e-4);
    assert!((result[1] - expected).abs() < 1e-4);
    assert!(result[2].abs() < 1e-6);
}

#[test]
fn aggregate_weighted_towards_higher_weight() {
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let result = aggregate_embeddings(&[a, b], &[3.0, 1.0]);
    // Weighted mean: [0.75, 0.25, 0], normalized
    assert!(
        result[0] > result[1],
        "higher weight should pull result towards that vector"
    );
}

#[test]
fn aggregate_empty_returns_empty() {
    let result = aggregate_embeddings(&[], &[]);
    assert!(result.is_empty());
}

#[test]
fn aggregate_result_is_l2_normalized() {
    let embeddings = vec![
        vec![1.0f32, 2.0, 3.0],
        vec![4.0, 5.0, 6.0],
        vec![7.0, 8.0, 9.0],
    ];
    let weights = vec![1.0, 2.0, 0.5];
    let result = aggregate_embeddings(&embeddings, &weights);
    let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!(
        (norm - 1.0).abs() < 1e-5,
        "result should be L2-normalized, got norm={norm}"
    );
}

// ============================================================
// SessionSummary
// ============================================================

#[test]
fn session_summary_from_single_event() {
    let event_embeddings = vec![vec![1.0f32, 0.0, 0.0]];
    let summary = SessionSummary::from_event_embeddings("sess-1".to_string(), &event_embeddings);
    assert_eq!(summary.session_id, "sess-1");
    assert_eq!(summary.event_count, 1);
    let norm: f32 = summary.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-5);
}

#[test]
fn session_summary_from_multiple_events() {
    let event_embeddings = vec![
        vec![1.0f32, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ];
    let summary = SessionSummary::from_event_embeddings("sess-2".to_string(), &event_embeddings);
    assert_eq!(summary.event_count, 3);
    // All three components should be roughly equal
    let max = summary.embedding.iter().cloned().fold(0.0f32, f32::max);
    let min = summary.embedding.iter().cloned().fold(1.0f32, f32::min);
    assert!(
        (max - min).abs() < 0.1,
        "three orthogonal vectors should average to roughly equal components"
    );
}

#[test]
fn session_summary_empty_events() {
    let summary = SessionSummary::from_event_embeddings("sess-empty".to_string(), &[]);
    assert_eq!(summary.event_count, 0);
    assert!(summary.embedding.is_empty());
}
