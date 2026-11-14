//! This module does the controlled-language lint of requirement sentences
//! (docs/research/matrix.md, "Controlled-language lint"). Each problem that
//! the lint finds is a warning. This project wrote its own word lists.

use crate::diag::Code;

/// Terms that you cannot test, escape clauses, open-ended phrases and loose
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

/// Words that end in `-ed` or `-en` but are not participles.
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
        if mentions(&joined, term) {
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

/// True if `joined` (words with one space between them, and a space at each
/// end) has the vague `term`. The word "many" after "how" asks for a number
/// ("how many"). It is not vague.
fn mentions(joined: &str, term: &str) -> bool {
    let needle = format!(" {term} ");
    let mut from = 0;
    while let Some(i) = joined[from..].find(&needle) {
        let at = from + i;
        if !(term == "many" && joined[..at].ends_with(" how")) {
            return true;
        }
        // Step over the space at the start of the match, so that a term at
        // the end of one match can start the next.
        from = at + 1;
    }
    false
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
        assert!(codes("When a tray overflows, the controller SHALL close the valve.").is_empty());
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
                "The controller SHALL log the valve id, the bed id, the row, the state and the time."
            ),
            vec![Code::SentenceLength]
        );
        assert!(codes("The port SHALL stay open.").is_empty());
    }

    #[test]
    fn many_after_how_is_not_vague() {
        assert!(codes("The log SHALL record how many rows the job read.").is_empty());
        assert_eq!(
            codes("The log SHALL record how many rows and many files."),
            vec![Code::VagueTerm]
        );
        assert_eq!(
            codes("The job SHALL read many rows."),
            vec![Code::VagueTerm]
        );
    }
}
