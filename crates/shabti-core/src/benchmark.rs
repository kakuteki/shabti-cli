use serde_json::{Value, json};

/// A memory context entry with relevance score.
#[derive(Debug, Clone)]
pub struct MemoryContext {
    pub content: String,
    pub relevance_score: f32,
}

impl MemoryContext {
    pub fn new(content: &str, relevance_score: f32) -> Self {
        Self {
            content: content.to_string(),
            relevance_score,
        }
    }
}

/// A benchmark scenario: question + expected answer + relevant memories.
#[derive(Debug, Clone)]
pub struct BenchmarkScenario {
    pub id: String,
    pub question: String,
    pub expected_answer: String,
    pub context_memories: Vec<MemoryContext>,
}

impl BenchmarkScenario {
    pub fn new(
        id: &str,
        question: &str,
        expected_answer: &str,
        context_memories: Vec<MemoryContext>,
    ) -> Self {
        Self {
            id: id.to_string(),
            question: question.to_string(),
            expected_answer: expected_answer.to_string(),
            context_memories,
        }
    }
}

/// Result of evaluating a single benchmark scenario.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub scenario_id: String,
    pub score_without_memory: f32,
    pub score_with_memory: f32,
}

impl BenchmarkResult {
    pub fn new(scenario_id: &str, score_without_memory: f32, score_with_memory: f32) -> Self {
        Self {
            scenario_id: scenario_id.to_string(),
            score_without_memory,
            score_with_memory,
        }
    }

    /// Calculate relative improvement: (with - without) / without.
    pub fn improvement(&self) -> f32 {
        if self.score_without_memory.abs() < f32::EPSILON {
            if self.score_with_memory > 0.0 {
                return f32::INFINITY;
            }
            return 0.0;
        }
        (self.score_with_memory - self.score_without_memory) / self.score_without_memory
    }

    pub fn to_json(&self) -> Value {
        json!({
            "scenario_id": self.scenario_id,
            "score_without_memory": self.score_without_memory,
            "score_with_memory": self.score_with_memory,
            "improvement": self.improvement(),
        })
    }
}

/// Summary of benchmark results across all scenarios.
#[derive(Debug, Clone)]
pub struct BenchmarkSummary {
    pub total_scenarios: usize,
    pub avg_score_without_memory: f32,
    pub avg_score_with_memory: f32,
    pub avg_improvement: f32,
    pub improved_count: usize,
}

impl BenchmarkSummary {
    pub fn to_json(&self) -> Value {
        json!({
            "total_scenarios": self.total_scenarios,
            "avg_score_without_memory": self.avg_score_without_memory,
            "avg_score_with_memory": self.avg_score_with_memory,
            "avg_improvement": self.avg_improvement,
            "improved_count": self.improved_count,
        })
    }
}

/// Runner for LLM task improvement benchmarks.
#[derive(Debug, Default)]
pub struct BenchmarkRunner {
    scenarios: Vec<BenchmarkScenario>,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_scenario(&mut self, scenario: BenchmarkScenario) {
        self.scenarios.push(scenario);
    }

    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Evaluate an answer against the expected answer.
    /// Returns a score in [0.0, 1.0].
    pub fn evaluate_answer(&self, actual: &str, expected: &str) -> f32 {
        let actual_lower = actual.to_lowercase();
        let expected_lower = expected.to_lowercase();

        if actual_lower == expected_lower {
            return 1.0;
        }

        if actual_lower.contains(&expected_lower) {
            // Partial match: score based on how much of the answer is the expected content.
            return expected_lower.len() as f32 / actual_lower.len() as f32;
        }

        0.0
    }

    /// Aggregate multiple benchmark results into a summary.
    pub fn aggregate(&self, results: &[BenchmarkResult]) -> BenchmarkSummary {
        let total = results.len();
        if total == 0 {
            return BenchmarkSummary {
                total_scenarios: 0,
                avg_score_without_memory: 0.0,
                avg_score_with_memory: 0.0,
                avg_improvement: 0.0,
                improved_count: 0,
            };
        }

        let sum_without: f32 = results.iter().map(|r| r.score_without_memory).sum();
        let sum_with: f32 = results.iter().map(|r| r.score_with_memory).sum();
        let sum_improvement: f32 = results.iter().map(|r| r.improvement()).sum();
        let improved = results
            .iter()
            .filter(|r| r.score_with_memory > r.score_without_memory)
            .count();

        BenchmarkSummary {
            total_scenarios: total,
            avg_score_without_memory: sum_without / total as f32,
            avg_score_with_memory: sum_with / total as f32,
            avg_improvement: sum_improvement / total as f32,
            improved_count: improved,
        }
    }
}
