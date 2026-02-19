use shabti_core::benchmark::{BenchmarkResult, BenchmarkRunner, BenchmarkScenario, MemoryContext};

// --- 6-6: LLM task improvement benchmark framework ---

#[test]
fn create_benchmark_scenario() {
    let scenario = BenchmarkScenario::new(
        "meeting-time",
        "What time is the meeting?",
        "3pm",
        vec![MemoryContext::new(
            "The meeting is scheduled at 3pm in the conference room",
            0.95,
        )],
    );

    assert_eq!(scenario.id, "meeting-time");
    assert_eq!(scenario.question, "What time is the meeting?");
    assert_eq!(scenario.expected_answer, "3pm");
    assert_eq!(scenario.context_memories.len(), 1);
}

#[test]
fn benchmark_scenario_without_context() {
    let scenario = BenchmarkScenario::new(
        "no-context",
        "What is the capital of France?",
        "Paris",
        vec![],
    );

    assert!(scenario.context_memories.is_empty());
}

#[test]
fn memory_context_with_relevance() {
    let ctx = MemoryContext::new("Important fact about the project", 0.87);

    assert_eq!(ctx.content, "Important fact about the project");
    assert!((ctx.relevance_score - 0.87).abs() < f32::EPSILON);
}

#[test]
fn benchmark_result_improvement_positive() {
    let result = BenchmarkResult::new(
        "test-scenario",
        0.6, // without memory
        0.9, // with memory
    );

    assert_eq!(result.scenario_id, "test-scenario");
    assert!((result.score_without_memory - 0.6).abs() < f32::EPSILON);
    assert!((result.score_with_memory - 0.9).abs() < f32::EPSILON);
    assert!((result.improvement() - 0.5).abs() < 0.01); // 50% improvement
}

#[test]
fn benchmark_result_no_improvement() {
    let result = BenchmarkResult::new("equal", 0.8, 0.8);
    assert!((result.improvement()).abs() < f32::EPSILON);
}

#[test]
fn benchmark_result_negative_improvement() {
    let result = BenchmarkResult::new("worse", 0.9, 0.7);
    assert!(result.improvement() < 0.0);
}

#[test]
fn benchmark_runner_add_scenarios() {
    let mut runner = BenchmarkRunner::new();

    runner.add_scenario(BenchmarkScenario::new(
        "s1",
        "Q1?",
        "A1",
        vec![MemoryContext::new("ctx1", 0.9)],
    ));
    runner.add_scenario(BenchmarkScenario::new("s2", "Q2?", "A2", vec![]));

    assert_eq!(runner.scenario_count(), 2);
}

#[test]
fn benchmark_runner_evaluate_exact_match() {
    let runner = BenchmarkRunner::new();

    // Exact match
    assert!((runner.evaluate_answer("Paris", "Paris") - 1.0).abs() < f32::EPSILON);

    // Case insensitive
    assert!((runner.evaluate_answer("paris", "Paris") - 1.0).abs() < f32::EPSILON);

    // Contains match
    let score = runner.evaluate_answer("The capital is Paris", "Paris");
    assert!(score > 0.0);
    assert!(score < 1.0);

    // No match
    assert!((runner.evaluate_answer("London", "Paris")).abs() < f32::EPSILON);
}

#[test]
fn benchmark_runner_aggregate_results() {
    let runner = BenchmarkRunner::new();

    let results = vec![
        BenchmarkResult::new("s1", 0.5, 0.8),
        BenchmarkResult::new("s2", 0.6, 0.9),
        BenchmarkResult::new("s3", 0.7, 0.7),
    ];

    let summary = runner.aggregate(&results);
    assert_eq!(summary.total_scenarios, 3);
    assert!(summary.avg_improvement > 0.0);
    assert!(summary.avg_score_with_memory > summary.avg_score_without_memory);
    assert_eq!(summary.improved_count, 2);
}

#[test]
fn benchmark_summary_to_json() {
    let runner = BenchmarkRunner::new();

    let results = vec![
        BenchmarkResult::new("s1", 0.5, 0.8),
        BenchmarkResult::new("s2", 0.6, 0.9),
    ];

    let summary = runner.aggregate(&results);
    let json = summary.to_json();

    assert_eq!(json["total_scenarios"], 2);
    assert!(json["avg_improvement"].as_f64().unwrap() > 0.0);
    assert_eq!(json["improved_count"], 2);
}

#[test]
fn benchmark_result_to_json() {
    let result = BenchmarkResult::new("test", 0.5, 0.8);
    let json = result.to_json();

    assert_eq!(json["scenario_id"], "test");
    assert!((json["score_without_memory"].as_f64().unwrap() - 0.5).abs() < 0.01);
    assert!((json["score_with_memory"].as_f64().unwrap() - 0.8).abs() < 0.01);
    assert!(json["improvement"].as_f64().unwrap() > 0.0);
}
