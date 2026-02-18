use std::collections::HashMap;

use hdbscan::{DistanceMetric, Hdbscan, HdbscanHyperParams};

use crate::resolution::{aggregate_embeddings, normalize_l2, SessionSummary};

/// A topic cluster discovered by HDBSCAN.
#[derive(Debug, Clone)]
pub struct TopicCluster {
    /// Sequential cluster ID.
    pub id: usize,
    /// L2-normalized centroid embedding.
    pub centroid: Vec<f32>,
    /// Session IDs belonging to this cluster.
    pub session_ids: Vec<String>,
    /// Optional human-readable label.
    pub label: Option<String>,
}

/// Cluster session embeddings using HDBSCAN.
/// Returns topic clusters (noise points are excluded).
/// Empty input returns empty vec.
pub fn cluster_sessions(
    sessions: &[SessionSummary],
    min_cluster_size: usize,
) -> Vec<TopicCluster> {
    if sessions.len() < min_cluster_size {
        return Vec::new();
    }

    // L2-normalize embeddings so Euclidean distance approximates cosine distance
    let data: Vec<Vec<f64>> = sessions
        .iter()
        .map(|s| {
            let normed = normalize_l2(&s.embedding);
            normed.into_iter().map(|x| x as f64).collect()
        })
        .collect();

    let params = HdbscanHyperParams::builder()
        .min_cluster_size(min_cluster_size)
        .dist_metric(DistanceMetric::Euclidean)
        .build();

    let clusterer = Hdbscan::new(&data, params);
    let labels = match clusterer.cluster() {
        Ok(labels) => labels,
        Err(_) => return Vec::new(),
    };

    // Group sessions by cluster label (skip noise = -1)
    let mut clusters_map: HashMap<i32, Vec<usize>> = HashMap::new();
    for (i, &label) in labels.iter().enumerate() {
        if label >= 0 {
            clusters_map.entry(label).or_default().push(i);
        }
    }

    // Sort cluster labels for deterministic ordering
    let mut cluster_labels: Vec<i32> = clusters_map.keys().cloned().collect();
    cluster_labels.sort();

    cluster_labels
        .into_iter()
        .enumerate()
        .map(|(seq_id, label)| {
            let indices = &clusters_map[&label];
            let session_ids: Vec<String> = indices
                .iter()
                .map(|&i| sessions[i].session_id.clone())
                .collect();

            // Compute centroid as mean of cluster member embeddings
            let embeddings: Vec<Vec<f32>> =
                indices.iter().map(|&i| sessions[i].embedding.clone()).collect();
            let weights: Vec<f32> = vec![1.0; embeddings.len()];
            let centroid = aggregate_embeddings(&embeddings, &weights);

            TopicCluster {
                id: seq_id,
                centroid,
                session_ids,
                label: None,
            }
        })
        .collect()
}
