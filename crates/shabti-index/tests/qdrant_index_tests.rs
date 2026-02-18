use shabti_core::entry::MemoryEntry;
use shabti_core::types::LifecycleState;
use shabti_index::{IndexConfig, QdrantIndex, SearchOptions, TimeRange};
use uuid::Uuid;

fn test_config(collection: &str) -> IndexConfig {
    IndexConfig {
        url: "http://localhost:6334".to_string(),
        collection_name: collection.to_string(),
        vector_size: 3,
    }
}

fn make_entry(content: &str, embedding: Vec<f32>) -> MemoryEntry {
    MemoryEntry::new(content.to_string(), embedding, "test-model".to_string())
}

fn make_entry_with_time(content: &str, embedding: Vec<f32>, created_at: i64) -> MemoryEntry {
    let mut entry = MemoryEntry::new(content.to_string(), embedding, "test-model".to_string());
    entry.created_at = created_at;
    entry
}

fn unique_collection() -> String {
    format!("test_{}", Uuid::new_v4().to_string().replace('-', ""))
}

#[tokio::test]
async fn insert_and_search_basic() {
    let col = unique_collection();
    let config = test_config(&col);
    let index = QdrantIndex::new(config).await.unwrap();

    let entry = make_entry("hello world", vec![1.0, 0.0, 0.0]);
    index.insert(&entry).await.unwrap();

    let results = index
        .search(&[1.0, 0.0, 0.0], 10, &SearchOptions::default())
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, entry.id);
    assert!(results[0].score > 0.9);

    index.delete_collection().await.unwrap();
}

#[tokio::test]
async fn insert_multiple_and_search_top_k() {
    let col = unique_collection();
    let index = QdrantIndex::new(test_config(&col)).await.unwrap();

    let e1 = make_entry("cat", vec![1.0, 0.0, 0.0]);
    let e2 = make_entry("dog", vec![0.9, 0.1, 0.0]);
    let e3 = make_entry("car", vec![0.0, 0.0, 1.0]);

    index.insert(&e1).await.unwrap();
    index.insert(&e2).await.unwrap();
    index.insert(&e3).await.unwrap();

    let results = index
        .search(&[1.0, 0.0, 0.0], 2, &SearchOptions::default())
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    // cat and dog should be most similar to [1,0,0]
    let ids: Vec<Uuid> = results.iter().map(|r| r.id).collect();
    assert!(ids.contains(&e1.id));
    assert!(ids.contains(&e2.id));

    index.delete_collection().await.unwrap();
}

#[tokio::test]
async fn delete_removes_entry() {
    let col = unique_collection();
    let index = QdrantIndex::new(test_config(&col)).await.unwrap();

    let entry = make_entry("to delete", vec![1.0, 0.0, 0.0]);
    index.insert(&entry).await.unwrap();
    index.delete(entry.id).await.unwrap();

    let results = index
        .search(&[1.0, 0.0, 0.0], 10, &SearchOptions::default())
        .await
        .unwrap();
    assert!(results.is_empty());

    index.delete_collection().await.unwrap();
}

#[tokio::test]
async fn update_payload_modifies_metadata() {
    let col = unique_collection();
    let index = QdrantIndex::new(test_config(&col)).await.unwrap();

    let entry = make_entry("updateable", vec![1.0, 0.0, 0.0]);
    let id = entry.id;
    index.insert(&entry).await.unwrap();

    index.update_access(id, 5, 1700000000).await.unwrap();

    let results = index
        .search(&[1.0, 0.0, 0.0], 1, &SearchOptions::default())
        .await
        .unwrap();
    assert_eq!(results[0].access_count, 5);
    assert_eq!(results[0].last_accessed, 1700000000);

    index.delete_collection().await.unwrap();
}

#[tokio::test]
async fn search_with_namespace_filter() {
    let col = unique_collection();
    let index = QdrantIndex::new(test_config(&col)).await.unwrap();

    let mut e1 = make_entry("private", vec![1.0, 0.0, 0.0]);
    e1.namespace = "tenant-a".to_string();
    let mut e2 = make_entry("other", vec![0.9, 0.1, 0.0]);
    e2.namespace = "tenant-b".to_string();

    index.insert(&e1).await.unwrap();
    index.insert(&e2).await.unwrap();

    let opts = SearchOptions {
        namespace: Some("tenant-a".to_string()),
        ..Default::default()
    };
    let results = index.search(&[1.0, 0.0, 0.0], 10, &opts).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, e1.id);

    index.delete_collection().await.unwrap();
}

#[tokio::test]
async fn search_with_time_range() {
    let col = unique_collection();
    let index = QdrantIndex::new(test_config(&col)).await.unwrap();

    let e1 = make_entry_with_time("old", vec![1.0, 0.0, 0.0], 1000);
    let e2 = make_entry_with_time("new", vec![0.9, 0.1, 0.0], 2000);

    index.insert(&e1).await.unwrap();
    index.insert(&e2).await.unwrap();

    let opts = SearchOptions {
        time_range: Some(TimeRange {
            start: Some(1500),
            end: None,
        }),
        ..Default::default()
    };
    let results = index.search(&[1.0, 0.0, 0.0], 10, &opts).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, e2.id);

    index.delete_collection().await.unwrap();
}

#[tokio::test]
async fn search_excludes_non_active_lifecycle() {
    let col = unique_collection();
    let index = QdrantIndex::new(test_config(&col)).await.unwrap();

    let e1 = make_entry("active", vec![1.0, 0.0, 0.0]);
    let mut e2 = make_entry("archived", vec![0.9, 0.1, 0.0]);
    e2.lifecycle_state = LifecycleState::Archived;

    index.insert(&e1).await.unwrap();
    index.insert(&e2).await.unwrap();

    // Default search only returns Active entries
    let results = index
        .search(&[1.0, 0.0, 0.0], 10, &SearchOptions::default())
        .await
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, e1.id);

    index.delete_collection().await.unwrap();
}

#[tokio::test]
async fn search_by_time_without_vector() {
    let col = unique_collection();
    let index = QdrantIndex::new(test_config(&col)).await.unwrap();

    let e1 = make_entry_with_time("jan", vec![1.0, 0.0, 0.0], 1704067200); // 2024-01-01
    let e2 = make_entry_with_time("feb", vec![0.0, 1.0, 0.0], 1706745600); // 2024-02-01
    let e3 = make_entry_with_time("mar", vec![0.0, 0.0, 1.0], 1709251200); // 2024-03-01

    index.insert(&e1).await.unwrap();
    index.insert(&e2).await.unwrap();
    index.insert(&e3).await.unwrap();

    let range = TimeRange {
        start: Some(1706000000),
        end: Some(1710000000),
    };
    let results = index.search_by_time(&range, 10).await.unwrap();
    assert_eq!(results.len(), 2);

    index.delete_collection().await.unwrap();
}
