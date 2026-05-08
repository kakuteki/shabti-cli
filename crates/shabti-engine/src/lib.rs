use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use shabti_core::Event;
use shabti_core::dedup::{DedupChecker, StoreResult};
use shabti_core::entry::{MemoryEntry, MemoryEntryBuilder};
use shabti_core::error::{ShabtiError, ShabtiResult};
use shabti_core::gate::FeatureGate;
use shabti_core::query::{Query, SearchExplanation};
use shabti_core::scoring::{self, TimeDecayConfig};
use shabti_core::traits::EmbeddingModel;
use shabti_core::types::OriginType;
use shabti_embedding::FastEmbedModel;
use shabti_graph::MemoryGraph;
use shabti_index::{IndexConfig, QdrantIndex, SearchOptions, TimeRange};
use shabti_storage::EventStore;
use tokio::sync::mpsc;
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
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: MemoryEntry,
    pub score: f32,
}

/// Result from execute_query with optional score explanation.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub entry: MemoryEntry,
    pub score: f32,
    pub explanation: Option<SearchExplanation>,
}

/// k-NN link generation parameters.
const KNN_K: usize = 5;
const KNN_MIN_SIMILARITY: f32 = 0.3;
/// Score decay factor per hop in graph traversal.
const LINK_DECAY_FACTOR: f32 = 0.7;

/// Background indexing task types.
enum IndexTask {
    GenerateLinks(Uuid),
    Flush(tokio::sync::oneshot::Sender<()>),
    Shutdown,
}

/// Validate a namespace string for safety and format.
/// Returns an error if the namespace is empty, too long, or contains forbidden patterns.
fn validate_namespace(ns: &str) -> ShabtiResult<()> {
    if ns.is_empty() {
        return Err(ShabtiError::Validation(
            "namespace must not be empty".to_string(),
        ));
    }
    if ns.len() > 256 {
        return Err(ShabtiError::Validation(
            "namespace must not exceed 256 characters".to_string(),
        ));
    }
    if ns.contains("..") || ns.starts_with('/') {
        return Err(ShabtiError::Validation(
            "namespace contains invalid characters".to_string(),
        ));
    }
    Ok(())
}

pub struct ShabtiEngine {
    embedding: Arc<FastEmbedModel>,
    index: Arc<QdrantIndex>,
    log: Arc<Mutex<shabti_storage::AppendLog>>,
    dedup: Mutex<DedupChecker>,
    entry_count: Mutex<usize>,
    graph: Arc<Mutex<MemoryGraph>>,
    event_store: Arc<Mutex<EventStore>>,
    bg_tx: mpsc::Sender<IndexTask>,
    bg_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl ShabtiEngine {
    pub async fn new(config: EngineConfig) -> ShabtiResult<Self> {
        let embedding = Arc::new(FastEmbedModel::new()?);
        let vector_size = config.vector_size;

        let index = Arc::new(
            QdrantIndex::new(IndexConfig {
                url: config.qdrant_url,
                collection_name: config.collection_name,
                vector_size,
            })
            .await?,
        );

        let log_path = config.data_dir.join("shabti.log");
        let log = shabti_storage::AppendLog::open(&log_path)?;

        // Rebuild dedup checker from existing log entries
        let mut dedup = DedupChecker::new();
        let mut count = 0;
        for entry in log.iter() {
            dedup.check_and_insert(&entry.content_hash, entry.id);
            count += 1;
        }

        // Load or create graph
        let graph_path = config.data_dir.join("graph.json");
        let graph = MemoryGraph::load(&graph_path)?;

        // Load or create event store
        let events_path = config.data_dir.join("events.jsonl");
        let event_store = EventStore::open(&events_path)?;

        // Background indexer channel
        let (bg_tx, bg_rx) = mpsc::channel::<IndexTask>(256);

        let log = Arc::new(Mutex::new(log));
        let graph = Arc::new(Mutex::new(graph));
        let event_store = Arc::new(Mutex::new(event_store));
        let index_clone = Arc::clone(&index);
        let log_clone = Arc::clone(&log);
        let graph_clone = Arc::clone(&graph);
        let graph_path_clone = graph_path.clone();

        // Spawn background indexer task
        let bg_handle = tokio::spawn(async move {
            Self::background_worker(bg_rx, index_clone, log_clone, graph_clone, graph_path_clone)
                .await;
        });

        Ok(Self {
            embedding,
            index,
            log,
            dedup: Mutex::new(dedup),
            entry_count: Mutex::new(count),
            graph,
            event_store,
            bg_tx,
            bg_handle: Mutex::new(Some(bg_handle)),
        })
    }

    async fn background_worker(
        mut rx: mpsc::Receiver<IndexTask>,
        index: Arc<QdrantIndex>,
        log: Arc<Mutex<shabti_storage::AppendLog>>,
        graph: Arc<Mutex<MemoryGraph>>,
        graph_path: PathBuf,
    ) {
        while let Some(task) = rx.recv().await {
            match task {
                IndexTask::GenerateLinks(entry_id) => {
                    // Read entry embedding from log
                    let entry = {
                        let log = log.lock().unwrap();
                        match log.read(entry_id) {
                            Ok(e) => e,
                            Err(_) => continue,
                        }
                    };

                    // k-NN search
                    let hits = match index
                        .search(&entry.embedding, KNN_K + 1, &SearchOptions::default())
                        .await
                    {
                        Ok(h) => h,
                        Err(_) => continue,
                    };

                    // Add nodes and edges
                    let mut g = graph.lock().unwrap();
                    g.add_node(entry_id);
                    for hit in &hits {
                        if hit.id == entry_id {
                            continue; // skip self
                        }
                        if hit.score >= KNN_MIN_SIMILARITY {
                            g.add_node(hit.id);
                            g.add_edge(entry_id, hit.id, hit.score);
                        }
                    }

                    // Save graph periodically (on every link generation for now)
                    let _ = g.save(&graph_path);
                }
                IndexTask::Flush(tx) => {
                    // All previously queued tasks have been processed at this point;
                    // notify the caller that the flush is complete.
                    let _ = tx.send(());
                }
                IndexTask::Shutdown => break,
            }
        }
    }

    pub async fn store(&self, content: &str, options: &StoreOptions) -> ShabtiResult<StoreResult> {
        // Validate namespace format if provided
        if let Some(ref ns) = options.namespace {
            validate_namespace(ns)?;
        }

        // TOCTOU fix: use a single lock acquisition to check for existing or
        // in-flight entries and atomically reserve the slot via mark_pending.
        // This prevents two concurrent tasks from both passing the initial
        // `contains` check and then both writing the same content.
        let content_hash = MemoryEntry::content_hash_of(content);
        {
            let mut dedup = self
                .dedup
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            if dedup.is_known(&content_hash) {
                // Already committed or being processed by another task — skip.
                // Use committed_id to distinguish the two cases without
                // accidentally inserting into the dedup state.
                let existing_id = dedup.committed_id(&content_hash).unwrap_or(Uuid::nil());
                return Ok(StoreResult::Skipped { existing_id });
            }
            // Reserve the slot so concurrent tasks see this hash as in-flight.
            dedup.mark_pending(&content_hash);
        }

        // Generate embedding (lock not held — expensive operation)
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
        if let Some(ttl) = options.ttl_seconds {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before UNIX epoch")
                .as_secs() as i64;
            builder = builder.expires_at(now + ttl as i64);
        }

        let entry = builder.build();
        let id = entry.id;

        // Commit to dedup: clear the pending reservation and insert the final
        // mapping in a single lock acquisition so no window exists between the
        // two states.
        {
            let mut dedup = self
                .dedup
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            dedup.clear_pending(&entry.content_hash);
            dedup.check_and_insert(&entry.content_hash, id);
        }

        // Write to append log
        {
            let mut log = self
                .log
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            log.append(&entry)?;
        }

        // Insert into Qdrant
        self.index.insert(&entry).await?;

        // Update count
        {
            let mut count = self
                .entry_count
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            *count += 1;
        }

        // Queue background link generation (if feature gate allows)
        if self.feature_gate().knn_links {
            let _ = self.bg_tx.send(IndexTask::GenerateLinks(id)).await;
        }

        Ok(StoreResult::Stored(id))
    }

    pub async fn delete(&self, id: Uuid) -> ShabtiResult<()> {
        // Delete from Qdrant
        self.index.delete(id).await?;

        // Remove from graph
        {
            let mut graph = self
                .graph
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            graph.remove_node(id);
        }

        // Decrement count
        {
            let mut count = self
                .entry_count
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            *count = count.saturating_sub(1);
        }

        Ok(())
    }

    pub async fn get(&self, id: Uuid) -> ShabtiResult<MemoryEntry> {
        let log = self
            .log
            .lock()
            .map_err(|e| ShabtiError::Storage(e.to_string()))?;
        log.read(id)
    }

    pub async fn search_similar(
        &self,
        query: &str,
        limit: usize,
        namespace: Option<&str>,
    ) -> ShabtiResult<Vec<SearchResult>> {
        // Validate namespace format if provided
        if let Some(ns) = namespace {
            validate_namespace(ns)?;
        }

        let query_embedding = self.embedding.embed(query)?;

        let opts = SearchOptions {
            namespace: namespace.map(|s| s.to_string()),
            exclude_expired: true,
            ..Default::default()
        };

        let hits = self.index.search(&query_embedding, limit, &opts).await?;

        let log = self
            .log
            .lock()
            .map_err(|e| ShabtiError::Storage(e.to_string()))?;
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

    /// Search with graph link expansion.
    /// First does a vector search, then expands through graph links up to max_hops.
    pub async fn search_with_links(
        &self,
        query: &str,
        limit: usize,
        max_hops: usize,
        namespace: Option<&str>,
    ) -> ShabtiResult<Vec<SearchResult>> {
        // Vector search for seeds
        let mut results = self.search_similar(query, limit, namespace).await?;

        // Expand through graph
        let graph = self
            .graph
            .lock()
            .map_err(|e| ShabtiError::Storage(e.to_string()))?;

        let mut seen: std::collections::HashSet<Uuid> =
            results.iter().map(|r| r.entry.id).collect();
        let mut link_candidates: Vec<(Uuid, f32)> = Vec::new();

        for result in &results {
            let neighbors = graph.neighbors(result.entry.id, max_hops);
            for (neighbor_id, edge_weight) in neighbors {
                if !seen.contains(&neighbor_id) {
                    // Decay score: original_score * edge_weight * decay_factor
                    let decayed = result.score * edge_weight * LINK_DECAY_FACTOR;
                    link_candidates.push((neighbor_id, decayed));
                    seen.insert(neighbor_id);
                }
            }
        }
        drop(graph);

        // Resolve link candidates from log
        let log = self
            .log
            .lock()
            .map_err(|e| ShabtiError::Storage(e.to_string()))?;
        for (id, score) in link_candidates {
            if let Ok(entry) = log.read(id) {
                results.push(SearchResult { entry, score });
            }
        }
        drop(log);

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit
        results.truncate(limit);

        Ok(results)
    }

    /// Explicitly generate k-NN links for a specific entry.
    pub async fn generate_links(&self, entry_id: Uuid) -> ShabtiResult<()> {
        let entry = {
            let log = self
                .log
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            log.read(entry_id)?
        };

        let hits = self
            .index
            .search(&entry.embedding, KNN_K + 1, &SearchOptions::default())
            .await?;

        let mut graph = self
            .graph
            .lock()
            .map_err(|e| ShabtiError::Storage(e.to_string()))?;
        graph.add_node(entry_id);
        for hit in &hits {
            if hit.id == entry_id {
                continue;
            }
            if hit.score >= KNN_MIN_SIMILARITY {
                graph.add_node(hit.id);
                graph.add_edge(entry_id, hit.id, hit.score);
            }
        }

        Ok(())
    }

    pub async fn search_by_time(
        &self,
        range: &TimeRange,
        limit: usize,
    ) -> ShabtiResult<Vec<MemoryEntry>> {
        let hits = self.index.search_by_time(range, limit).await?;

        let log = self
            .log
            .lock()
            .map_err(|e| ShabtiError::Storage(e.to_string()))?;
        let mut results = Vec::new();
        for hit in hits {
            if let Ok(entry) = log.read(hit.id) {
                results.push(entry);
            }
        }

        Ok(results)
    }

    pub fn current_model_id(&self) -> &str {
        self.embedding.model_id()
    }

    pub fn entry_count(&self) -> usize {
        *self.entry_count.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn feature_gate(&self) -> FeatureGate {
        FeatureGate::from_count(self.entry_count())
    }

    /// Access the memory graph (snapshot).
    pub fn graph(&self) -> MemoryGraph {
        let g = self.graph.lock().unwrap();
        // Return a clone via JSON roundtrip (MemoryGraph doesn't impl Clone)
        let json = g.to_json().unwrap_or_else(|_| "{}".into());
        drop(g);
        MemoryGraph::from_json(&json).unwrap_or_default()
    }

    /// List all events.
    pub fn list_events(&self) -> Vec<Event> {
        let store = self.event_store.lock().unwrap();
        store.list().to_vec()
    }

    /// Flush background tasks — wait for all queued tasks to complete.
    ///
    /// Sends a `Flush` sentinel through the worker channel. Because the channel
    /// is FIFO, the worker will process every previously enqueued task before
    /// it reaches the sentinel and sends back the completion signal, giving a
    /// deterministic guarantee instead of an arbitrary sleep.
    pub async fn flush_background(&self) -> ShabtiResult<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.bg_tx
            .send(IndexTask::Flush(tx))
            .await
            .map_err(|_| ShabtiError::Storage("background worker が停止しています".into()))?;
        tokio::time::timeout(std::time::Duration::from_secs(10), rx)
            .await
            .map_err(|_| ShabtiError::Timeout("flush_background がタイムアウトしました".into()))?
            .ok();
        Ok(())
    }

    /// Execute a composable query with time decay scoring and optional explanation.
    pub async fn execute_query(&self, query: &Query) -> ShabtiResult<Vec<QueryResult>> {
        // Validate namespace format if provided
        if let Some(ref ns) = query.namespace {
            validate_namespace(ns)?;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX epoch")
            .as_secs() as i64;
        let decay_config = TimeDecayConfig::default();

        // Build search options
        let opts = SearchOptions {
            namespace: query.namespace.clone(),
            time_range: if query.time_start.is_some() || query.time_end.is_some() {
                Some(TimeRange {
                    start: query.time_start,
                    end: query.time_end,
                })
            } else {
                None
            },
            exclude_expired: true,
            ..Default::default()
        };

        // Embed query text
        let query_embedding = self.embedding.embed(&query.text)?;

        // Fetch more candidates than limit so filtering still yields enough
        let fetch_limit = query.limit * 3;

        // Route: use graph link expansion if max_hops > 0, else vector search
        let raw_results = if query.max_hops > 0 {
            // search_with_links handles vector search + graph expansion internally
            self.search_with_links(
                &query.text,
                fetch_limit,
                query.max_hops,
                query.namespace.as_deref(),
            )
            .await?
        } else {
            // Direct vector search with options
            let hits = self
                .index
                .search(&query_embedding, fetch_limit, &opts)
                .await?;
            let log = self
                .log
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            let mut results = Vec::new();
            for hit in hits {
                if let Ok(entry) = log.read(hit.id) {
                    results.push(SearchResult {
                        entry,
                        score: hit.score,
                    });
                }
            }
            results
        };

        // Apply time decay scoring, filtering, and explanation
        let mut query_results = Vec::new();
        for sr in raw_results {
            // Filter: exclude superseded entries
            if query.exclude_superseded && sr.entry.superseded_by.is_some() {
                continue;
            }

            let semantic_sim = sr.score;
            let decay = scoring::time_decay(sr.entry.created_at, now, &decay_config);
            let boost = scoring::access_boost(sr.entry.access_count);
            let composite = semantic_sim * decay * boost;

            // Filter: min_score
            if let Some(min) = query.min_score
                && composite < min
            {
                continue;
            }

            let explanation = if query.with_explanation {
                Some(SearchExplanation {
                    semantic_similarity: semantic_sim,
                    time_decay_factor: decay,
                    access_boost_factor: boost,
                    matched_at_level: 0,
                    link_hops: 0,
                    is_superseded: sr.entry.superseded_by.is_some(),
                    dedupe_count: 0,
                })
            } else {
                None
            };

            query_results.push(QueryResult {
                entry: sr.entry,
                score: composite,
                explanation,
            });
        }

        // Sort by composite score descending
        query_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        query_results.truncate(query.limit);

        Ok(query_results)
    }

    /// List all entries from the append log, with optional namespace filter and limit.
    pub fn list_entries(
        &self,
        namespace: Option<&str>,
        limit: Option<usize>,
    ) -> ShabtiResult<Vec<MemoryEntry>> {
        // Validate namespace format if provided
        if let Some(ns) = namespace {
            validate_namespace(ns)?;
        }

        let log = self
            .log
            .lock()
            .map_err(|e| ShabtiError::Storage(e.to_string()))?;

        let iter = log.iter();

        let filtered: Vec<MemoryEntry> = match namespace {
            Some(ns) => iter.filter(|e| e.namespace == ns).collect(),
            None => iter.collect(),
        };

        match limit {
            Some(n) => Ok(filtered.into_iter().take(n).collect()),
            None => Ok(filtered),
        }
    }

    /// Garbage collect expired entries. Returns the number of entries removed.
    pub async fn gc(&self) -> ShabtiResult<usize> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX epoch")
            .as_secs() as i64;

        let expired_ids = self.index.scroll_expired(now, 1000).await?;
        let removed = expired_ids.len();

        for id in &expired_ids {
            self.index.delete(*id).await?;
            // Remove from graph
            {
                let mut graph = self
                    .graph
                    .lock()
                    .map_err(|e| ShabtiError::Storage(e.to_string()))?;
                graph.remove_node(*id);
            }
        }

        // Decrement count
        if removed > 0 {
            let mut count = self
                .entry_count
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            *count = count.saturating_sub(removed);
        }

        Ok(removed)
    }

    /// Gracefully shut down the background indexer.
    pub async fn shutdown(&self) -> ShabtiResult<()> {
        let _ = self.bg_tx.send(IndexTask::Shutdown).await;

        let handle = {
            let mut h = self
                .bg_handle
                .lock()
                .map_err(|e| ShabtiError::Storage(e.to_string()))?;
            h.take()
        };

        if let Some(h) = handle {
            let _ = h.await;
        }
        Ok(())
    }

    pub async fn cleanup(&self) -> ShabtiResult<()> {
        self.index.delete_collection().await
    }
}
