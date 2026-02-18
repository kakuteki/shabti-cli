use crate::error::ShabtiResult;

pub trait EmbeddingModel: Send + Sync {
    fn embed(&self, text: &str) -> ShabtiResult<Vec<f32>>;
    fn embed_batch(&self, texts: &[&str]) -> ShabtiResult<Vec<Vec<f32>>>;
    fn model_id(&self) -> &str;
    fn dimension(&self) -> usize;
}
