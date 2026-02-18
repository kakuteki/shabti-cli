#[macro_use]
extern crate napi_derive;

use std::path::PathBuf;
use std::sync::Arc;

use napi::bindgen_prelude::*;
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
pub struct NapiStatus {
    pub entry_count: u32,
    pub tier: String,
    pub qdrant_url: String,
    pub data_dir: String,
    pub model_id: String,
}

// ============================================================
// ShabtiNapi — main binding class
// ============================================================

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
        let opts = options.unwrap_or_default();
        let store_opts = StoreOptions {
            namespace: opts.namespace,
            session_id: opts.session_id,
            tags: opts.tags,
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
        })
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
    pub async fn shutdown(&self) -> Result<()> {
        let engine = Arc::clone(&self.engine);
        engine
            .shutdown()
            .await
            .map_err(|e| Error::from_reason(format!("{e}")))?;
        Ok(())
    }
}
