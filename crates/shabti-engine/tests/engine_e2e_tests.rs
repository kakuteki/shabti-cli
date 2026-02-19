//! E2E tests for ShabtiEngine with real Qdrant + fastembed.
//! Requires Qdrant running at localhost:6333.

use shabti_engine::{EngineConfig, ShabtiEngine, StoreOptions};
use tempfile::TempDir;

fn test_config(tmp: &TempDir) -> EngineConfig {
    EngineConfig {
        qdrant_url: "http://localhost:6334".to_string(),
        collection_name: format!("shabti_test_{}", uuid::Uuid::new_v4().simple()),
        data_dir: tmp.path().to_path_buf(),
        vector_size: 384, // multilingual-e5-small dimension
    }
}

#[tokio::test]
async fn engine_init_and_status() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    assert_eq!(engine.entry_count(), 0);
    assert_eq!(engine.current_model_id(), "MultilingualE5Small");

    let gate = engine.feature_gate();
    assert!(!gate.knn_links); // 0 entries → no knn

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_store_and_search() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    // Store a memory
    let result = engine
        .store(
            "Rust is a systems programming language",
            &StoreOptions::default(),
        )
        .await
        .unwrap();

    assert!(matches!(result, shabti_core::dedup::StoreResult::Stored(_)));
    assert_eq!(engine.entry_count(), 1);

    // Search for it
    let results = engine
        .search_similar("systems programming", 5, None)
        .await
        .unwrap();
    assert!(!results.is_empty());
    assert!(results[0].entry.content.contains("Rust"));
    assert!(results[0].score > 0.5);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_dedup() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    let content = "Duplicate content test";

    // First store succeeds
    let r1 = engine
        .store(content, &StoreOptions::default())
        .await
        .unwrap();
    assert!(matches!(r1, shabti_core::dedup::StoreResult::Stored(_)));

    // Second store is skipped (same content hash)
    let r2 = engine
        .store(content, &StoreOptions::default())
        .await
        .unwrap();
    assert!(matches!(
        r2,
        shabti_core::dedup::StoreResult::Skipped { .. }
    ));

    assert_eq!(engine.entry_count(), 1);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_store_with_namespace() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    let opts_work = StoreOptions {
        namespace: Some("work".to_string()),
        ..Default::default()
    };
    let opts_personal = StoreOptions {
        namespace: Some("personal".to_string()),
        ..Default::default()
    };

    engine
        .store("Meeting at 3pm with team", &opts_work)
        .await
        .unwrap();
    engine
        .store("Buy groceries after work", &opts_personal)
        .await
        .unwrap();

    // Search in work namespace only
    let results = engine
        .search_similar("meeting", 10, Some("work"))
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].entry.content.contains("Meeting"));

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_execute_query_with_explanation() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    engine
        .store("Tokyo is the capital of Japan", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store(
            "Python is a popular programming language",
            &StoreOptions::default(),
        )
        .await
        .unwrap();

    let query = shabti_core::query::QueryBuilder::new("capital of Japan")
        .limit(5)
        .with_explanation()
        .build();

    let results = engine.execute_query(&query).await.unwrap();
    assert!(!results.is_empty());

    // First result should be about Tokyo
    assert!(results[0].entry.content.contains("Tokyo"));

    // Should have explanation
    let exp = results[0].explanation.as_ref().unwrap();
    assert!(exp.semantic_similarity > 0.0);
    assert!(exp.time_decay_factor > 0.0);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_multiple_entries_ranking() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    engine
        .store("Cats are domestic animals", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("Dogs are loyal pets", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store(
            "Quantum physics describes subatomic particles",
            &StoreOptions::default(),
        )
        .await
        .unwrap();

    let results = engine.search_similar("pet animals", 3, None).await.unwrap();
    assert_eq!(results.len(), 3);

    // Pet-related entries should rank higher than physics
    let top_contents: Vec<&str> = results.iter().map(|r| r.entry.content.as_str()).collect();
    let physics_rank = top_contents
        .iter()
        .position(|c| c.contains("Quantum"))
        .unwrap();
    assert_eq!(physics_rank, 2, "Physics should be ranked last");

    // Scores should be in descending order
    for w in results.windows(2) {
        assert!(w[0].score >= w[1].score);
    }

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

// ============================================================
// Graceful degradation & error handling
// ============================================================

#[tokio::test]
async fn engine_invalid_qdrant_url_returns_error() {
    let tmp = TempDir::new().unwrap();
    let config = EngineConfig {
        qdrant_url: "http://localhost:19999".to_string(), // no server here
        collection_name: format!("shabti_test_{}", uuid::Uuid::new_v4().simple()),
        data_dir: tmp.path().to_path_buf(),
        vector_size: 384,
    };

    let result = ShabtiEngine::new(config).await;
    assert!(result.is_err(), "should fail with unreachable Qdrant");
}

#[tokio::test]
async fn engine_search_empty_collection() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    // Search on empty collection should return empty vec, not error
    let results = engine.search_similar("anything", 10, None).await.unwrap();
    assert!(results.is_empty());

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_search_nonexistent_namespace() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    engine
        .store("test entry", &StoreOptions::default())
        .await
        .unwrap();

    // Search in non-existent namespace should return empty, not error
    let results = engine
        .search_similar("test", 10, Some("nonexistent"))
        .await
        .unwrap();
    assert!(results.is_empty());

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}
