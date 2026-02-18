/// Composable Query DSL for memory search.

/// A compiled query with all search parameters.
#[derive(Debug, Clone)]
pub struct Query {
    pub text: String,
    pub limit: usize,
    pub namespace: Option<String>,
    pub time_start: Option<i64>,
    pub time_end: Option<i64>,
    pub cluster_id: Option<usize>,
    pub max_hops: usize,
    pub min_score: Option<f32>,
    pub exclude_superseded: bool,
    pub with_explanation: bool,
}

/// Builder for constructing queries with a fluent API.
pub struct QueryBuilder {
    text: String,
    limit: usize,
    namespace: Option<String>,
    time_start: Option<i64>,
    time_end: Option<i64>,
    cluster_id: Option<usize>,
    max_hops: usize,
    min_score: Option<f32>,
    exclude_superseded: bool,
    with_explanation: bool,
}

impl QueryBuilder {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            limit: 10,
            namespace: None,
            time_start: None,
            time_end: None,
            cluster_id: None,
            max_hops: 0,
            min_score: None,
            exclude_superseded: false,
            with_explanation: false,
        }
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn namespace(mut self, ns: &str) -> Self {
        self.namespace = Some(ns.to_string());
        self
    }

    pub fn time_range(mut self, start: i64, end: i64) -> Self {
        self.time_start = Some(start);
        self.time_end = Some(end);
        self
    }

    pub fn in_cluster(mut self, cluster_id: usize) -> Self {
        self.cluster_id = Some(cluster_id);
        self
    }

    pub fn follow_links(mut self, max_hops: usize) -> Self {
        self.max_hops = max_hops;
        self
    }

    pub fn min_score(mut self, score: f32) -> Self {
        self.min_score = Some(score);
        self
    }

    pub fn exclude_superseded(mut self) -> Self {
        self.exclude_superseded = true;
        self
    }

    pub fn with_explanation(mut self) -> Self {
        self.with_explanation = true;
        self
    }

    pub fn build(self) -> Query {
        Query {
            text: self.text,
            limit: self.limit,
            namespace: self.namespace,
            time_start: self.time_start,
            time_end: self.time_end,
            cluster_id: self.cluster_id,
            max_hops: self.max_hops,
            min_score: self.min_score,
            exclude_superseded: self.exclude_superseded,
            with_explanation: self.with_explanation,
        }
    }
}

/// Score breakdown for a search result.
#[derive(Debug, Clone)]
pub struct SearchExplanation {
    /// Raw cosine similarity score.
    pub semantic_similarity: f32,
    /// Time decay factor applied.
    pub time_decay_factor: f32,
    /// Access frequency boost factor.
    pub access_boost_factor: f32,
    /// Resolution level where match occurred (0=entry, 1=event, 2=session, 3=topic).
    pub matched_at_level: u8,
    /// Number of graph link hops (0 = direct hit).
    pub link_hops: u8,
    /// Whether this entry has been superseded by a newer version.
    pub is_superseded: bool,
    /// Number of near-duplicates in the same group.
    pub dedupe_count: u32,
}

impl Default for SearchExplanation {
    fn default() -> Self {
        Self {
            semantic_similarity: 0.0,
            time_decay_factor: 1.0,
            access_boost_factor: 1.0,
            matched_at_level: 0,
            link_hops: 0,
            is_superseded: false,
            dedupe_count: 0,
        }
    }
}

impl SearchExplanation {
    /// Compute the final composite score from all factors.
    pub fn final_score(&self) -> f32 {
        self.semantic_similarity * self.time_decay_factor * self.access_boost_factor
    }
}
