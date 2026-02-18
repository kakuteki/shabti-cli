use shabti_core::query::{Query, QueryBuilder, SearchExplanation};

// ============================================================
// QueryBuilder
// ============================================================

#[test]
fn builder_default_values() {
    let query = QueryBuilder::new("test query").build();
    assert_eq!(query.text, "test query");
    assert_eq!(query.limit, 10);
    assert!(query.namespace.is_none());
    assert!(query.time_start.is_none());
    assert!(query.time_end.is_none());
    assert!(query.cluster_id.is_none());
    assert_eq!(query.max_hops, 0);
    assert!(query.min_score.is_none());
    assert!(!query.exclude_superseded);
    assert!(!query.with_explanation);
}

#[test]
fn builder_semantic_query() {
    let query = QueryBuilder::new("find cats")
        .limit(5)
        .build();
    assert_eq!(query.text, "find cats");
    assert_eq!(query.limit, 5);
}

#[test]
fn builder_time_range() {
    let query = QueryBuilder::new("query")
        .time_range(1000, 2000)
        .build();
    assert_eq!(query.time_start, Some(1000));
    assert_eq!(query.time_end, Some(2000));
}

#[test]
fn builder_namespace_filter() {
    let query = QueryBuilder::new("query")
        .namespace("tenant-a")
        .build();
    assert_eq!(query.namespace.as_deref(), Some("tenant-a"));
}

#[test]
fn builder_cluster_filter() {
    let query = QueryBuilder::new("query")
        .in_cluster(3)
        .build();
    assert_eq!(query.cluster_id, Some(3));
}

#[test]
fn builder_follow_links() {
    let query = QueryBuilder::new("query")
        .follow_links(2)
        .build();
    assert_eq!(query.max_hops, 2);
}

#[test]
fn builder_min_score() {
    let query = QueryBuilder::new("query")
        .min_score(0.5)
        .build();
    assert_eq!(query.min_score, Some(0.5));
}

#[test]
fn builder_exclude_superseded() {
    let query = QueryBuilder::new("query")
        .exclude_superseded()
        .build();
    assert!(query.exclude_superseded);
}

#[test]
fn builder_with_explanation() {
    let query = QueryBuilder::new("query")
        .with_explanation()
        .build();
    assert!(query.with_explanation);
}

#[test]
fn builder_full_chain() {
    let query = QueryBuilder::new("complex query")
        .namespace("ns")
        .time_range(100, 200)
        .in_cluster(1)
        .follow_links(3)
        .min_score(0.7)
        .exclude_superseded()
        .with_explanation()
        .limit(20)
        .build();

    assert_eq!(query.text, "complex query");
    assert_eq!(query.namespace.as_deref(), Some("ns"));
    assert_eq!(query.time_start, Some(100));
    assert_eq!(query.time_end, Some(200));
    assert_eq!(query.cluster_id, Some(1));
    assert_eq!(query.max_hops, 3);
    assert_eq!(query.min_score, Some(0.7));
    assert!(query.exclude_superseded);
    assert!(query.with_explanation);
    assert_eq!(query.limit, 20);
}

// ============================================================
// SearchExplanation
// ============================================================

#[test]
fn explanation_default_values() {
    let explanation = SearchExplanation::default();
    assert_eq!(explanation.semantic_similarity, 0.0);
    assert_eq!(explanation.time_decay_factor, 1.0);
    assert_eq!(explanation.access_boost_factor, 1.0);
    assert_eq!(explanation.matched_at_level, 0);
    assert_eq!(explanation.link_hops, 0);
    assert!(!explanation.is_superseded);
    assert_eq!(explanation.dedupe_count, 0);
}

#[test]
fn explanation_final_score() {
    let explanation = SearchExplanation {
        semantic_similarity: 0.9,
        time_decay_factor: 0.8,
        access_boost_factor: 1.5,
        matched_at_level: 0,
        link_hops: 0,
        is_superseded: false,
        dedupe_count: 0,
    };
    let score = explanation.final_score();
    let expected = 0.9 * 0.8 * 1.5;
    assert!(
        (score - expected).abs() < 1e-6,
        "final_score should be product of factors: expected {expected}, got {score}"
    );
}
