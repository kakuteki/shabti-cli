use std::collections::HashMap;

use shabti_core::{LifecycleState, MemoryEntry, MemoryEntryBuilder, OriginType};
use uuid::Uuid;

#[test]
fn new_generates_non_nil_uuid() {
    let entry = MemoryEntry::new("Hello, world!".into(), vec![0.1; 384], "model-v1".into());
    assert_ne!(entry.id, Uuid::nil());
}

#[test]
fn new_generates_unique_uuids() {
    let e1 = MemoryEntry::new("a".into(), vec![0.1; 384], "m".into());
    let e2 = MemoryEntry::new("b".into(), vec![0.1; 384], "m".into());
    assert_ne!(e1.id, e2.id);
}

#[test]
fn new_computes_sha256_content_hash() {
    let entry = MemoryEntry::new("Hello, world!".into(), vec![0.1; 384], "m".into());
    assert!(!entry.content_hash.is_empty());
    // SHA-256 hex string is 64 characters
    assert_eq!(entry.content_hash.len(), 64);
    // Must be valid hex
    assert!(entry.content_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn same_content_same_hash() {
    let e1 = MemoryEntry::new("Hello".into(), vec![0.1; 384], "m".into());
    let e2 = MemoryEntry::new("Hello".into(), vec![0.9; 384], "other".into());
    assert_eq!(e1.content_hash, e2.content_hash);
}

#[test]
fn different_content_different_hash() {
    let e1 = MemoryEntry::new("Hello".into(), vec![0.1; 384], "m".into());
    let e2 = MemoryEntry::new("World".into(), vec![0.1; 384], "m".into());
    assert_ne!(e1.content_hash, e2.content_hash);
}

#[test]
fn whitespace_difference_produces_different_hash() {
    let e1 = MemoryEntry::new("hello world".into(), vec![0.1; 3], "m".into());
    let e2 = MemoryEntry::new("hello  world".into(), vec![0.1; 3], "m".into());
    assert_ne!(e1.content_hash, e2.content_hash);
}

#[test]
fn content_hash_of_matches_entry_hash() {
    let entry = MemoryEntry::new("test content".into(), vec![0.1; 3], "m".into());
    assert_eq!(entry.content_hash, MemoryEntry::content_hash_of("test content"));
}

#[test]
fn new_sets_correct_defaults() {
    let entry = MemoryEntry::new("test".into(), vec![0.1; 3], "m".into());
    assert_eq!(entry.access_count, 0);
    assert_eq!(entry.lifecycle_state, LifecycleState::Active);
    assert_eq!(entry.origin_type, OriginType::UserInput);
    assert!(entry.keywords.is_empty());
    assert!(entry.tags.is_empty());
    assert!(entry.event_id.is_none());
    assert!(entry.dedupe_group_id.is_none());
    assert!(entry.superseded_by.is_none());
    assert_eq!(entry.gap_before, 0.0);
    assert_eq!(entry.gap_after, 0.0);
    assert_eq!(entry.namespace, "default");
    assert_eq!(entry.session_id, "");
    assert!(entry.metadata.is_empty());
}

#[test]
fn new_sets_timestamps() {
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let entry = MemoryEntry::new("test".into(), vec![0.1; 3], "m".into());
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    assert!(entry.created_at >= before && entry.created_at <= after);
    assert!(entry.last_accessed >= before && entry.last_accessed <= after);
}

#[test]
fn new_preserves_content_and_embedding() {
    let content = "important memory".to_string();
    let embedding = vec![0.1, 0.2, 0.3];
    let model = "test-model".to_string();
    let entry = MemoryEntry::new(content.clone(), embedding.clone(), model.clone());

    assert_eq!(entry.content, content);
    assert_eq!(entry.embedding, embedding);
    assert_eq!(entry.model_id, model);
}

#[test]
fn builder_sets_all_optional_fields() {
    let mut meta = HashMap::new();
    meta.insert("key".to_string(), serde_json::json!("value"));

    let entry = MemoryEntryBuilder::new("test".into(), vec![0.1; 3], "m".into())
        .session_id("sess-1".into())
        .namespace("tenant-1".into())
        .keywords(vec!["kw1".into(), "kw2".into()])
        .tags(vec!["tag1".into()])
        .origin_type(OriginType::AgentGenerated)
        .gap_before(1.5)
        .gap_after(2.0)
        .metadata(meta.clone())
        .build();

    assert_eq!(entry.session_id, "sess-1");
    assert_eq!(entry.namespace, "tenant-1");
    assert_eq!(entry.keywords, vec!["kw1", "kw2"]);
    assert_eq!(entry.tags, vec!["tag1"]);
    assert_eq!(entry.origin_type, OriginType::AgentGenerated);
    assert_eq!(entry.gap_before, 1.5);
    assert_eq!(entry.gap_after, 2.0);
    assert_eq!(entry.metadata, meta);
}

#[test]
fn builder_defaults_match_new() {
    let from_new = MemoryEntry::new("test".into(), vec![0.1; 3], "m".into());
    let from_builder = MemoryEntryBuilder::new("test".into(), vec![0.1; 3], "m".into()).build();

    // Same defaults (can't compare IDs since they're different UUIDs)
    assert_eq!(from_new.namespace, from_builder.namespace);
    assert_eq!(from_new.lifecycle_state, from_builder.lifecycle_state);
    assert_eq!(from_new.origin_type, from_builder.origin_type);
    assert_eq!(from_new.access_count, from_builder.access_count);
}

#[test]
fn serde_json_roundtrip() {
    let entry = MemoryEntry::new("test content".into(), vec![0.1, 0.2, 0.3], "m".into());
    let json = serde_json::to_string(&entry).unwrap();
    let restored: MemoryEntry = serde_json::from_str(&json).unwrap();

    assert_eq!(entry.id, restored.id);
    assert_eq!(entry.content, restored.content);
    assert_eq!(entry.content_hash, restored.content_hash);
    assert_eq!(entry.embedding, restored.embedding);
    assert_eq!(entry.model_id, restored.model_id);
    assert_eq!(entry.created_at, restored.created_at);
    assert_eq!(entry.lifecycle_state, restored.lifecycle_state);
    assert_eq!(entry.origin_type, restored.origin_type);
    assert_eq!(entry.namespace, restored.namespace);
}

#[test]
fn lifecycle_state_serde_roundtrip() {
    for state in [
        LifecycleState::Active,
        LifecycleState::Merged,
        LifecycleState::Archived,
        LifecycleState::Superseded,
    ] {
        let json = serde_json::to_string(&state).unwrap();
        let restored: LifecycleState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, restored);
    }
}

#[test]
fn origin_type_serde_roundtrip() {
    for ot in [
        OriginType::UserInput,
        OriginType::AgentGenerated,
        OriginType::Imported,
    ] {
        let json = serde_json::to_string(&ot).unwrap();
        let restored: OriginType = serde_json::from_str(&json).unwrap();
        assert_eq!(ot, restored);
    }
}

#[test]
fn lifecycle_state_serializes_as_snake_case() {
    assert_eq!(
        serde_json::to_string(&LifecycleState::Active).unwrap(),
        "\"active\""
    );
    assert_eq!(
        serde_json::to_string(&OriginType::AgentGenerated).unwrap(),
        "\"agent_generated\""
    );
}
