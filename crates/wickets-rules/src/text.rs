//! The few text helpers rule files use for their own words.

/// The phrase with its first letter made a capital.
pub fn sentence_start(phrase: &str) -> String {
    let mut chars = phrase.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
