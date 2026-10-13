//! Matching a ROM's file name to a game's name in LaunchBox, as
//! `tools/fetch_art.py` does: brackets dropped, a trailing ", The" put
//! back in front, and only letters and digits compared.

/// The name as compared: "Legend of Zelda, The - A Link (USA)" becomes
/// "the legend of zelda a link".
pub fn norm(s: &str) -> String {
    let mut bare = String::with_capacity(s.len());
    let (mut round, mut square) = (0u32, 0u32);
    for ch in s.chars() {
        match ch {
            '(' => round += 1,
            ')' if round > 0 => round -= 1,
            '[' => square += 1,
            ']' if square > 0 => square -= 1,
            _ if round == 0 && square == 0 => bare.push(ch),
            _ => {}
        }
    }
    let bare = article_first(bare.trim());
    let lower = bare.to_lowercase().replace('&', "and").replace('é', "e");
    let mut out = String::with_capacity(lower.len());
    let mut gap = false;
    for ch in lower.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.push(ch);
        } else {
            gap = true;
        }
    }
    out
}

/// "Zelda, The - X" as "The Zelda - X": the last ", the", ", a" or ", an"
/// that ends a word moves to the front.
fn article_first(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.len() != s.len() {
        return s.to_string();
    }
    let mut best: Option<(usize, usize)> = None;
    for article in ["the", "a", "an"] {
        let pat = format!(", {article}");
        let mut from = 0;
        while let Some(i) = lower[from..].find(&pat).map(|i| i + from) {
            let after = i + pat.len();
            let ends = lower[after..]
                .chars()
                .next()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
            if ends && best.is_none_or(|(b, _)| i > b) {
                best = Some((i, article.len()));
            }
            from = i + 1;
        }
    }
    match best {
        Some((i, n)) => format!("{} {}{}", &s[i + 2..i + 2 + n], &s[..i], &s[i + 2 + n..]),
        None => s.to_string(),
    }
}

/// The name without a leading article, for a second try.
pub fn loose(s: &str) -> String {
    let n = norm(s);
    for article in ["the ", "a ", "an "] {
        if let Some(rest) = n.strip_prefix(article) {
            return rest.to_string();
        }
    }
    n
}

/// The name with no spaces, so "Duck Tales" finds "DuckTales".
pub fn squash(s: &str) -> String {
    format!("~{}", loose(s).replace(' ', ""))
}

/// The keys a ROM's name is looked up by, in the order they are tried.
pub fn lookups(stem: &str) -> Vec<String> {
    let mut keys = vec![norm(stem), loose(stem), squash(stem)];
    if let Some((a, b)) = stem.split_once(" - ") {
        keys.push(norm(&format!("{a} {b}")));
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_compare_as_fetch_art_py_compares_them() {
        assert_eq!(
            norm("Legend of Zelda, The - A Link to the Past (USA)"),
            "the legend of zelda a link to the past"
        );
        assert_eq!(norm("Streets of Rage 3 (USA)"), "streets of rage 3");
        assert_eq!(norm("Mario & Luigi [b1] (Europe) (En,Fr)"), "mario and luigi");
        assert_eq!(norm("Pokémon Red"), "pokemon red");
        assert_eq!(loose("The Lion King"), "lion king");
        assert_eq!(squash("Duck Tales (USA)"), "~ducktales");
        assert_eq!(norm("Zelda, Adventure"), "zelda adventure");
        assert_eq!(norm("Simpsons, The"), "the simpsons");
    }
}
