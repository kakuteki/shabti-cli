use shabti_core::wasm_store::{InMemoryStore, WasmMemoryStore};

// --- 6-5: WASM target foundation (InMemoryStore) ---

#[test]
fn store_and_get() {
    let mut store = InMemoryStore::new();

    let id = store.store("Hello world", &[0.1, 0.2, 0.3], "default");
    let entry = store.get(&id);

    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.content, "Hello world");
    assert_eq!(entry.namespace, "default");
    assert_eq!(entry.embedding, vec![0.1, 0.2, 0.3]);
}

#[test]
fn get_nonexistent() {
    let store = InMemoryStore::new();
    assert!(store.get("nonexistent-id").is_none());
}

#[test]
fn search_by_cosine_similarity() {
    let mut store = InMemoryStore::new();

    store.store("Similar A", &[1.0, 0.0, 0.0], "default");
    store.store("Similar B", &[0.9, 0.1, 0.0], "default");
    store.store("Different", &[0.0, 0.0, 1.0], "default");

    let results = store.search(&[1.0, 0.0, 0.0], 2, None);

    assert_eq!(results.len(), 2);
    // First result should be most similar
    assert_eq!(results[0].content, "Similar A");
    assert!(results[0].score > results[1].score);
}

#[test]
fn search_with_namespace_filter() {
    let mut store = InMemoryStore::new();

    store.store("Work note", &[1.0, 0.0, 0.0], "work");
    store.store("Personal note", &[0.9, 0.1, 0.0], "personal");
    store.store("Work task", &[0.8, 0.2, 0.0], "work");

    let results = store.search(&[1.0, 0.0, 0.0], 10, Some("work"));
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.namespace == "work"));
}

#[test]
fn search_limit() {
    let mut store = InMemoryStore::new();

    for i in 0..10 {
        store.store(&format!("Entry {i}"), &[i as f32 / 10.0, 0.0, 0.0], "default");
    }

    let results = store.search(&[1.0, 0.0, 0.0], 3, None);
    assert_eq!(results.len(), 3);
}

#[test]
fn delete_entry() {
    let mut store = InMemoryStore::new();

    let id = store.store("To delete", &[1.0, 0.0, 0.0], "default");
    assert!(store.get(&id).is_some());

    assert!(store.delete(&id));
    assert!(store.get(&id).is_none());
}

#[test]
fn delete_nonexistent() {
    let mut store = InMemoryStore::new();
    assert!(!store.delete("nonexistent"));
}

#[test]
fn entry_count() {
    let mut store = InMemoryStore::new();
    assert_eq!(store.count(), 0);

    store.store("A", &[1.0], "default");
    store.store("B", &[0.5], "default");
    assert_eq!(store.count(), 2);

    let id = store.store("C", &[0.3], "default");
    store.delete(&id);
    assert_eq!(store.count(), 2);
}

#[test]
fn search_hit_fields() {
    let mut store = InMemoryStore::new();
    store.store("Test content", &[1.0, 0.0], "ns1");

    let results = store.search(&[1.0, 0.0], 1, None);
    assert_eq!(results.len(), 1);

    let hit = &results[0];
    assert_eq!(hit.content, "Test content");
    assert_eq!(hit.namespace, "ns1");
    assert!(!hit.id.is_empty());
    assert!(hit.score > 0.0);
}

#[test]
fn cosine_similarity_correctness() {
    let mut store = InMemoryStore::new();

    // Identical vectors should have score 1.0
    store.store("Exact match", &[1.0, 0.0, 0.0], "default");

    let results = store.search(&[1.0, 0.0, 0.0], 1, None);
    assert!((results[0].score - 1.0).abs() < 0.001);

    // Orthogonal vectors should have score 0.0
    store.store("Orthogonal", &[0.0, 1.0, 0.0], "default");

    let results = store.search(&[1.0, 0.0, 0.0], 2, None);
    let orthogonal = results.iter().find(|r| r.content == "Orthogonal").unwrap();
    assert!(orthogonal.score.abs() < 0.001);
}
