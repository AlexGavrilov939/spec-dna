//! Ranking of sections against the words of a task, for
//! `spec-dna context --query`: Okapi BM25 over the sections the assembly
//! planned. Deterministic, with no network, model or stored index.

/// English function words that carry no topic.
const STOP: &[&str] = &[
    "a", "about", "after", "all", "also", "an", "and", "any", "are", "as", "at", "be", "been",
    "before", "but", "by", "can", "could", "do", "does", "each", "for", "from", "had", "has",
    "have", "how", "if", "in", "into", "is", "it", "its", "may", "more", "must", "no", "not", "of",
    "on", "or", "other", "our", "should", "so", "some", "such", "than", "that", "the", "their",
    "them", "then", "there", "these", "they", "this", "those", "to", "under", "up", "use", "was",
    "we", "were", "what", "when", "where", "which", "while", "who", "why", "will", "with", "would",
    "you",
];

const K1: f64 = 1.2;
const B: f64 = 0.75;
/// A heading word counts this many times.
const HEADING_WEIGHT: u32 = 3;

/// Lowercase words: runs of letters and digits in any script.
fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
}

/// The distinct terms of a query, in first-seen order: words of at least 2
/// characters that are not function words.
pub fn terms(query: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for w in words(query) {
        if w.chars().count() >= 2 && !STOP.contains(&w.as_str()) && !out.contains(&w) {
            out.push(w);
        }
    }
    out
}

/// A word matches a term when they are equal, or when the shorter one has
/// at least 4 characters, begins the longer one, and the longer one adds at
/// most 3 (`gate` and `gates`, `restart` and `restarting`).
fn matches(word: &str, term: &str) -> bool {
    if word == term {
        return true;
    }
    let (short, long) = if word.len() <= term.len() {
        (word, term)
    } else {
        (term, word)
    };
    let (s, l) = (short.chars().count(), long.chars().count());
    s >= 4 && l - s <= 3 && long.starts_with(short)
}

/// BM25 score of each section, given as (heading, body), for `terms`.
pub fn bm25(terms: &[String], sections: &[(&str, &str)]) -> Vec<f64> {
    let n = sections.len();
    if n == 0 || terms.is_empty() {
        return vec![0.0; n];
    }
    let mut tf = vec![vec![0u32; terms.len()]; n];
    let mut len = vec![0u32; n];
    for (i, (heading, body)) in sections.iter().enumerate() {
        let weighted = words(heading)
            .map(|w| (w, HEADING_WEIGHT))
            .chain(words(body).map(|w| (w, 1)));
        for (w, weight) in weighted {
            len[i] += weight;
            for (k, t) in terms.iter().enumerate() {
                if matches(&w, t) {
                    tf[i][k] += weight;
                }
            }
        }
    }
    let avg = (len.iter().map(|&l| f64::from(l)).sum::<f64>() / n as f64).max(1.0);
    let df: Vec<f64> = (0..terms.len())
        .map(|k| tf.iter().filter(|row| row[k] > 0).count() as f64)
        .collect();
    (0..n)
        .map(|i| {
            (0..terms.len())
                .filter(|&k| tf[i][k] > 0)
                .map(|k| {
                    let f = f64::from(tf[i][k]);
                    let idf = ((n as f64 - df[k] + 0.5) / (df[k] + 0.5) + 1.0).ln();
                    let norm = K1 * (1.0 - B + B * f64::from(len[i]) / avg);
                    idf * f * (K1 + 1.0) / (f + norm)
                })
                .sum()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terms_drop_function_words_and_repeats() {
        assert_eq!(
            terms("Enable the feature gate by default; the GATE stays on in v1.35"),
            ["enable", "feature", "gate", "default", "stays", "v1", "35"]
        );
        assert_eq!(
            terms("Проверить таймаут сессии"),
            ["проверить", "таймаут", "сессии"]
        );
        assert!(terms("a to of").is_empty());
    }

    #[test]
    fn near_forms_match() {
        assert!(matches("gates", "gate"));
        assert!(matches("restarting", "restart"));
        assert!(matches("settings", "setting"));
        assert!(!matches("pods", "pod"));
        assert!(!matches("gatekeeper", "gate"));
        assert!(!matches("other", "gate"));
    }

    #[test]
    fn ranks_the_section_that_matches() {
        let t = terms("restart backoff limit");
        let s = bm25(
            &t,
            &[
                ("Motivation", "Users want faster pods."),
                ("Backoff", "The restart backoff has a limit of 300 seconds."),
                ("Graduation", "Beta when tests pass."),
            ],
        );
        assert_eq!(s[0], 0.0);
        assert_eq!(s[2], 0.0);
        assert!(s[1] > 0.0);
        // Same input, same scores.
        assert_eq!(
            s,
            bm25(
                &t,
                &[
                    ("Motivation", "Users want faster pods."),
                    ("Backoff", "The restart backoff has a limit of 300 seconds."),
                    ("Graduation", "Beta when tests pass."),
                ],
            )
        );
    }
}
