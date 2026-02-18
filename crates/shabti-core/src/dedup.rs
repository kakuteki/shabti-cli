use std::collections::{HashMap, HashSet};

use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreResult {
    Stored(Uuid),
    Skipped { existing_id: Uuid },
}

pub struct DedupChecker {
    hashes: HashSet<String>,
    hash_to_id: HashMap<String, Uuid>,
}

impl DedupChecker {
    pub fn new() -> Self {
        Self {
            hashes: HashSet::new(),
            hash_to_id: HashMap::new(),
        }
    }

    pub fn check_and_insert(&mut self, content_hash: &str, id: Uuid) -> StoreResult {
        if let Some(&existing_id) = self.hash_to_id.get(content_hash) {
            return StoreResult::Skipped { existing_id };
        }
        self.hashes.insert(content_hash.to_string());
        self.hash_to_id.insert(content_hash.to_string(), id);
        StoreResult::Stored(id)
    }

    pub fn contains(&self, content_hash: &str) -> bool {
        self.hashes.contains(content_hash)
    }

    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }
}

impl Default for DedupChecker {
    fn default() -> Self {
        Self::new()
    }
}
