use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

/// Compute cosine similarity between two vectors.
/// Returns a value between -1.0 and 1.0.
///
/// Panics if vectors have different dimensions.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(
        a.len(),
        b.len(),
        "vectors must have same dimension: {} vs {}",
        a.len(),
        b.len()
    );
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Load a fastembed model with project defaults.
pub fn load_model(model: EmbeddingModel) -> anyhow::Result<TextEmbedding> {
    let options = InitOptions::new(model).with_show_download_progress(true);
    Ok(TextEmbedding::try_new(options)?)
}

/// Generate embeddings for a slice of texts.
pub fn embed_texts(
    model: &mut TextEmbedding,
    texts: &[&str],
) -> anyhow::Result<Vec<Vec<f32>>> {
    Ok(model.embed(texts.to_vec(), None)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "identical vectors should have similarity 1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            sim.abs() < 1e-6,
            "orthogonal vectors should have similarity 0.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - (-1.0)).abs() < 1e-6,
            "opposite vectors should have similarity -1.0, got {sim}"
        );
    }

    #[test]
    fn cosine_similarity_zero_vector_returns_zero() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0, "zero vector should yield similarity 0.0");
    }

    #[test]
    #[should_panic(expected = "dimension")]
    fn cosine_similarity_panics_on_different_dimensions() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        cosine_similarity(&a, &b);
    }

    #[test]
    fn cosine_similarity_high_dimensional() {
        let mut a = vec![0.0f32; 384];
        let mut b = vec![0.0f32; 384];
        a[0] = 1.0;
        a[1] = 0.5;
        b[0] = 1.0;
        b[1] = 0.5;
        let sim = cosine_similarity(&a, &b);
        assert!(
            (sim - 1.0).abs() < 1e-6,
            "identical sparse vectors should have similarity 1.0, got {sim}"
        );
    }
}
