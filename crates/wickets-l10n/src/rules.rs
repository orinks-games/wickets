//! Wicket's own checks on every translated line, on top of Hearken's
//! (placeholders, numbers, key names, plural forms).

use hearken_l10n_tools::Rule;

pub const ALL: &[Rule] = &[short_speech, no_new_symbols];

/// Whether `code` writes without spaces between words, so its lines are
/// shorter in characters and the length rule does not apply.
fn wide(code: &str) -> bool {
    matches!(code.split(['_', '-']).next(), Some("zh" | "ja"))
}

/// Short speech: a translation may run longer than its English, but not
/// far longer. Short lines are left alone, where a word or two is a big
/// share.
fn short_speech(code: &str, english: &str, translated: &str) -> Vec<String> {
    let (en, tr) = (spoken_len(english), spoken_len(translated));
    if wide(code) || en < 30 || tr * 10 <= en * 17 {
        return Vec::new();
    }
    vec![format!(
        "is {} percent longer than the English; say it in fewer words",
        (tr * 100 / en).saturating_sub(100)
    )]
}

/// Length as heard: a plural `{n:one|few|many}` counts as its longest form,
/// not as all of its forms written side by side.
fn spoken_len(line: &str) -> usize {
    let mut total = 0;
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        total += rest[..open].chars().count();
        let Some(close) = rest[open..].find('}') else {
            return total + rest[open..].chars().count();
        };
        let inner = &rest[open + 1..open + close];
        total += match inner.split_once(':') {
            Some((_, forms)) => forms
                .split('|')
                .map(|f| f.chars().count())
                .max()
                .unwrap_or(0),
            None => inner.chars().count() + 2,
        };
        rest = &rest[open + close + 1..];
    }
    total + rest.chars().count()
}

/// Symbols a screen reader reads out by name, or text that only works on a
/// screen, may not appear in a translation when the English has none.
fn no_new_symbols(_code: &str, english: &str, translated: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (symbol, name) in [
        ('(', "parentheses"),
        ('[', "brackets"),
        ('%', "a percent sign"),
        ('&', "an ampersand"),
        ('/', "a slash"),
        ('\u{2014}', "an em-dash"),
        ('\u{2026}', "an ellipsis"),
    ] {
        if translated.contains(symbol) && !english.contains(symbol) {
            out.push(format!("adds {name}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_translations_and_new_symbols_are_flagged() {
        let english = "Press Enter to light the lamp above the door.";
        assert!(short_speech("de", english, "Drücke Enter, um die Lampe anzuzünden.").is_empty());
        assert_eq!(short_speech("de", english, &"x".repeat(80)).len(), 1);
        assert!(short_speech("ja", english, &"x".repeat(80)).is_empty());
        assert!(short_speech(
            "ru",
            "Treasure chest, {value} coins.",
            "Сундук с сокровищем, {value} {value:монета|монеты|монет}."
        )
        .is_empty());
        assert_eq!(
            no_new_symbols("es", "Lamps lit.", "Faroles (encendidos)."),
            ["adds parentheses"]
        );
    }
}
