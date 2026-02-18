#[macro_use]
extern crate napi_derive;

/// 0-7b: Basic function call (Rust → Node.js)
#[napi]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// 0-7c: Struct passing (Rust struct → JS object)
#[napi(object)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub created_at: i64,
}

/// Create a MemoryEntry from parameters.
#[napi]
pub fn create_memory_entry(id: String, content: String, score: f64) -> MemoryEntry {
    MemoryEntry {
        id,
        content,
        score,
        created_at: 1_700_000_000,
    }
}

/// Accept a MemoryEntry and return a summary string.
#[napi]
pub fn summarize_entry(entry: MemoryEntry) -> String {
    format!(
        "[{}] {} (score: {:.2})",
        entry.id, entry.content, entry.score
    )
}

/// 0-7d: Async function (Rust async → Node.js Promise)
#[napi]
pub async fn compute_similarity(a: Vec<f64>, b: Vec<f64>) -> napi::Result<f64> {
    if a.len() != b.len() {
        return Err(napi::Error::from_reason(format!(
            "dimension mismatch: {} vs {}",
            a.len(),
            b.len()
        )));
    }
    // Simulate async work (cosine similarity computation)
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return Ok(0.0);
    }
    Ok(dot / (norm_a * norm_b))
}

/// Return multiple memory entries (Vec<Struct> → JS Array<Object>)
#[napi]
pub fn create_batch_entries(count: i32) -> Vec<MemoryEntry> {
    (0..count)
        .map(|i| MemoryEntry {
            id: format!("entry-{i}"),
            content: format!("Memory content #{i}"),
            score: 0.5 + (i as f64) * 0.01,
            created_at: 1_700_000_000 + i as i64 * 3600,
        })
        .collect()
}
