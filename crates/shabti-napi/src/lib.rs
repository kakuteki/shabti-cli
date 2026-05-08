#[macro_use]
extern crate napi_derive;

use std::path::PathBuf;
use std::sync::Arc;

use napi::bindgen_prelude::*;
use shabti_core::contradiction::detect_contradiction;
use shabti_core::dedup::StoreResult;
use shabti_core::query::QueryBuilder;
use shabti_engine::{EngineConfig, ShabtiEngine, StoreOptions};
use shabti_storage::snapshot;
use tokio::runtime::Runtime;

// ============================================================
// Input/Output types for napi
// ============================================================

#[napi(object)]
#[derive(Clone)]
pub struct NapiEngineConfig {
    pub qdrant_url: String,
    pub collection_name: Option<String>,
    pub data_dir: String,
}

#[napi(object)]
#[derive(Clone, Default)]
pub struct NapiStoreOptions {
    pub namespace: Option<String>,
    pub session_id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub ttl_seconds: Option<u32>,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiStoreResult {
    pub status: String,
    pub id: Option<String>,
    pub existing_id: Option<String>,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiSearchResult {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub namespace: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiExplanation {
    pub semantic_similarity: f64,
    pub time_decay_factor: f64,
    pub access_boost_factor: f64,
    pub matched_at_level: u32,
    pub link_hops: u32,
    pub is_superseded: bool,
    pub dedupe_count: u32,
    pub final_score: f64,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiQueryResult {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub namespace: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub explanation: Option<NapiExplanation>,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiQuery {
    pub text: String,
    pub limit: Option<u32>,
    pub namespace: Option<String>,
    pub time_start: Option<i64>,
    pub time_end: Option<i64>,
    pub cluster_id: Option<u32>,
    pub max_hops: Option<u32>,
    pub min_score: Option<f64>,
    pub exclude_superseded: Option<bool>,
    pub with_explanation: Option<bool>,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiSnapshot {
    pub id: String,
    pub created_at: i64,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiExportEntry {
    pub id: String,
    pub content: String,
    pub namespace: String,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub session_id: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub lifecycle_state: String,
    pub origin_type: String,
    pub model_id: String,
    pub embedding: Option<Vec<f64>>,
    pub metadata: Option<String>,
}

#[napi(object)]
#[derive(Clone, Default)]
pub struct NapiListOptions {
    pub namespace: Option<String>,
    pub limit: Option<u32>,
    pub include_embeddings: Option<bool>,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiStatus {
    pub entry_count: u32,
    pub tier: String,
    pub qdrant_url: String,
    pub data_dir: String,
    pub model_id: String,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiContradictionResult {
    pub detected: bool,
    pub contradiction_type: Option<String>,
}

#[napi(object)]
#[derive(Clone)]
pub struct NapiGraphInfo {
    pub node_count: u32,
    pub edge_count: u32,
}

/// Detect contradiction between two text strings.
/// Returns whether a contradiction was found and its type (NumericDifference or NegationDifference).
#[napi]
pub fn detect_contradiction_napi(text_a: String, text_b: String) -> NapiContradictionResult {
    match detect_contradiction(&text_a, &text_b) {
        Some(ct) => NapiContradictionResult {
            detected: true,
            contradiction_type: Some(format!("{ct:?}")),
        },
        None => NapiContradictionResult {
            detected: false,
            contradiction_type: None,
        },
    }
}

// ============================================================
// ShabtiNapi — main binding class
// ============================================================

const MAX_CONTENT_CHARS: usize = 1_048_576; // 1MB

#[napi(js_name = "ShabtiEngine")]
pub struct ShabtiNapi {
    engine: Arc<ShabtiEngine>,
    /// Keep runtime alive for the lifetime of the engine.
    _runtime: Arc<Runtime>,
    data_dir: PathBuf,
    snapshots_dir: PathBuf,
    qdrant_url: String,
}

#[napi]
impl ShabtiNapi {
    #[napi(constructor)]
    pub fn new(config: NapiEngineConfig) -> Result<Self> {
        let data_dir = PathBuf::from(&config.data_dir);
        let snapshots_dir = data_dir.join("snapshots");
        let qdrant_url = config.qdrant_url.clone();

        let runtime = Arc::new(
            Runtime::new().map_err(|e| Error::from_reason(format!("tokio runtime: {e}")))?,
        );

        let engine_config = EngineConfig {
            qdrant_url: config.qdrant_url,
            collection_name: config
                .collection_name
                .unwrap_or_else(|| "shabti".to_string()),
            data_dir: data_dir.clone(),
            vector_size: 384,
        };

        let engine = runtime
            .block_on(ShabtiEngine::new(engine_config))
            .map_err(|e| Error::from_reason(format!("engine init: {e}")))?;

        Ok(Self {
            engine: Arc::new(engine),
            _runtime: runtime,
            data_dir,
            snapshots_dir,
            qdrant_url,
        })
    }

    #[napi]
    pub async fn store(
        &self,
        content: String,
        options: Option<NapiStoreOptions>,
    ) -> Result<NapiStoreResult> {
        if content.len() > MAX_CONTENT_CHARS {
            return Err(Error::from_reason(format!(
                "コンテンツサイズが上限(1MB)を超えています"
            )));
        }
        let opts = options.unwrap_or_default();
        let store_opts = StoreOptions {
            namespace: opts.namespace,
            session_id: opts.session_id,
            tags: opts.tags,
            ttl_seconds: opts.ttl_seconds.map(|v| v as u64),
            ..Default::default()
        };

        let engine = Arc::clone(&self.engine);
        let result = engine
            .store(&content, &store_opts)
            .await
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        match result {
            StoreResult::Stored(id) => Ok(NapiStoreResult {
                status: "stored".to_string(),
                id: Some(id.to_string()),
                existing_id: None,
            }),
            StoreResult::Skipped { existing_id } => Ok(NapiStoreResult {
                status: "skipped".to_string(),
                id: None,
                existing_id: Some(existing_id.to_string()),
            }),
        }
    }

    #[napi]
    pub async fn search(&self, query: String, limit: Option<u32>) -> Result<Vec<NapiSearchResult>> {
        let engine = Arc::clone(&self.engine);
        let limit = limit.unwrap_or(10) as usize;

        let results = engine
            .search_similar(&query, limit, None)
            .await
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        Ok(results
            .into_iter()
            .map(|r| NapiSearchResult {
                id: r.entry.id.to_string(),
                content: r.entry.content,
                score: r.score as f64,
                namespace: r.entry.namespace,
                created_at: r.entry.created_at,
                expires_at: r.entry.expires_at,
            })
            .collect())
    }

    #[napi]
    pub async fn execute_query(&self, query: NapiQuery) -> Result<Vec<NapiQueryResult>> {
        let mut builder = QueryBuilder::new(&query.text);

        if let Some(limit) = query.limit {
            builder = builder.limit(limit as usize);
        }
        if let Some(ref ns) = query.namespace {
            builder = builder.namespace(ns);
        }
        if let Some(start) = query.time_start {
            let end = query.time_end.unwrap_or(i64::MAX);
            builder = builder.time_range(start, end);
        }
        if let Some(cid) = query.cluster_id {
            builder = builder.in_cluster(cid as usize);
        }
        if let Some(hops) = query.max_hops {
            builder = builder.follow_links(hops as usize);
        }
        if let Some(score) = query.min_score {
            builder = builder.min_score(score as f32);
        }
        if query.exclude_superseded.unwrap_or(false) {
            builder = builder.exclude_superseded();
        }
        if query.with_explanation.unwrap_or(false) {
            builder = builder.with_explanation();
        }

        let built = builder.build();
        let engine = Arc::clone(&self.engine);

        let results = engine
            .execute_query(&built)
            .await
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        Ok(results
            .into_iter()
            .map(|r| {
                let explanation = r.explanation.map(|exp| NapiExplanation {
                    semantic_similarity: exp.semantic_similarity as f64,
                    time_decay_factor: exp.time_decay_factor as f64,
                    access_boost_factor: exp.access_boost_factor as f64,
                    matched_at_level: exp.matched_at_level as u32,
                    link_hops: exp.link_hops as u32,
                    is_superseded: exp.is_superseded,
                    dedupe_count: exp.dedupe_count,
                    final_score: exp.final_score() as f64,
                });
                NapiQueryResult {
                    id: r.entry.id.to_string(),
                    content: r.entry.content,
                    score: r.score as f64,
                    namespace: r.entry.namespace,
                    created_at: r.entry.created_at,
                    expires_at: r.entry.expires_at,
                    explanation,
                }
            })
            .collect())
    }

    #[napi]
    pub async fn get(&self, id: String) -> Result<NapiSearchResult> {
        let uuid = uuid::Uuid::parse_str(&id)
            .map_err(|e| Error::from_reason(format!("invalid UUID: {e}")))?;

        let engine = Arc::clone(&self.engine);
        let entry = engine
            .get(uuid)
            .await
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        Ok(NapiSearchResult {
            id: entry.id.to_string(),
            content: entry.content,
            score: 1.0,
            namespace: entry.namespace,
            created_at: entry.created_at,
            expires_at: entry.expires_at,
        })
    }

    #[napi]
    pub async fn delete(&self, id: String) -> Result<()> {
        let uuid = uuid::Uuid::parse_str(&id)
            .map_err(|e| Error::from_reason(format!("invalid UUID: {e}")))?;

        let engine = Arc::clone(&self.engine);
        engine
            .delete(uuid)
            .await
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        Ok(())
    }

    #[napi]
    pub fn snapshot_create(&self) -> Result<NapiSnapshot> {
        let snap = snapshot::create_snapshot(&self.data_dir, &self.snapshots_dir)
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        Ok(NapiSnapshot {
            id: snap.id.to_string(),
            created_at: snap.created_at,
        })
    }

    #[napi]
    pub fn snapshot_restore(&self, id: String) -> Result<()> {
        let snaps = snapshot::list_snapshots(&self.snapshots_dir)
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        let snap = snaps
            .iter()
            .find(|s| s.id.to_string() == id)
            .ok_or_else(|| Error::from_reason(format!("snapshot {id} not found")))?;

        snapshot::restore_snapshot(snap, &self.data_dir)
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        Ok(())
    }

    #[napi]
    pub fn snapshot_list(&self) -> Result<Vec<NapiSnapshot>> {
        let snaps = snapshot::list_snapshots(&self.snapshots_dir)
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        Ok(snaps
            .into_iter()
            .map(|s| NapiSnapshot {
                id: s.id.to_string(),
                created_at: s.created_at,
            })
            .collect())
    }

    #[napi]
    pub fn model_id(&self) -> String {
        self.engine.current_model_id().to_string()
    }

    #[napi]
    pub fn status(&self) -> NapiStatus {
        let count = self.engine.entry_count();
        let gate = self.engine.feature_gate();

        NapiStatus {
            entry_count: count as u32,
            tier: format!("{:?}", gate.tier),
            qdrant_url: self.qdrant_url.clone(),
            data_dir: self.data_dir.to_string_lossy().to_string(),
            model_id: self.engine.current_model_id().to_string(),
        }
    }

    #[napi]
    pub fn list_entries(&self, options: Option<NapiListOptions>) -> Result<Vec<NapiExportEntry>> {
        let opts = options.unwrap_or_default();
        let include_emb = opts.include_embeddings.unwrap_or(false);

        let entries = self
            .engine
            .list_entries(opts.namespace.as_deref(), opts.limit.map(|v| v as usize))
            .map_err(|e| Error::from_reason(format!("{e}")))?;

        Ok(entries
            .into_iter()
            .map(|e| NapiExportEntry {
                id: e.id.to_string(),
                content: e.content,
                namespace: e.namespace,
                tags: e.tags,
                keywords: e.keywords,
                session_id: e.session_id,
                created_at: e.created_at,
                expires_at: e.expires_at,
                lifecycle_state: format!("{:?}", e.lifecycle_state),
                origin_type: format!("{:?}", e.origin_type),
                model_id: e.model_id,
                embedding: if include_emb {
                    Some(e.embedding.into_iter().map(|v| v as f64).collect())
                } else {
                    None
                },
                metadata: if e.metadata.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&e.metadata).unwrap_or_default())
                },
            })
            .collect())
    }

    #[napi]
    pub async fn gc(&self) -> Result<u32> {
        let engine = Arc::clone(&self.engine);
        let removed = engine
            .gc()
            .await
            .map_err(|e| Error::from_reason(format!("{e}")))?;
        Ok(removed as u32)
    }

    #[napi]
    pub async fn shutdown(&self) -> Result<()> {
        let engine = Arc::clone(&self.engine);
        engine
            .shutdown()
            .await
            .map_err(|e| Error::from_reason(format!("{e}")))?;
        Ok(())
    }

    #[napi]
    pub fn graph_info(&self) -> NapiGraphInfo {
        let graph = self.engine.graph();
        NapiGraphInfo {
            node_count: graph.node_count() as u32,
            edge_count: graph.edge_count() as u32,
        }
    }
}
