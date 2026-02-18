use std::collections::HashMap;

use crate::event::cosine_similarity;

/// Detects near-duplicate memory entries based on embedding cosine similarity.
pub struct NearDuplicateDetector {
    threshold: f32,
    /// Representative embedding per group (group_id → embedding).
    representatives: Vec<Vec<f32>>,
}

impl NearDuplicateDetector {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold,
            representatives: Vec::new(),
        }
    }

    /// Check if two embeddings are near-duplicates (cosine similarity >= threshold).
    pub fn is_near_duplicate(&self, a: &[f32], b: &[f32]) -> bool {
        cosine_similarity(a, b) >= self.threshold
    }

    /// Assign an embedding to a group. Returns the group ID.
    /// Near-duplicate embeddings get the same group ID as an existing representative.
    /// New unique embeddings create a new group.
    pub fn assign_group(&mut self, embedding: &[f32]) -> usize {
        for (i, rep) in self.representatives.iter().enumerate() {
            if cosine_similarity(embedding, rep) >= self.threshold {
                return i;
            }
        }
        let new_id = self.representatives.len();
        self.representatives.push(embedding.to_vec());
        new_id
    }

    /// Number of distinct groups.
    pub fn group_count(&self) -> usize {
        self.representatives.len()
    }

    /// Deduplicate results by group, keeping the entry with highest access_count per group.
    /// Each entry is (group_label, score, access_count).
    pub fn deduplicate_by_group<'a>(
        &self,
        entries: &'a [(&'a str, f32, u32)],
    ) -> Vec<&'a (&'a str, f32, u32)> {
        let mut best_per_group: HashMap<&str, &'a (&'a str, f32, u32)> = HashMap::new();
        for entry in entries {
            let group = entry.0;
            match best_per_group.get(group) {
                Some(existing) if existing.2 >= entry.2 => {}
                _ => {
                    best_per_group.insert(group, entry);
                }
            }
        }
        best_per_group.into_values().collect()
    }
}

impl Default for NearDuplicateDetector {
    fn default() -> Self {
        Self::new(0.98)
    }
}
