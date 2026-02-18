use shabti_core::contradiction::{
    extract_numbers, has_negation, detect_contradiction, ContradictionType,
};

// ============================================================
// Number extraction
// ============================================================

#[test]
fn extract_numbers_from_japanese() {
    let numbers = extract_numbers("価格は1000円です");
    assert!(numbers.contains(&"1000".to_string()));
}

#[test]
fn extract_numbers_multiple() {
    let numbers = extract_numbers("10時から15時まで営業しています");
    assert_eq!(numbers.len(), 2);
    assert!(numbers.contains(&"10".to_string()));
    assert!(numbers.contains(&"15".to_string()));
}

#[test]
fn extract_numbers_with_decimals() {
    let numbers = extract_numbers("気温は36.5度でした");
    assert!(numbers.contains(&"36.5".to_string()));
}

#[test]
fn extract_numbers_none() {
    let numbers = extract_numbers("今日は天気がいいです");
    assert!(numbers.is_empty());
}

// ============================================================
// Negation detection
// ============================================================

#[test]
fn negation_nai() {
    assert!(has_negation("猫が好きではない"));
}

#[test]
fn negation_zu() {
    assert!(has_negation("食べず"));
}

#[test]
fn negation_english_not() {
    assert!(has_negation("this is not correct"));
}

#[test]
fn negation_english_never() {
    assert!(has_negation("I never go there"));
}

#[test]
fn no_negation() {
    assert!(!has_negation("猫が好きです"));
}

#[test]
fn no_negation_english() {
    assert!(!has_negation("this is correct"));
}

// ============================================================
// Contradiction detection
// ============================================================

#[test]
fn contradiction_different_numbers() {
    let result = detect_contradiction(
        "この商品の価格は1000円です",
        "この商品の価格は2000円です",
    );
    assert!(
        result.is_some(),
        "different prices should be a contradiction"
    );
    assert!(matches!(
        result.unwrap(),
        ContradictionType::NumericDifference
    ));
}

#[test]
fn contradiction_negation() {
    let result = detect_contradiction(
        "猫が好きです",
        "猫が好きではない",
    );
    assert!(
        result.is_some(),
        "negation should be a contradiction"
    );
    assert!(matches!(
        result.unwrap(),
        ContradictionType::NegationDifference
    ));
}

#[test]
fn no_contradiction_same_content() {
    let result = detect_contradiction(
        "東京の天気は晴れです",
        "東京の天気は晴れです",
    );
    assert!(result.is_none(), "identical content should not be a contradiction");
}

#[test]
fn no_contradiction_complementary() {
    let result = detect_contradiction(
        "猫はペットとして人気があります",
        "犬もペットとして愛されています",
    );
    assert!(result.is_none(), "complementary content should not be a contradiction");
}

#[test]
fn contradiction_english_numbers() {
    let result = detect_contradiction(
        "The population is 1000000",
        "The population is 2000000",
    );
    assert!(result.is_some());
}

#[test]
fn contradiction_english_negation() {
    let result = detect_contradiction(
        "The system is working correctly",
        "The system is not working correctly",
    );
    assert!(result.is_some());
}
