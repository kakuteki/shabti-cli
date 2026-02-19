use uuid::Uuid;

/// A search result from the WASM-compatible store.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub id: String,
    pub content: String,
    pub namespace: String,
    pub score: f32,
}

/// Trait for WASM-compatible memory stores.
pub trait WasmMemoryStore {
    fn store(&mut self, content: &str, embedding: &[f32], namespace: &str) -> String;
    fn get(&self, id: &str) -> Option<&StoredEntry>;
    fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        namespace: Option<&str>,
    ) -> Vec<SearchHit>;
    fn delete(&mut self, id: &str) -> bool;
    fn count(&self) -> usize;
}

/// A stored entry in the in-memory store.
#[derive(Debug, Clone)]
pub struct StoredEntry {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub namespace: String,
}

/// In-memory store with brute-force cosine similarity search.
/// Suitable for WASM/browser environments without Qdrant.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    entries: Vec<StoredEntry>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WasmMemoryStore for InMemoryStore {
    fn store(&mut self, content: &str, embedding: &[f32], namespace: &str) -> String {
        let id = Uuid::new_v4().to_string();
        self.entries.push(StoredEntry {
            id: id.clone(),
            content: content.to_string(),
            embedding: embedding.to_vec(),
            namespace: namespace.to_string(),
        });
        id
    }

    fn get(&self, id: &str) -> Option<&StoredEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        namespace: Option<&str>,
    ) -> Vec<SearchHit> {
        let mut scored: Vec<SearchHit> = self
            .entries
            .iter()
            .filter(|e| namespace.is_none_or(|ns| e.namespace == ns))
            .map(|e| SearchHit {
                id: e.id.clone(),
                content: e.content.clone(),
                namespace: e.namespace.clone(),
                score: cosine_similarity(query_embedding, &e.embedding),
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);
        scored
    }

    fn delete(&mut self, id: &str) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() < len_before
    }

    fn count(&self) -> usize {
        self.entries.len()
    }
}

/// Compute cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if mag_a < f32::EPSILON || mag_b < f32::EPSILON {
        return 0.0;
    }

    dot / (mag_a * mag_b)
}
