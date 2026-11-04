//! Small helpers for spoken English.

/// "a" or "an" for the word or number that follows: "an Orchard Lantern",
/// "an August evening", "an 8 day streak", "a 3 day streak".
pub fn a_or_an(next: &str) -> &'static str {
    let next = next.trim_start();
    let Some(first) = next.chars().next() else {
        return "a";
    };
    if first.is_ascii_digit() {
        // Eight, eighty, eleven and eighteen (and their thousands) start
        // with a vowel sound.
        let digits: String = next.chars().take_while(|c| c.is_ascii_digit()).collect();
        let eleven_or_eighteen = matches!(digits.len(), 2 | 5 | 8)
            && (digits.starts_with("11") || digits.starts_with("18"));
        return if first == '8' || eleven_or_eighteen {
            "an"
        } else {
            "a"
        };
    }
    if matches!(first.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    }
}

/// "A 3 day streak." or "An 8 day streak.", for challenge results.
pub fn streak_sentence(n: u32, unit: &str) -> String {
    let n = n.to_string();
    let article = if a_or_an(&n) == "an" { "An" } else { "A" };
    format!("{article} {n} {unit} streak.")
}

#[cfg(test)]
mod tests {
    use super::{a_or_an, streak_sentence};

    #[test]
    fn streak_sentences() {
        assert_eq!(streak_sentence(3, "day"), "A 3 day streak.");
        assert_eq!(streak_sentence(8, "day"), "An 8 day streak.");
    }

    #[test]
    fn picks_an_before_a_vowel_sound() {
        assert_eq!(a_or_an("Orchard Lantern"), "an");
        assert_eq!(a_or_an("August"), "an");
        assert_eq!(a_or_an("Dipper"), "a");
        assert_eq!(a_or_an("8 day"), "an");
        assert_eq!(a_or_an("11"), "an");
        assert_eq!(a_or_an("18"), "an");
        assert_eq!(a_or_an("80"), "an");
        assert_eq!(a_or_an("3"), "a");
        assert_eq!(a_or_an("1100"), "a");
        assert_eq!(a_or_an("110"), "a");
    }
}
