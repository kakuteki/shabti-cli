use qdrant_client::qdrant::vectors_config::Config;
use qdrant_client::qdrant::{
    Condition, CreateCollectionBuilder, DeletePointsBuilder, Distance, Filter,
    HnswConfigDiffBuilder, PointStruct, Range, ScrollPointsBuilder,
    SearchPointsBuilder, SetPayloadPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
    VectorsConfig,
};
use qdrant_client::{Payload, Qdrant};
use serde_json::json;
use uuid::Uuid;

use shabti_core::entry::MemoryEntry;
use shabti_core::error::{ShabtiError, ShabtiResult};

pub struct IndexConfig {
    pub url: String,
    pub collection_name: String,
    pub vector_size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub namespace: Option<String>,
    pub time_range: Option<TimeRange>,
    pub min_score: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start: Option<i64>,
    pub end: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub id: Uuid,
    pub score: f32,
    pub access_count: u32,
    pub last_accessed: i64,
}

pub struct QdrantIndex {
    client: Qdrant,
    collection_name: String,
}

impl QdrantIndex {
    pub async fn new(config: IndexConfig) -> ShabtiResult<Self> {
        let client = Qdrant::from_url(&config.url)
            .build()
            .map_err(|e| ShabtiError::Index(e.to_string()))?;

        let collection_name = config.collection_name.clone();

        let exists = client
            .collection_exists(&collection_name)
            .await
            .map_err(|e| ShabtiError::Index(e.to_string()))?;

        if !exists {
            client
                .create_collection(
                    CreateCollectionBuilder::new(&collection_name).vectors_config(
                        VectorsConfig {
                            config: Some(Config::Params(
                                VectorParamsBuilder::new(config.vector_size, Distance::Cosine)
                                    .hnsw_config(HnswConfigDiffBuilder::default())
                                    .build(),
                            )),
                        },
                    ),
                )
                .await
                .map_err(|e| ShabtiError::Index(e.to_string()))?;
        }

        Ok(Self {
            client,
            collection_name,
        })
    }

    pub async fn insert(&self, entry: &MemoryEntry) -> ShabtiResult<()> {
        let point_id = uuid_to_point_id(entry.id);
        let payload = entry_to_payload(entry);
        let point = PointStruct::new(point_id, entry.embedding.clone(), payload);

        self.client
            .upsert_points(UpsertPointsBuilder::new(&self.collection_name, vec![point]).wait(true))
            .await
            .map_err(|e| ShabtiError::Index(e.to_string()))?;

        Ok(())
    }

    pub async fn search(
        &self,
        embedding: &[f32],
        limit: usize,
        options: &SearchOptions,
    ) -> ShabtiResult<Vec<SearchHit>> {
        let filter = build_filter(options, true);

        let mut builder = SearchPointsBuilder::new(
            &self.collection_name,
            embedding.to_vec(),
            limit as u64,
        )
        .filter(filter)
        .with_payload(true);

        if let Some(min_score) = options.min_score {
            builder = builder.score_threshold(min_score);
        }

        let results = self
            .client
            .search_points(builder)
            .await
            .map_err(|e| ShabtiError::Index(e.to_string()))?;

        Ok(results
            .result
            .into_iter()
            .filter_map(|point| {
                let id = point_id_to_uuid(&point.id?)?;
                let payload = point.payload;
                Some(SearchHit {
                    id,
                    score: point.score,
                    access_count: extract_u32(&payload, "access_count"),
                    last_accessed: extract_i64(&payload, "last_accessed"),
                })
            })
            .collect())
    }

    pub async fn search_by_time(
        &self,
        range: &TimeRange,
        limit: usize,
    ) -> ShabtiResult<Vec<SearchHit>> {
        let options = SearchOptions {
            time_range: Some(range.clone()),
            ..Default::default()
        };
        let filter = build_filter(&options, true);

        let results = self
            .client
            .scroll(
                ScrollPointsBuilder::new(&self.collection_name)
                    .filter(filter)
                    .limit(limit as u32)
                    .with_payload(true),
            )
            .await
            .map_err(|e| ShabtiError::Index(e.to_string()))?;

        Ok(results
            .result
            .into_iter()
            .filter_map(|point| {
                let id = point_id_to_uuid(&point.id?)?;
                let payload = point.payload;
                Some(SearchHit {
                    id,
                    score: 0.0,
                    access_count: extract_u32(&payload, "access_count"),
                    last_accessed: extract_i64(&payload, "last_accessed"),
                })
            })
            .collect())
    }

    pub async fn delete(&self, id: Uuid) -> ShabtiResult<()> {
        let point_id = uuid_to_point_id(id);
        self.client
            .delete_points(
                DeletePointsBuilder::new(&self.collection_name)
                    .points(vec![point_id])
                    .wait(true),
            )
            .await
            .map_err(|e| ShabtiError::Index(e.to_string()))?;
        Ok(())
    }

    pub async fn update_access(
        &self,
        id: Uuid,
        access_count: u32,
        last_accessed: i64,
    ) -> ShabtiResult<()> {
        let point_id = uuid_to_point_id(id);
        let payload_json = json!({
            "access_count": access_count,
            "last_accessed": last_accessed,
        });
        let payload: Payload = Payload::try_from(payload_json)
            .map_err(|e| ShabtiError::Index(e.to_string()))?;

        self.client
            .set_payload(
                SetPayloadPointsBuilder::new(&self.collection_name, payload)
                    .points_selector(vec![point_id])
                    .wait(true),
            )
            .await
            .map_err(|e| ShabtiError::Index(e.to_string()))?;
        Ok(())
    }

    pub async fn delete_collection(&self) -> ShabtiResult<()> {
        self.client
            .delete_collection(&self.collection_name)
            .await
            .map_err(|e| ShabtiError::Index(e.to_string()))?;
        Ok(())
    }
}

fn build_filter(options: &SearchOptions, active_only: bool) -> Filter {
    let mut filter = Filter::default();

    if active_only {
        filter
            .must
            .push(Condition::matches("lifecycle_state", "active".to_string()));
    }

    if let Some(ref ns) = options.namespace {
        filter
            .must
            .push(Condition::matches("namespace", ns.clone()));
    }

    if let Some(ref tr) = options.time_range {
        let mut range = Range::default();
        if let Some(start) = tr.start {
            range.gte = Some(start as f64);
        }
        if let Some(end) = tr.end {
            range.lt = Some(end as f64);
        }
        filter
            .must
            .push(Condition::range("created_at", range));
    }

    filter
}

fn entry_to_payload(entry: &MemoryEntry) -> Payload {
    Payload::try_from(json!({
        "content": entry.content,
        "content_hash": entry.content_hash,
        "model_id": entry.model_id,
        "created_at": entry.created_at,
        "session_id": entry.session_id,
        "namespace": entry.namespace,
        "lifecycle_state": serde_json::to_value(&entry.lifecycle_state).unwrap_or_default(),
        "origin_type": serde_json::to_value(&entry.origin_type).unwrap_or_default(),
        "access_count": entry.access_count as i64,
        "last_accessed": entry.last_accessed,
        "gap_before": entry.gap_before,
        "gap_after": entry.gap_after,
    }))
    .unwrap_or_default()
}

fn uuid_to_point_id(id: Uuid) -> qdrant_client::qdrant::PointId {
    id.to_string().into()
}

fn point_id_to_uuid(point_id: &qdrant_client::qdrant::PointId) -> Option<Uuid> {
    use qdrant_client::qdrant::point_id::PointIdOptions;
    match &point_id.point_id_options {
        Some(PointIdOptions::Uuid(s)) => Uuid::parse_str(s).ok(),
        _ => None,
    }
}

fn extract_u32(
    payload: &std::collections::HashMap<String, qdrant_client::qdrant::Value>,
    key: &str,
) -> u32 {
    payload
        .get(key)
        .and_then(|v| v.as_integer())
        .unwrap_or(0) as u32
}

fn extract_i64(
    payload: &std::collections::HashMap<String, qdrant_client::qdrant::Value>,
    key: &str,
) -> i64 {
    payload
        .get(key)
        .and_then(|v| v.as_integer())
        .unwrap_or(0)
}
