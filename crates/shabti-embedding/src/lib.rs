use std::sync::Mutex;

use fastembed::{EmbeddingModel as FEModel, InitOptions, TextEmbedding};
use shabti_core::error::{ShabtiError, ShabtiResult};
use shabti_core::traits::EmbeddingModel;

pub struct FastEmbedModel {
    model: Mutex<TextEmbedding>,
    model_id: String,
    dimension: usize,
}

impl FastEmbedModel {
    pub fn new() -> ShabtiResult<Self> {
        Self::with_model(FEModel::MultilingualE5Small)
    }

    pub fn with_model(model_type: FEModel) -> ShabtiResult<Self> {
        let model_id = format!("{model_type:?}");
        // 初回実行時のみ ~/.cache/fastembed/ にモデルをダウンロードします
        let mut model = TextEmbedding::try_new(
            InitOptions::new(model_type).with_show_download_progress(true), // false → true
        )
        .map_err(|e| ShabtiError::Embedding(e.to_string()))?;

        // Probe dimension by embedding a dummy text
        let probe = model
            .embed(vec!["probe"], None)
            .map_err(|e| ShabtiError::Embedding(e.to_string()))?;
        let dimension = probe.first().map(|v| v.len()).unwrap_or(0);

        Ok(Self {
            model: Mutex::new(model),
            model_id,
            dimension,
        })
    }
}

impl EmbeddingModel for FastEmbedModel {
    fn embed(&self, text: &str) -> ShabtiResult<Vec<f32>> {
        let mut model = self
            .model
            .lock()
            .map_err(|e| ShabtiError::Embedding(e.to_string()))?;
        let results = model
            .embed(vec![text], None)
            .map_err(|e| ShabtiError::Embedding(e.to_string()))?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| ShabtiError::Embedding("no embedding returned".into()))
    }

    fn embed_batch(&self, texts: &[&str]) -> ShabtiResult<Vec<Vec<f32>>> {
        let mut model = self
            .model
            .lock()
            .map_err(|e| ShabtiError::Embedding(e.to_string()))?;
        let owned: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
        model
            .embed(owned, None)
            .map_err(|e| ShabtiError::Embedding(e.to_string()))
    }

    fn model_id(&self) -> &str {
        &self.model_id
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}
