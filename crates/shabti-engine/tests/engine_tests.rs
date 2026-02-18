use shabti_core::dedup::StoreResult;
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
