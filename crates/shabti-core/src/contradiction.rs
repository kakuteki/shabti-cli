use regex::Regex;
use std::sync::LazyLock;

/// Type of contradiction detected between two memory entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContradictionType {
    NumericDifference,
    NegationDifference,
}

static NUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d+\.?\d*").unwrap());

static NEGATION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Japanese negation
        Regex::new(r"ない").unwrap(),
        Regex::new(r"ません").unwrap(),
        Regex::new(r"ず$").unwrap(),
        Regex::new(r"ず[、。]").unwrap(),
        Regex::new(r"ぬ$").unwrap(),
        Regex::new(r"不[可能正確]").unwrap(),
        Regex::new(r"非").unwrap(),
        Regex::new(r"無[い理効]").unwrap(),
        // English negation
        Regex::new(r"(?i)\bnot\b").unwrap(),
        Regex::new(r"(?i)\bno\b").unwrap(),
        Regex::new(r"(?i)\bnever\b").unwrap(),
        Regex::new(r"(?i)\bnone\b").unwrap(),
        Regex::new(r"(?i)\bneither\b").unwrap(),
        Regex::new(r"(?i)n't\b").unwrap(),
    ]
});

/// Extract all numbers (integers and decimals) from text.
pub fn extract_numbers(text: &str) -> Vec<String> {
    NUMBER_RE
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Check if text contains negation words (Japanese or English).
pub fn has_negation(text: &str) -> bool {
    NEGATION_PATTERNS.iter().any(|re| re.is_match(text))
}

/// Detect contradiction between two texts.
/// Returns the type of contradiction if found, None otherwise.
///
/// Detection rules:
/// 1. If both texts contain numbers but the number sets differ → NumericDifference
/// 2. If one text has negation and the other doesn't → NegationDifference
pub fn detect_contradiction(text_a: &str, text_b: &str) -> Option<ContradictionType> {
    // Check numeric contradiction
    let nums_a = extract_numbers(text_a);
    let nums_b = extract_numbers(text_b);
    if !nums_a.is_empty() && !nums_b.is_empty() {
        let mut sorted_a = nums_a.clone();
        let mut sorted_b = nums_b.clone();
        sorted_a.sort();
        sorted_b.sort();
        if sorted_a != sorted_b {
            return Some(ContradictionType::NumericDifference);
        }
    }

    // Check negation contradiction
    let neg_a = has_negation(text_a);
    let neg_b = has_negation(text_b);
    if neg_a != neg_b {
        return Some(ContradictionType::NegationDifference);
    }

    None
}
