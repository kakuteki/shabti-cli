use shabti_core::procedural::{ActionPattern, ActionRecord, ProceduralMemory};

// --- 6-3: Procedural Memory (action pattern learning) ---

#[test]
fn record_action() {
    let record = ActionRecord::new("web_search", "query: Rust async", "5 results found", true, 120);

    assert_eq!(record.tool_name, "web_search");
    assert_eq!(record.input, "query: Rust async");
    assert_eq!(record.output, "5 results found");
    assert!(record.success);
    assert_eq!(record.duration_ms, 120);
}

#[test]
fn procedural_memory_record_and_retrieve() {
    let mut mem = ProceduralMemory::new();

    mem.record(ActionRecord::new("search", "q1", "r1", true, 10));
    mem.record(ActionRecord::new("store", "d1", "ok", true, 5));
    mem.record(ActionRecord::new("search", "q2", "r2", true, 15));

    assert_eq!(mem.history().len(), 3);
}

#[test]
fn procedural_memory_history_for_tool() {
    let mut mem = ProceduralMemory::new();

    mem.record(ActionRecord::new("search", "q1", "r1", true, 10));
    mem.record(ActionRecord::new("store", "d1", "ok", true, 5));
    mem.record(ActionRecord::new("search", "q2", "r2", false, 15));

    let search_history = mem.history_for_tool("search");
    assert_eq!(search_history.len(), 2);

    let store_history = mem.history_for_tool("store");
    assert_eq!(store_history.len(), 1);
}

#[test]
fn detect_repeated_pattern() {
    let mut mem = ProceduralMemory::new();

    // Same tool sequence repeated: search → analyze → store
    for _ in 0..3 {
        mem.record(ActionRecord::new("search", "q", "r", true, 10));
        mem.record(ActionRecord::new("analyze", "data", "result", true, 20));
        mem.record(ActionRecord::new("store", "content", "ok", true, 5));
    }

    let patterns = mem.detect_patterns(3); // min_occurrences = 3
    assert!(!patterns.is_empty());

    let pattern = &patterns[0];
    assert_eq!(pattern.steps.len(), 3);
    assert_eq!(pattern.steps[0], "search");
    assert_eq!(pattern.steps[1], "analyze");
    assert_eq!(pattern.steps[2], "store");
    assert_eq!(pattern.occurrences, 3);
}

#[test]
fn no_pattern_below_threshold() {
    let mut mem = ProceduralMemory::new();

    mem.record(ActionRecord::new("search", "q", "r", true, 10));
    mem.record(ActionRecord::new("store", "d", "ok", true, 5));

    let patterns = mem.detect_patterns(3);
    assert!(patterns.is_empty());
}

#[test]
fn suggest_next_action() {
    let mut mem = ProceduralMemory::new();

    // Establish pattern: search → store
    for _ in 0..3 {
        mem.record(ActionRecord::new("search", "q", "r", true, 10));
        mem.record(ActionRecord::new("store", "d", "ok", true, 5));
    }

    // After a search, suggest store
    let suggestion = mem.suggest_next("search", 2);
    assert!(suggestion.is_some());
    assert_eq!(suggestion.unwrap(), "store");
}

#[test]
fn no_suggestion_without_pattern() {
    let mem = ProceduralMemory::new();
    assert!(mem.suggest_next("search", 2).is_none());
}

#[test]
fn action_pattern_success_rate() {
    let pattern = ActionPattern {
        steps: vec!["a".into(), "b".into()],
        occurrences: 10,
        successes: 8,
    };

    assert!((pattern.success_rate() - 0.8).abs() < f32::EPSILON);
}

#[test]
fn tool_success_rate() {
    let mut mem = ProceduralMemory::new();

    mem.record(ActionRecord::new("search", "q1", "r1", true, 10));
    mem.record(ActionRecord::new("search", "q2", "r2", true, 15));
    mem.record(ActionRecord::new("search", "q3", "r3", false, 20));
    mem.record(ActionRecord::new("store", "d1", "ok", true, 5));

    let rate = mem.tool_success_rate("search");
    assert!((rate - 2.0 / 3.0).abs() < 0.01);

    let rate = mem.tool_success_rate("store");
    assert!((rate - 1.0).abs() < f32::EPSILON);

    let rate = mem.tool_success_rate("unknown");
    assert!((rate - 0.0).abs() < f32::EPSILON);
}
