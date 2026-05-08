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
    /// Hashes currently being processed (reserved to prevent TOCTOU races).
    pending: HashSet<String>,
}

impl DedupChecker {
    pub fn new() -> Self {
        Self {
            hashes: HashSet::new(),
            hash_to_id: HashMap::new(),
            pending: HashSet::new(),
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

    /// Returns true if the hash is already committed **or** is currently being
    /// processed by another task (i.e. marked as pending).  Callers should use
    /// this instead of `contains` when deciding whether to proceed with an
    /// expensive operation such as embedding generation, so that a concurrent
    /// task that has already reserved the slot is visible.
    pub fn is_known(&self, content_hash: &str) -> bool {
        self.hashes.contains(content_hash) || self.pending.contains(content_hash)
    }

    /// Return the committed ID for a hash if it exists, or `None` if the hash
    /// is unknown or only pending.  Used together with `is_known` to
    /// distinguish the "already committed" case from the "in-flight" case.
    pub fn committed_id(&self, content_hash: &str) -> Option<Uuid> {
        self.hash_to_id.get(content_hash).copied()
    }

    /// Reserve `content_hash` as in-flight so that concurrent tasks see it via
    /// `is_known` and skip the duplicate work.  Must be paired with a call to
    /// `clear_pending` once the entry has been committed (or on failure).
    pub fn mark_pending(&mut self, content_hash: &str) {
        self.pending.insert(content_hash.to_string());
    }

    /// Remove the in-flight reservation set by `mark_pending`.
    pub fn clear_pending(&mut self, content_hash: &str) {
        self.pending.remove(content_hash);
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
