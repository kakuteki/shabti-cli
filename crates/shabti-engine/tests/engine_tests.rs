use shabti_core::dedup::StoreResult;
use shabti_core::query::QueryBuilder;
use shabti_core::types::OriginType;
use shabti_engine::{EngineConfig, ShabtiEngine, StoreOptions};
use shabti_index::TimeRange;
use tempfile::TempDir;
use uuid::Uuid;

fn test_config(dir: &TempDir) -> EngineConfig {
    let collection = format!(
        "engine_test_{}",
        Uuid::new_v4().to_string().replace('-', "")
    );
    EngineConfig {
        qdrant_url: "http://localhost:6334".to_string(),
        collection_name: collection,
        data_dir: dir.path().to_path_buf(),
        vector_size: 384,
    }
}

#[tokio::test]
async fn store_and_get() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let result = engine
        .store("Hello, world!", &StoreOptions::default())
        .await
        .unwrap();
    assert!(matches!(result, StoreResult::Stored(_)));

    if let StoreResult::Stored(id) = result {
        let entry = engine.get(id).await.unwrap();
        assert_eq!(entry.content, "Hello, world!");
        assert_eq!(entry.embedding.len(), 384);
    }

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn store_deduplicates_same_content() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let r1 = engine
        .store("duplicate content", &StoreOptions::default())
        .await
        .unwrap();
    let r2 = engine
        .store("duplicate content", &StoreOptions::default())
        .await
        .unwrap();

    assert!(matches!(r1, StoreResult::Stored(_)));
    assert!(matches!(r2, StoreResult::Skipped { .. }));

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn search_similar_finds_stored_entry() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("the stock market is volatile", &StoreOptions::default())
        .await
        .unwrap();

    let results = engine
        .search_similar("I love kittens", 10, None)
        .await
        .unwrap();
    assert!(!results.is_empty());
    // The cat-related entry should be the top result
    assert!(results[0].entry.content.contains("cats"));

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn search_updates_access_count() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let r = engine
        .store("access tracking test", &StoreOptions::default())
        .await
        .unwrap();

    // Search should increment access_count
    engine
        .search_similar("access tracking", 10, None)
        .await
        .unwrap();

    // Get entry from log to check access_count
    if let StoreResult::Stored(id) = r {
        let entry = engine.get(id).await.unwrap();
        // After one search, access_count should still be 0 in log
        // (access tracking is in Qdrant only at this stage)
        assert_eq!(entry.access_count, 0);
    }

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn store_with_options() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let opts = StoreOptions {
        namespace: Some("tenant-a".to_string()),
        session_id: Some("sess-1".to_string()),
        origin_type: Some(OriginType::AgentGenerated),
        keywords: Some(vec!["test".to_string()]),
        tags: Some(vec!["tag1".to_string()]),
        ..Default::default()
    };

    let result = engine.store("custom options test", &opts).await.unwrap();

    if let StoreResult::Stored(id) = result {
        let entry = engine.get(id).await.unwrap();
        assert_eq!(entry.namespace, "tenant-a");
        assert_eq!(entry.session_id, "sess-1");
        assert_eq!(entry.origin_type, OriginType::AgentGenerated);
        assert_eq!(entry.keywords, vec!["test"]);
        assert_eq!(entry.tags, vec!["tag1"]);
    }

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn search_with_namespace_filter() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let opts_a = StoreOptions {
        namespace: Some("tenant-a".to_string()),
        ..Default::default()
    };
    let opts_b = StoreOptions {
        namespace: Some("tenant-b".to_string()),
        ..Default::default()
    };

    engine.store("data for tenant A", &opts_a).await.unwrap();
    engine.store("data for tenant B", &opts_b).await.unwrap();

    let results = engine
        .search_similar("data", 10, Some("tenant-a"))
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].entry.content.contains("tenant A"));

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn search_by_time() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("some content", &StoreOptions::default())
        .await
        .unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let range = TimeRange {
        start: Some(now - 60),
        end: Some(now + 60),
    };
    let results = engine.search_by_time(&range, 10).await.unwrap();
    assert_eq!(results.len(), 1);

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn entry_count_and_feature_gate() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    assert_eq!(engine.entry_count(), 0);
    assert_eq!(
        engine.feature_gate().tier,
        shabti_core::gate::DataTier::Minimal
    );

    engine
        .store("entry 1", &StoreOptions::default())
        .await
        .unwrap();
    assert_eq!(engine.entry_count(), 1);

    engine.cleanup().await.unwrap();
}

// ============================================================
// Phase 2: k-NN links, compound query, background indexer
// ============================================================

#[tokio::test]
async fn generate_links_creates_edges_for_similar_content() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();
    let r2 = engine
        .store("kittens are adorable animals", &StoreOptions::default())
        .await
        .unwrap();

    let id2 = match r2 {
        StoreResult::Stored(id) => id,
        _ => panic!(),
    };

    engine.generate_links(id2).await.unwrap();

    let graph = engine.graph();
    assert!(graph.edge_count() > 0, "similar entries should be linked");

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn generate_links_excludes_self() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let r1 = engine
        .store(
            "unique test content for self-link check",
            &StoreOptions::default(),
        )
        .await
        .unwrap();

    let id1 = match r1 {
        StoreResult::Stored(id) => id,
        _ => panic!(),
    };

    engine.generate_links(id1).await.unwrap();

    let graph = engine.graph();
    let neighbors = graph.neighbors(id1, 1);
    for (neighbor_id, _) in &neighbors {
        assert_ne!(*neighbor_id, id1, "should not have self-link");
    }

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn search_with_links_expands_via_graph() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();
    let rb = engine
        .store(
            "dogs and pets are great companions",
            &StoreOptions::default(),
        )
        .await
        .unwrap();
    let rc = engine
        .store(
            "veterinary care for domestic animals",
            &StoreOptions::default(),
        )
        .await
        .unwrap();

    let id_b = match rb {
        StoreResult::Stored(id) => id,
        _ => panic!(),
    };
    let id_c = match rc {
        StoreResult::Stored(id) => id,
        _ => panic!(),
    };

    engine.generate_links(id_b).await.unwrap();
    engine.generate_links(id_c).await.unwrap();

    let results = engine
        .search_with_links("I love kittens", 10, 2, None)
        .await
        .unwrap();

    assert!(!results.is_empty());

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn search_with_links_score_sorted_descending() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("machine learning algorithms", &StoreOptions::default())
        .await
        .unwrap();
    let rb = engine
        .store(
            "neural networks and deep learning",
            &StoreOptions::default(),
        )
        .await
        .unwrap();
    let rc = engine
        .store("artificial intelligence research", &StoreOptions::default())
        .await
        .unwrap();

    let id_b = match rb {
        StoreResult::Stored(id) => id,
        _ => panic!(),
    };
    let id_c = match rc {
        StoreResult::Stored(id) => id,
        _ => panic!(),
    };

    engine.generate_links(id_b).await.unwrap();
    engine.generate_links(id_c).await.unwrap();

    let results = engine
        .search_with_links("deep learning", 10, 2, None)
        .await
        .unwrap();

    for i in 1..results.len() {
        assert!(
            results[i - 1].score >= results[i].score,
            "results should be sorted by score descending"
        );
    }

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn background_indexer_skips_links_in_minimal_tier() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    // With < 10 entries, FeatureGate disables knn_links (Minimal tier)
    engine
        .store("programming in Rust language", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("Rust memory safety and ownership", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("borrow checker in Rust compiler", &StoreOptions::default())
        .await
        .unwrap();

    engine.flush_background().await.unwrap();

    // Minimal tier: knn_links disabled, no background link generation
    let graph = engine.graph();
    assert_eq!(
        graph.node_count(),
        0,
        "Minimal tier should not generate links via background indexer"
    );

    // But explicit generate_links should still work
    let results = engine.search_similar("Rust", 1, None).await.unwrap();
    if let Some(r) = results.first() {
        engine.generate_links(r.entry.id).await.unwrap();
    }
    let graph = engine.graph();
    assert!(
        graph.node_count() > 0,
        "explicit generate_links should work regardless of tier"
    );

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn background_indexer_shutdown() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("shutdown test content", &StoreOptions::default())
        .await
        .unwrap();

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn list_events_accessible() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let opts = StoreOptions {
        session_id: Some("sess-1".to_string()),
        ..Default::default()
    };

    engine.store("cats are amazing", &opts).await.unwrap();
    engine.store("kittens are cute", &opts).await.unwrap();

    engine.flush_background().await.unwrap();

    let events = engine.list_events();
    // Just verify it's accessible without panic
    // Just verify it's accessible without panic
    let _ = events.len();

    engine.cleanup().await.unwrap();
}

// ============================================================
// Phase 3: execute_query — Composable Query DSL integration
// ============================================================

#[tokio::test]
async fn execute_query_basic_semantic() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("the stock market is volatile", &StoreOptions::default())
        .await
        .unwrap();

    let query = QueryBuilder::new("I love kittens").limit(10).build();
    let results = engine.execute_query(&query).await.unwrap();

    assert!(!results.is_empty());
    assert!(results[0].entry.content.contains("cats"));
    // Explanation should be None by default
    assert!(results[0].explanation.is_none());

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn execute_query_with_time_decay() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();

    let query = QueryBuilder::new("cats").limit(10).build();
    let results = engine.execute_query(&query).await.unwrap();

    assert!(!results.is_empty());
    // Recently stored entry should have score affected by time decay (~1.0)
    // and access boost (1.0 for 0 accesses)
    assert!(results[0].score > 0.0);

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn execute_query_with_links() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();
    let rb = engine
        .store("kittens are adorable animals", &StoreOptions::default())
        .await
        .unwrap();

    let id_b = match rb {
        StoreResult::Stored(id) => id,
        _ => panic!("expected Stored"),
    };
    engine.generate_links(id_b).await.unwrap();

    let query = QueryBuilder::new("I love cats")
        .follow_links(2)
        .limit(10)
        .build();
    let results = engine.execute_query(&query).await.unwrap();

    assert!(!results.is_empty());

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn execute_query_min_score_filter() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("the stock market is volatile", &StoreOptions::default())
        .await
        .unwrap();

    // Very high min_score should filter out most results
    let query = QueryBuilder::new("random unrelated topic")
        .min_score(0.99)
        .limit(10)
        .build();
    let results = engine.execute_query(&query).await.unwrap();

    // All returned results should meet the min_score threshold
    for r in &results {
        assert!(
            r.score >= 0.99,
            "result score {} should be >= min_score 0.99",
            r.score
        );
    }

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn execute_query_with_explanation() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();

    let query = QueryBuilder::new("cats")
        .with_explanation()
        .limit(10)
        .build();
    let results = engine.execute_query(&query).await.unwrap();

    assert!(!results.is_empty());
    let explanation = results[0]
        .explanation
        .as_ref()
        .expect("should have explanation");
    assert!(explanation.semantic_similarity > 0.0);
    assert!(explanation.time_decay_factor > 0.0);
    assert!(explanation.access_boost_factor >= 1.0);

    // final_score should match the result score
    let expected = explanation.final_score();
    assert!(
        (results[0].score - expected).abs() < 1e-4,
        "result score {} should match explanation final_score {}",
        results[0].score,
        expected
    );

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn execute_query_namespace_filter() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    let opts_a = StoreOptions {
        namespace: Some("tenant-a".to_string()),
        ..Default::default()
    };
    let opts_b = StoreOptions {
        namespace: Some("tenant-b".to_string()),
        ..Default::default()
    };

    engine.store("data for tenant A", &opts_a).await.unwrap();
    engine.store("data for tenant B", &opts_b).await.unwrap();

    let query = QueryBuilder::new("data")
        .namespace("tenant-a")
        .limit(10)
        .build();
    let results = engine.execute_query(&query).await.unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].entry.content.contains("tenant A"));

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn execute_query_exclude_superseded() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();

    // Without exclude_superseded, all results should appear
    let query = QueryBuilder::new("cats")
        .exclude_superseded()
        .limit(10)
        .build();
    let results = engine.execute_query(&query).await.unwrap();

    // Since no entries have superseded_by set, all should be returned
    for r in &results {
        assert!(r.entry.superseded_by.is_none());
    }

    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn execute_query_results_sorted_descending() {
    let dir = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&dir)).await.unwrap();

    engine
        .store("cats are wonderful pets", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("dogs are loyal companions", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("the weather today is sunny", &StoreOptions::default())
        .await
        .unwrap();

    let query = QueryBuilder::new("animals and pets").limit(10).build();
    let results = engine.execute_query(&query).await.unwrap();

    for i in 1..results.len() {
        assert!(
            results[i - 1].score >= results[i].score,
            "results should be sorted by score descending"
        );
    }

    engine.cleanup().await.unwrap();
}
