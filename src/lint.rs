//! The controlled-language lint on requirement sentences
//! (docs/research/matrix.md, "Controlled-language lint"). Every finding is
//! a warning. The word lists are this project's own.

use crate::diag::Code;

/// Terms that cannot be tested, escape clauses, open endings and loose
/// quantities.
const VAGUE: &[&str] = &[
    "adequate",
    "adequately",
    "appropriate",
    "appropriately",
    "fast",
    "quick",
    "quickly",
    "user-friendly",
    "easy",
    "easily",
    "robust",
    "efficient",
    "efficiently",
    "seamless",
    "seamlessly",
    "reasonable",
    "reasonably",
    "sufficient",
    "sufficiently",
    "flexible",
    "intuitive",
    "normally",
    "typically",
    "usually",
    "optimal",
    "significant",
    "if possible",
    "as appropriate",
    "where practical",
    "if practical",
    "as far as possible",
    "where possible",
    "as needed",
    "if necessary",
    "etc",
    "and so on",
    "including but not limited to",
    "and/or",
    "some",
    "several",
    "various",
    "many",
    "few",
    "approximately",
];

const BE: &[&str] = &["is", "are", "was", "were", "be", "been", "being"];

const IRREGULAR: &[&str] = &[
    "built", "bought", "brought", "caught", "chosen", "done", "driven", "found", "given", "held",
    "hidden", "kept", "known", "left", "lost", "made", "paid", "put", "read", "run", "seen",
    "sent", "set", "shown", "sold", "spent", "taken", "taught", "thrown", "told", "written",
];

/// Words ending in `-ed` or `-en` that are not participles.
const NOT_PARTICIPLE: &[&str] = &[
    "open", "even", "often", "token", "seven", "eleven", "need", "speed", "feed", "seed",
];

pub fn lint(text: &str, max_words: u32) -> Vec<(Code, String, String)> {
    let mut out = Vec::new();
    if !crate::parse::has_bcp14(text) {
        out.push((
            Code::RequirementKeyword,
            String::new(),
            "the requirement has no uppercase BCP 14 keyword".into(),
        ));
    }
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| c.is_whitespace())
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '/'))
        .filter(|w| !w.is_empty())
        .collect();
    if words.first() == Some(&"if") && !words.contains(&"then") {
        out.push((
            Code::EarsPattern,
            String::new(),
            "an `If` requirement has no `then`".into(),
        ));
    }
    if words.len() > max_words as usize {
        out.push((
            Code::SentenceLength,
            String::new(),
            format!(
                "the requirement has {} words, more than {max_words}",
                words.len()
            ),
        ));
    }
    let joined = format!(" {} ", words.join(" "));
    for term in VAGUE {
        if joined.contains(&format!(" {term} ")) {
            out.push((
                Code::VagueTerm,
                (*term).to_string(),
                format!("vague term `{term}`"),
            ));
        }
    }
    for pair in words.windows(2) {
        let (be, next) = (pair[0], pair[1]);
        if BE.contains(&be) && is_participle(next) {
            out.push((
                Code::PassiveVoice,
                format!("{be} {next}"),
                format!("passive voice `{be} {next}`; name the actor"),
            ));
            break;
        }
    }
    out
}

fn is_participle(w: &str) -> bool {
    if NOT_PARTICIPLE.contains(&w) {
        return false;
    }
    IRREGULAR.contains(&w) || (w.len() > 4 && (w.ends_with("ed") || w.ends_with("en")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(text: &str) -> Vec<Code> {
        lint(text, 12).into_iter().map(|f| f.0).collect()
    }

    #[test]
    fn clean_ears_requirement() {
        assert!(codes("When a token expires, the gateway SHALL reject the request.").is_empty());
    }

    #[test]
    fn findings() {
        assert_eq!(
            codes("The request is rejected."),
            vec![Code::RequirementKeyword, Code::PassiveVoice]
        );
        assert_eq!(
            codes("If the cache is full, the service SHALL evict entries."),
            vec![Code::EarsPattern]
        );
        assert_eq!(
            codes("The API SHALL be fast and robust, if possible."),
            vec![Code::VagueTerm, Code::VagueTerm, Code::VagueTerm]
        );
        assert_eq!(
            codes(
                "The gateway SHALL log the request id, the user id, the route, the status and the time."
            ),
            vec![Code::SentenceLength]
        );
        assert!(codes("The port SHALL stay open.").is_empty());
    }
}
