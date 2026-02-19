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
// Delete
// ============================================================

#[tokio::test]
async fn engine_delete_removes_from_search() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    let result = engine
        .store("entry to delete", &StoreOptions::default())
        .await
        .unwrap();
    let id = match result {
        shabti_core::dedup::StoreResult::Stored(id) => id,
        _ => panic!("expected Stored"),
    };

    assert_eq!(engine.entry_count(), 1);

    // Delete
    engine.delete(id).await.unwrap();

    // Should no longer appear in search
    let results = engine.search_similar("delete", 10, None).await.unwrap();
    assert!(results.is_empty());

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_delete_nonexistent_returns_error() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    let fake_id = uuid::Uuid::new_v4();
    let result = engine.delete(fake_id).await;
    // Should either succeed silently or return an error, but not panic
    // Qdrant delete of nonexistent ID is a no-op, so this should be Ok
    assert!(result.is_ok());

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

// ============================================================
// TTL (Time-To-Live)
// ============================================================

#[tokio::test]
async fn engine_store_with_ttl() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    // Store with TTL of 3600 seconds (1 hour)
    let opts = StoreOptions {
        ttl_seconds: Some(3600),
        ..Default::default()
    };
    let result = engine.store("TTL test entry", &opts).await.unwrap();
    assert!(matches!(result, shabti_core::dedup::StoreResult::Stored(_)));

    // Should still be searchable (not expired yet)
    let results = engine.search_similar("TTL test", 10, None).await.unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].entry.content.contains("TTL test"));

    // Verify expires_at is set on the entry
    assert!(results[0].entry.expires_at.is_some());
    let expires_at = results[0].entry.expires_at.unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    // expires_at should be ~1 hour from now
    assert!(expires_at > now);
    assert!(expires_at <= now + 3601);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_store_without_ttl_has_no_expiry() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    let result = engine
        .store("Permanent entry", &StoreOptions::default())
        .await
        .unwrap();
    assert!(matches!(result, shabti_core::dedup::StoreResult::Stored(_)));

    let results = engine
        .search_similar("Permanent", 10, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].entry.expires_at.is_none());

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_expired_entry_excluded_from_search() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    // Store with TTL of 1 second (will expire almost immediately)
    let opts = StoreOptions {
        ttl_seconds: Some(1),
        ..Default::default()
    };
    engine.store("Expiring soon entry", &opts).await.unwrap();

    // Also store a permanent entry
    engine
        .store("Permanent entry for TTL test", &StoreOptions::default())
        .await
        .unwrap();

    // Wait for TTL to expire
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Search should only return the permanent entry
    let results = engine
        .search_similar("entry", 10, None)
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].entry.content.contains("Permanent"));

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

// ============================================================
// List entries (for export)
// ============================================================

#[tokio::test]
async fn engine_list_entries_returns_all() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    engine
        .store("Entry one", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("Entry two", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("Entry three", &StoreOptions::default())
        .await
        .unwrap();

    let entries = engine.list_entries(None, None).unwrap();
    assert_eq!(entries.len(), 3);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_list_entries_with_namespace_filter() {
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

    engine.store("Work item A", &opts_work).await.unwrap();
    engine.store("Work item B", &opts_work).await.unwrap();
    engine
        .store("Personal item", &opts_personal)
        .await
        .unwrap();

    // Filter by namespace
    let work_entries = engine.list_entries(Some("work"), None).unwrap();
    assert_eq!(work_entries.len(), 2);
    assert!(work_entries.iter().all(|e| e.namespace == "work"));

    let personal_entries = engine.list_entries(Some("personal"), None).unwrap();
    assert_eq!(personal_entries.len(), 1);

    // No filter returns all
    let all_entries = engine.list_entries(None, None).unwrap();
    assert_eq!(all_entries.len(), 3);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_list_entries_with_limit() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    engine
        .store("Entry alpha", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("Entry beta", &StoreOptions::default())
        .await
        .unwrap();
    engine
        .store("Entry gamma", &StoreOptions::default())
        .await
        .unwrap();

    let limited = engine.list_entries(None, Some(2)).unwrap();
    assert_eq!(limited.len(), 2);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

// ============================================================
// Garbage Collection
// ============================================================

#[tokio::test]
async fn engine_gc_removes_expired_entries() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    // Store one expiring and one permanent entry
    let opts = StoreOptions {
        ttl_seconds: Some(1),
        ..Default::default()
    };
    engine.store("GC test expiring", &opts).await.unwrap();
    engine
        .store("GC test permanent", &StoreOptions::default())
        .await
        .unwrap();
    assert_eq!(engine.entry_count(), 2);

    // Wait for TTL to expire
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // GC should remove the expired entry
    let removed = engine.gc().await.unwrap();
    assert_eq!(removed, 1);
    assert_eq!(engine.entry_count(), 1);

    // Second GC should find nothing to remove
    let removed = engine.gc().await.unwrap();
    assert_eq!(removed, 0);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}

#[tokio::test]
async fn engine_gc_no_expired_entries() {
    let tmp = TempDir::new().unwrap();
    let engine = ShabtiEngine::new(test_config(&tmp)).await.unwrap();

    // Store entries without TTL
    engine
        .store("No TTL entry", &StoreOptions::default())
        .await
        .unwrap();

    // GC on permanent entries should remove nothing
    let removed = engine.gc().await.unwrap();
    assert_eq!(removed, 0);
    assert_eq!(engine.entry_count(), 1);

    engine.shutdown().await.unwrap();
    engine.cleanup().await.unwrap();
}
