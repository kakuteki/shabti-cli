//! Multi-resolution embedding aggregation (Level 0→1→2).

/// L2-normalize a vector. Returns zero vector if input is zero.
pub fn normalize_l2(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

/// Compute weighted average of embeddings, then L2-normalize.
/// Returns empty vec if inputs are empty.
pub fn aggregate_embeddings(embeddings: &[Vec<f32>], weights: &[f32]) -> Vec<f32> {
    if embeddings.is_empty() {
        return Vec::new();
    }

    let dim = embeddings[0].len();
    let mut weighted_sum = vec![0.0f32; dim];
    let mut weight_total = 0.0f32;

    for (emb, &w) in embeddings.iter().zip(weights.iter()) {
        for (i, &val) in emb.iter().enumerate() {
            weighted_sum[i] += val * w;
        }
        weight_total += w;
    }

    if weight_total > 0.0 {
        for val in &mut weighted_sum {
            *val /= weight_total;
        }
    }

    normalize_l2(&weighted_sum)
}

/// Summary embedding for a session (Level 2).
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub embedding: Vec<f32>,
    pub event_count: usize,
}

impl SessionSummary {
    /// Create a session summary from event-level embeddings.
    /// Uses equal weights for all events.
    pub fn from_event_embeddings(session_id: String, event_embeddings: &[Vec<f32>]) -> Self {
        if event_embeddings.is_empty() {
            return Self {
                session_id,
                embedding: Vec::new(),
                event_count: 0,
            };
        }

        let weights: Vec<f32> = vec![1.0; event_embeddings.len()];
        let embedding = aggregate_embeddings(event_embeddings, &weights);

        Self {
            session_id,
            embedding,
            event_count: event_embeddings.len(),
        }
    }
}
