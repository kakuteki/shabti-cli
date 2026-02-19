use std::collections::HashMap;

/// A recorded action (tool usage).
#[derive(Debug, Clone)]
pub struct ActionRecord {
    pub tool_name: String,
    pub input: String,
    pub output: String,
    pub success: bool,
    pub duration_ms: u64,
}

impl ActionRecord {
    pub fn new(
        tool_name: &str,
        input: &str,
        output: &str,
        success: bool,
        duration_ms: u64,
    ) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            input: input.to_string(),
            output: output.to_string(),
            success,
            duration_ms,
        }
    }
}

/// A detected pattern of tool usage steps.
#[derive(Debug, Clone)]
pub struct ActionPattern {
    pub steps: Vec<String>,
    pub occurrences: usize,
    pub successes: usize,
}

impl ActionPattern {
    pub fn success_rate(&self) -> f32 {
        if self.occurrences == 0 {
            return 0.0;
        }
        self.successes as f32 / self.occurrences as f32
    }
}

/// Procedural memory: records actions and detects patterns.
#[derive(Debug, Default)]
pub struct ProceduralMemory {
    records: Vec<ActionRecord>,
}

impl ProceduralMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, action: ActionRecord) {
        self.records.push(action);
    }

    pub fn history(&self) -> &[ActionRecord] {
        &self.records
    }

    pub fn history_for_tool(&self, tool_name: &str) -> Vec<&ActionRecord> {
        self.records
            .iter()
            .filter(|r| r.tool_name == tool_name)
            .collect()
    }

    pub fn tool_success_rate(&self, tool_name: &str) -> f32 {
        let tool_records: Vec<_> = self.history_for_tool(tool_name);
        if tool_records.is_empty() {
            return 0.0;
        }
        let successes = tool_records.iter().filter(|r| r.success).count();
        successes as f32 / tool_records.len() as f32
    }

    /// Detect repeated tool sequences of various window sizes.
    pub fn detect_patterns(&self, min_occurrences: usize) -> Vec<ActionPattern> {
        let tool_seq: Vec<&str> = self.records.iter().map(|r| r.tool_name.as_str()).collect();
        let mut patterns = Vec::new();

        // Try window sizes from 2 to half the sequence length
        let max_window = tool_seq.len() / 2;
        for window_size in 2..=max_window {
            let mut seq_counts: HashMap<Vec<&str>, usize> = HashMap::new();

            for chunk in tool_seq.windows(window_size) {
                *seq_counts.entry(chunk.to_vec()).or_default() += 1;
            }

            for (seq, count) in seq_counts {
                if count >= min_occurrences {
                    // Count how many times all steps succeeded
                    let mut successes = 0;
                    for window in self.records.windows(window_size) {
                        let names: Vec<&str> = window.iter().map(|r| r.tool_name.as_str()).collect();
                        if names == seq && window.iter().all(|r| r.success) {
                            successes += 1;
                        }
                    }

                    patterns.push(ActionPattern {
                        steps: seq.into_iter().map(|s| s.to_string()).collect(),
                        occurrences: count,
                        successes,
                    });
                }
            }
        }

        // Sort by steps length descending (prefer longer patterns), then by occurrences
        patterns.sort_by(|a, b| {
            b.steps
                .len()
                .cmp(&a.steps.len())
                .then_with(|| b.occurrences.cmp(&a.occurrences))
        });
        patterns
    }

    /// Suggest the next tool based on detected patterns.
    pub fn suggest_next(&self, current_tool: &str, min_occurrences: usize) -> Option<String> {
        let tool_seq: Vec<&str> = self.records.iter().map(|r| r.tool_name.as_str()).collect();
        let mut pair_counts: HashMap<(&str, &str), usize> = HashMap::new();

        for window in tool_seq.windows(2) {
            *pair_counts.entry((window[0], window[1])).or_default() += 1;
        }

        pair_counts
            .iter()
            .filter(|((from, _), count)| *from == current_tool && **count >= min_occurrences)
            .max_by_key(|(_, count)| **count)
            .map(|((_, to), _)| to.to_string())
    }
}
