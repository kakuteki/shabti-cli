use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::types::{LifecycleState, OriginType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub content: String,
    pub content_hash: String,
    pub embedding: Vec<f32>,
    pub model_id: String,
    pub created_at: i64,
    pub session_id: String,
    pub gap_before: f64,
    pub gap_after: f64,
    pub event_id: Option<String>,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,
    pub access_count: u32,
    pub last_accessed: i64,
    pub dedupe_group_id: Option<String>,
    pub superseded_by: Option<Uuid>,
    pub lifecycle_state: LifecycleState,
    pub origin_type: OriginType,
    pub namespace: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl MemoryEntry {
    pub fn new(content: String, embedding: Vec<f32>, model_id: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX epoch")
            .as_secs() as i64;
        let content_hash = Self::content_hash_of(&content);

        Self {
            id: Uuid::new_v4(),
            content,
            content_hash,
            embedding,
            model_id,
            created_at: now,
            session_id: String::new(),
            gap_before: 0.0,
            gap_after: 0.0,
            event_id: None,
            keywords: Vec::new(),
            tags: Vec::new(),
            access_count: 0,
            last_accessed: now,
            dedupe_group_id: None,
            superseded_by: None,
            lifecycle_state: LifecycleState::Active,
            origin_type: OriginType::UserInput,
            namespace: "default".to_string(),
            metadata: HashMap::new(),
        }
    }

    pub fn content_hash_of(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        hex_encode(&result)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub struct MemoryEntryBuilder {
    content: String,
    embedding: Vec<f32>,
    model_id: String,
    session_id: Option<String>,
    namespace: Option<String>,
    keywords: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    origin_type: Option<OriginType>,
    gap_before: Option<f64>,
    gap_after: Option<f64>,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

impl MemoryEntryBuilder {
    pub fn new(content: String, embedding: Vec<f32>, model_id: String) -> Self {
        Self {
            content,
            embedding,
            model_id,
            session_id: None,
            namespace: None,
            keywords: None,
            tags: None,
            origin_type: None,
            gap_before: None,
            gap_after: None,
            metadata: None,
        }
    }

    pub fn session_id(mut self, id: String) -> Self {
        self.session_id = Some(id);
        self
    }

    pub fn namespace(mut self, ns: String) -> Self {
        self.namespace = Some(ns);
        self
    }

    pub fn keywords(mut self, kw: Vec<String>) -> Self {
        self.keywords = Some(kw);
        self
    }

    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn origin_type(mut self, ot: OriginType) -> Self {
        self.origin_type = Some(ot);
        self
    }

    pub fn gap_before(mut self, gap: f64) -> Self {
        self.gap_before = Some(gap);
        self
    }

    pub fn gap_after(mut self, gap: f64) -> Self {
        self.gap_after = Some(gap);
        self
    }

    pub fn metadata(mut self, meta: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(meta);
        self
    }

    pub fn build(self) -> MemoryEntry {
        let mut entry = MemoryEntry::new(self.content, self.embedding, self.model_id);

        if let Some(v) = self.session_id {
            entry.session_id = v;
        }
        if let Some(v) = self.namespace {
            entry.namespace = v;
        }
        if let Some(v) = self.keywords {
            entry.keywords = v;
        }
        if let Some(v) = self.tags {
            entry.tags = v;
        }
        if let Some(v) = self.origin_type {
            entry.origin_type = v;
        }
        if let Some(v) = self.gap_before {
            entry.gap_before = v;
        }
        if let Some(v) = self.gap_after {
            entry.gap_after = v;
        }
        if let Some(v) = self.metadata {
            entry.metadata = v;
        }

        entry
    }
}
