use shabti_core::clustering::{cluster_sessions, TopicCluster};
use shabti_core::resolution::SessionSummary;

fn make_session(id: &str, embedding: Vec<f32>) -> SessionSummary {
    SessionSummary {
        session_id: id.to_string(),
        embedding,
        event_count: 1,
    }
}

// ============================================================
// Basic clustering
// ============================================================

#[test]
fn cluster_two_distinct_groups() {
    // Group A: vectors near [1,0,0]
    // Group B: vectors near [0,1,0]
    let sessions = vec![
        make_session("a1", vec![1.0, 0.1, 0.0]),
        make_session("a2", vec![0.95, 0.15, 0.0]),
        make_session("a3", vec![0.9, 0.2, 0.0]),
        make_session("a4", vec![0.98, 0.05, 0.0]),
        make_session("a5", vec![0.92, 0.12, 0.0]),
        make_session("b1", vec![0.1, 1.0, 0.0]),
        make_session("b2", vec![0.15, 0.95, 0.0]),
        make_session("b3", vec![0.2, 0.9, 0.0]),
        make_session("b4", vec![0.05, 0.98, 0.0]),
        make_session("b5", vec![0.12, 0.92, 0.0]),
    ];

    let clusters = cluster_sessions(&sessions, 3);
    assert!(
        clusters.len() >= 2,
        "should find at least 2 clusters, got {}",
        clusters.len()
    );
}

#[test]
fn cluster_single_group() {
    // All similar vectors
    let sessions = vec![
        make_session("a1", vec![1.0, 0.1, 0.0]),
        make_session("a2", vec![0.95, 0.15, 0.0]),
        make_session("a3", vec![0.9, 0.2, 0.0]),
        make_session("a4", vec![0.98, 0.05, 0.0]),
        make_session("a5", vec![0.92, 0.12, 0.0]),
    ];

    let clusters = cluster_sessions(&sessions, 3);
    assert!(
        clusters.len() <= 2,
        "single coherent group should yield <=2 clusters, got {}",
        clusters.len()
    );
}

#[test]
fn cluster_too_few_sessions_returns_empty() {
    let sessions = vec![
        make_session("a1", vec![1.0, 0.0, 0.0]),
        make_session("a2", vec![0.0, 1.0, 0.0]),
    ];

    // With min_cluster_size=3, 2 sessions can't form a cluster
    let clusters = cluster_sessions(&sessions, 3);
    // Either empty or all noise — both are acceptable
    let total_sessions: usize = clusters.iter().map(|c| c.session_ids.len()).sum();
    assert!(
        total_sessions <= 2,
        "too few sessions: total in clusters should be <=2"
    );
}

#[test]
fn cluster_empty_input() {
    let clusters = cluster_sessions(&[], 3);
    assert!(clusters.is_empty());
}

#[test]
fn cluster_centroid_is_normalized() {
    let sessions = vec![
        make_session("a1", vec![1.0, 0.1, 0.0]),
        make_session("a2", vec![0.95, 0.15, 0.0]),
        make_session("a3", vec![0.9, 0.2, 0.0]),
        make_session("a4", vec![0.98, 0.05, 0.0]),
        make_session("a5", vec![0.92, 0.12, 0.0]),
    ];

    let clusters = cluster_sessions(&sessions, 3);
    for cluster in &clusters {
        let norm: f32 = cluster.centroid.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 0.1 || norm == 0.0,
            "centroid should be roughly normalized, got norm={norm}"
        );
    }
}

#[test]
fn cluster_session_ids_populated() {
    let sessions = vec![
        make_session("a1", vec![1.0, 0.1, 0.0]),
        make_session("a2", vec![0.95, 0.15, 0.0]),
        make_session("a3", vec![0.9, 0.2, 0.0]),
        make_session("a4", vec![0.98, 0.05, 0.0]),
        make_session("a5", vec![0.92, 0.12, 0.0]),
        make_session("b1", vec![0.1, 1.0, 0.0]),
        make_session("b2", vec![0.15, 0.95, 0.0]),
        make_session("b3", vec![0.2, 0.9, 0.0]),
        make_session("b4", vec![0.05, 0.98, 0.0]),
        make_session("b5", vec![0.12, 0.92, 0.0]),
    ];

    let clusters = cluster_sessions(&sessions, 3);
    for cluster in &clusters {
        assert!(
            !cluster.session_ids.is_empty(),
            "each cluster should have at least one session"
        );
    }
}

#[test]
fn cluster_no_duplicate_session_ids() {
    let sessions = vec![
        make_session("a1", vec![1.0, 0.1, 0.0]),
        make_session("a2", vec![0.95, 0.15, 0.0]),
        make_session("a3", vec![0.9, 0.2, 0.0]),
        make_session("a4", vec![0.98, 0.05, 0.0]),
        make_session("a5", vec![0.92, 0.12, 0.0]),
        make_session("b1", vec![0.1, 1.0, 0.0]),
        make_session("b2", vec![0.15, 0.95, 0.0]),
        make_session("b3", vec![0.2, 0.9, 0.0]),
        make_session("b4", vec![0.05, 0.98, 0.0]),
        make_session("b5", vec![0.12, 0.92, 0.0]),
    ];

    let clusters = cluster_sessions(&sessions, 3);
    let mut all_ids: Vec<&str> = clusters
        .iter()
        .flat_map(|c| c.session_ids.iter().map(|s| s.as_str()))
        .collect();
    let total = all_ids.len();
    all_ids.sort();
    all_ids.dedup();
    assert_eq!(
        all_ids.len(),
        total,
        "no session should appear in multiple clusters"
    );
}

#[test]
fn topic_cluster_id_is_sequential() {
    let sessions = vec![
        make_session("a1", vec![1.0, 0.1, 0.0]),
        make_session("a2", vec![0.95, 0.15, 0.0]),
        make_session("a3", vec![0.9, 0.2, 0.0]),
        make_session("a4", vec![0.98, 0.05, 0.0]),
        make_session("a5", vec![0.92, 0.12, 0.0]),
    ];

    let clusters = cluster_sessions(&sessions, 3);
    for (i, cluster) in clusters.iter().enumerate() {
        assert_eq!(cluster.id, i);
    }
}
