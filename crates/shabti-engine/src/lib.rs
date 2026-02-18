use std::path::PathBuf;
use std::sync::Mutex;

use shabti_core::dedup::{DedupChecker, StoreResult};
use shabti_core::entry::{MemoryEntry, MemoryEntryBuilder};
use shabti_core::error::{ShabtiError, ShabtiResult};
use shabti_core::gate::FeatureGate;
use shabti_core::traits::EmbeddingModel;
use shabti_core::types::OriginType;
use shabti_embedding::FastEmbedModel;
use shabti_index::{IndexConfig, QdrantIndex, SearchOptions, TimeRange};
use uuid::Uuid;

pub struct EngineConfig {
    pub qdrant_url: String,
    pub collection_name: String,
    pub data_dir: PathBuf,
    pub vector_size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct StoreOptions {
    pub namespace: Option<String>,
    pub session_id: Option<String>,
    pub origin_type: Option<OriginType>,
    pub keywords: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: MemoryEntry,
    pub score: f32,
}

pub struct ShabtiEngine {
    embedding: FastEmbedModel,
    index: QdrantIndex,
    log: Mutex<shabti_storage::AppendLog>,
    dedup: Mutex<DedupChecker>,
    entry_count: Mutex<usize>,
}

impl ShabtiEngine {
    pub async fn new(config: EngineConfig) -> ShabtiResult<Self> {
        let embedding = FastEmbedModel::new()?;
        let vector_size = config.vector_size;

        let index = QdrantIndex::new(IndexConfig {
            url: config.qdrant_url,
            collection_name: config.collection_name,
            vector_size,
        })
        .await?;

        let log_path = config.data_dir.join("shabti.log");
        let log = shabti_storage::AppendLog::open(&log_path)?;

        // Rebuild dedup checker from existing log entries
        let mut dedup = DedupChecker::new();
        let mut count = 0;
        for entry in log.iter() {
            dedup.check_and_insert(&entry.content_hash, entry.id);
            count += 1;
        }

        Ok(Self {
            embedding,
            index,
            log: Mutex::new(log),
            dedup: Mutex::new(dedup),
            entry_count: Mutex::new(count),
        })
    }

    pub async fn store(&self, content: &str, options: &StoreOptions) -> ShabtiResult<StoreResult> {
        // Check dedup first (single lock acquisition to avoid deadlock)
        let content_hash = MemoryEntry::content_hash_of(content);
        {
            let mut dedup = self.dedup.lock().map_err(|e| ShabtiError::Storage(e.to_string()))?;
            if dedup.contains(&content_hash) {
                let result = dedup.check_and_insert(&content_hash, Uuid::new_v4());
                return Ok(result);
            }
        }

        // Generate embedding
        let embedding = self.embedding.embed(content)?;

        // Build entry
        let mut builder = MemoryEntryBuilder::new(
            content.to_string(),
            embedding,
            self.embedding.model_id().to_string(),
        );

        if let Some(ref ns) = options.namespace {
            builder = builder.namespace(ns.clone());
        }
        if let Some(ref sid) = options.session_id {
            builder = builder.session_id(sid.clone());
        }
        if let Some(ref ot) = options.origin_type {
            builder = builder.origin_type(*ot);
        }
        if let Some(ref kw) = options.keywords {
            builder = builder.keywords(kw.clone());
        }
        if let Some(ref tags) = options.tags {
            builder = builder.tags(tags.clone());
        }

        let entry = builder.build();
        let id = entry.id;

        // Register in dedup
        {
            let mut dedup = self.dedup.lock().map_err(|e| ShabtiError::Storage(e.to_string()))?;
            dedup.check_and_insert(&entry.content_hash, id);
        }

        // Write to append log
        {
            let mut log = self.log.lock().map_err(|e| ShabtiError::Storage(e.to_string()))?;
            log.append(&entry)?;
        }

        // Insert into Qdrant
        self.index.insert(&entry).await?;

        // Update count
        {
            let mut count = self.entry_count.lock().map_err(|e| ShabtiError::Storage(e.to_string()))?;
            *count += 1;
        }

        Ok(StoreResult::Stored(id))
    }

    pub async fn get(&self, id: Uuid) -> ShabtiResult<MemoryEntry> {
        let log = self.log.lock().map_err(|e| ShabtiError::Storage(e.to_string()))?;
        log.read(id)
    }

    pub async fn search_similar(
        &self,
        query: &str,
        limit: usize,
        namespace: Option<&str>,
    ) -> ShabtiResult<Vec<SearchResult>> {
        let query_embedding = self.embedding.embed(query)?;

        let opts = SearchOptions {
            namespace: namespace.map(|s| s.to_string()),
            ..Default::default()
        };

        let hits = self.index.search(&query_embedding, limit, &opts).await?;

        let log = self.log.lock().map_err(|e| ShabtiError::Storage(e.to_string()))?;
        let mut results = Vec::new();
        for hit in hits {
            if let Ok(entry) = log.read(hit.id) {
                results.push(SearchResult {
                    entry,
                    score: hit.score,
                });
            }
        }

        Ok(results)
    }

    pub async fn search_by_time(
        &self,
        range: &TimeRange,
        limit: usize,
    ) -> ShabtiResult<Vec<MemoryEntry>> {
        let hits = self.index.search_by_time(range, limit).await?;

        let log = self.log.lock().map_err(|e| ShabtiError::Storage(e.to_string()))?;
        let mut results = Vec::new();
        for hit in hits {
            if let Ok(entry) = log.read(hit.id) {
                results.push(entry);
            }
        }

        Ok(results)
    }

    pub fn entry_count(&self) -> usize {
        *self.entry_count.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn feature_gate(&self) -> FeatureGate {
        FeatureGate::from_count(self.entry_count())
    }

    pub async fn cleanup(&self) -> ShabtiResult<()> {
        self.index.delete_collection().await
    }
}
