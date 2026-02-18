use qdrant_client::qdrant::{
    Condition, CountPointsBuilder, CreateCollectionBuilder, CreateFieldIndexCollectionBuilder,
    Distance, FieldType, Filter, PointStruct, Range, SearchPointsBuilder, UpsertPointsBuilder,
    VectorParamsBuilder,
};
use qdrant_client::{Payload, Qdrant};
use serde_json::json;

/// Default Qdrant gRPC URL for local development.
pub const DEFAULT_URL: &str = "http://localhost:6334";
/// Collection name used by this PoC.
pub const COLLECTION_NAME: &str = "shabti_poc";
/// Vector dimension (matching fastembed models: AllMiniLM, MultilingualE5Small, BGE).
pub const VECTOR_DIM: u64 = 384;

/// Create a Qdrant client connected to the given URL.
pub fn create_client(url: &str) -> anyhow::Result<Qdrant> {
    Ok(Qdrant::from_url(url).build()?)
}

/// Create the PoC collection (deletes existing one first).
pub async fn setup_collection(client: &Qdrant) -> anyhow::Result<()> {
    // Delete if exists (ignore errors)
    let _ = client.delete_collection(COLLECTION_NAME).await;

    client
        .create_collection(
            CreateCollectionBuilder::new(COLLECTION_NAME)
                .vectors_config(VectorParamsBuilder::new(VECTOR_DIM, Distance::Cosine)),
        )
        .await?;

    // Create payload index on created_at for efficient time-range queries
    client
        .create_field_index(
            CreateFieldIndexCollectionBuilder::new(
                COLLECTION_NAME,
                "created_at",
                FieldType::Integer,
            ),
        )
        .await?;

    Ok(())
}

/// Insert points with vector + payload (content, created_at timestamp).
pub async fn insert_points(client: &Qdrant, points: Vec<PointData>) -> anyhow::Result<()> {
    let qdrant_points: Vec<PointStruct> = points
        .into_iter()
        .map(|p| {
            PointStruct::new(
                p.id,
                p.vector,
                Payload::try_from(json!({
                    "content": p.content,
                    "created_at": p.created_at,
                }))
                .unwrap(),
            )
        })
        .collect();

    // Upsert in chunks to avoid large payloads
    for chunk in qdrant_points.chunks(100) {
        client
            .upsert_points(UpsertPointsBuilder::new(COLLECTION_NAME, chunk.to_vec()).wait(true))
            .await?;
    }

    Ok(())
}

/// Search for similar vectors, returning top-k results.
pub async fn search_similar(
    client: &Qdrant,
    query_vector: Vec<f32>,
    limit: u64,
) -> anyhow::Result<Vec<SearchResult>> {
    let response = client
        .search_points(
            SearchPointsBuilder::new(COLLECTION_NAME, query_vector, limit).with_payload(true),
        )
        .await?;

    Ok(response.result.into_iter().map(parse_result).collect())
}

/// Search with time-range filter (created_at as integer timestamp).
pub async fn search_with_time_filter(
    client: &Qdrant,
    query_vector: Vec<f32>,
    limit: u64,
    time_from: Option<i64>,
    time_to: Option<i64>,
) -> anyhow::Result<Vec<SearchResult>> {
    let range = Range {
        gte: time_from.map(|t| t as f64),
        lte: time_to.map(|t| t as f64),
        ..Default::default()
    };

    let filter = Filter::must([Condition::range("created_at", range)]);

    let response = client
        .search_points(
            SearchPointsBuilder::new(COLLECTION_NAME, query_vector, limit)
                .filter(filter)
                .with_payload(true),
        )
        .await?;

    Ok(response.result.into_iter().map(parse_result).collect())
}

/// Get the number of points in the collection.
pub async fn count_points(client: &Qdrant) -> anyhow::Result<u64> {
    let response = client
        .count(CountPointsBuilder::new(COLLECTION_NAME).exact(true))
        .await?;
    Ok(response.result.map(|r| r.count).unwrap_or(0))
}

/// Input data for inserting a point.
#[derive(Debug, Clone)]
pub struct PointData {
    pub id: u64,
    pub vector: Vec<f32>,
    pub content: String,
    pub created_at: i64,
}

/// A search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: u64,
    pub score: f32,
    pub content: String,
    pub created_at: i64,
}

fn parse_result(point: qdrant_client::qdrant::ScoredPoint) -> SearchResult {
    let id = match point.id.unwrap().point_id_options.unwrap() {
        qdrant_client::qdrant::point_id::PointIdOptions::Num(n) => n,
        qdrant_client::qdrant::point_id::PointIdOptions::Uuid(u) => {
            panic!("unexpected UUID id: {u}")
        }
    };
    let content = point
        .payload
        .get("content")
        .and_then(|v| v.as_str())
        .map_or(String::new(), |s| s.to_string());
    let created_at = point
        .payload
        .get("created_at")
        .and_then(|v| v.as_integer())
        .unwrap_or(0);

    SearchResult {
        id,
        score: point.score,
        content,
        created_at,
    }
}
