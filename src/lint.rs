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

/// The phrases that SD903 reports when `[lint] history-terms` is absent.
pub const DEFAULT_HISTORY_TERMS: [&str; 7] = [
    "previously",
    "formerly",
    "no longer",
    "used to",
    "was changed",
    "before this change",
    "after this change",
];

/// The text of a requirement that SD903 and SD904 read: without link
/// destinations, without inline code spans and without `[[id]]` references.
/// Their text is a name, not prose, so an id with a date does not match.
pub fn prose(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains(['`', '[']) {
        return std::borrow::Cow::Borrowed(text);
    }
    let stripped = crate::parse::strip_link_destinations(text);
    let b = stripped.as_bytes();
    let mut out = String::with_capacity(stripped.len());
    let mut i = 0;
    let mut from = 0;
    while i < b.len() {
        if b[i] == b'`' {
            let n = b[i..].iter().take_while(|&&c| c == b'`').count();
            // The span ends at the next run of exactly `n` backticks.
            let mut j = i + n;
            let mut close = None;
            while j < b.len() {
                if b[j] == b'`' {
                    let m = b[j..].iter().take_while(|&&c| c == b'`').count();
                    if m == n {
                        close = Some(j + m);
                        break;
                    }
                    j += m;
                } else {
                    j += 1;
                }
            }
            match close {
                Some(end) => {
                    out.push_str(&stripped[from..i]);
                    out.push(' ');
                    i = end;
                    from = end;
                }
                None => i += n,
            }
        } else if b[i] == b'[' && b.get(i + 1) == Some(&b'[') {
            match stripped[i + 2..].find("]]") {
                Some(e) => {
                    out.push_str(&stripped[from..i]);
                    out.push(' ');
                    i += 2 + e + 2;
                    from = i;
                }
                None => i += 2,
            }
        } else {
            i += 1;
        }
    }
    out.push_str(&stripped[from..]);
    std::borrow::Cow::Owned(out)
}

/// The words of a text, in lower case. A word is a run of letters and digits
/// with an inner `-`, `'` or `_`. Another character, such as a space or a
/// comma, ends a word.
pub fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || matches!(c, '-' | '\'' | '\u{2019}' | '_')))
        .map(|w| w.trim_matches(|c: char| matches!(c, '-' | '\'' | '\u{2019}' | '_')))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// A set of phrases that match as whole words. SD903 uses it for the history
/// phrases, and SD904 for the forbidden synonyms.
#[derive(Default)]
pub struct Phrases {
    by_first: std::collections::HashMap<String, Vec<usize>>,
    items: Vec<Item>,
}

struct Item {
    words: Vec<String>,
    /// The phrase as the author wrote it.
    label: String,
    /// The term that replaces a forbidden synonym. Empty for a history phrase.
    term: String,
}

impl Phrases {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Adds a phrase. A phrase with no word is ignored.
    pub fn add(&mut self, phrase: &str, term: &str) {
        let words = words(phrase);
        let Some(first) = words.first().cloned() else {
            return;
        };
        self.by_first
            .entry(first)
            .or_default()
            .push(self.items.len());
        self.items.push(Item {
            words,
            label: phrase.trim().to_string(),
            term: term.to_string(),
        });
    }

    /// The history phrases: the `history-terms` of the configuration, or the
    /// default phrases if the key is absent.
    pub fn history(terms: Option<&[String]>) -> Phrases {
        let mut p = Phrases::default();
        match terms {
            Some(t) => t.iter().for_each(|s| p.add(s, "")),
            None => DEFAULT_HISTORY_TERMS.iter().for_each(|s| p.add(s, "")),
        }
        p
    }

    /// Returns each phrase that the words contain, with its term. A phrase
    /// is in the result once. "used to" after a form of "be" is not a match
    /// ("SHALL be used to"): it is the passive of "use".
    pub fn find<'a>(&'a self, ws: &[String]) -> Vec<(&'a str, &'a str)> {
        let mut out: Vec<(&str, &str)> = Vec::new();
        for (i, w) in ws.iter().enumerate() {
            let Some(cands) = self.by_first.get(w) else {
                continue;
            };
            for &k in cands {
                let it = &self.items[k];
                if ws.len() - i < it.words.len() || ws[i..i + it.words.len()] != it.words[..] {
                    continue;
                }
                if it.words == ["used", "to"] && i > 0 && BE.contains(&ws[i - 1].as_str()) {
                    continue;
                }
                if !out.iter().any(|o| o.0 == it.label) {
                    out.push((&it.label, &it.term));
                }
            }
        }
        out
    }
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// Returns the history that a requirement text states: a calendar date,
/// `round <n>`, `iteration <n>` and the phrases of `phrases`. `text` is the
/// output of [`prose`] and `ws` is `words(text)`. The result has each match
/// once, in lower case.
pub fn history(text: &str, ws: &[String], phrases: &Phrases) -> Vec<String> {
    let mut out = history_numbers(text);
    for (label, _) in phrases.find(ws) {
        let m = label.to_lowercase();
        if !out.contains(&m) {
            out.push(m);
        }
    }
    out
}

fn is_alnum(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

/// True if the byte at `i` can start a whole word: no letter or digit
/// before it, and no `.`, `/` or `-` before it, as in a version or a URL.
fn start_ok(b: &[u8], i: usize) -> bool {
    i == 0 || !(is_alnum(b[i - 1]) || matches!(b[i - 1], b'.' | b'/' | b'-' | b'_'))
}

/// True if a match can end before `i`: no letter or digit at `i`, and no
/// `.`, `/` or `-` that a digit follows.
fn end_ok(b: &[u8], i: usize) -> bool {
    match b.get(i) {
        None => true,
        Some(&c) if is_alnum(c) || c == b'_' => false,
        Some(b'.' | b'/' | b'-') => !b.get(i + 1).is_some_and(u8::is_ascii_digit),
        Some(_) => true,
    }
}

/// A run of 1 to `max` digits at `i`. Returns its value and its end.
fn number(b: &[u8], i: usize, max: usize) -> Option<(u32, usize)> {
    let n = b[i..].iter().take_while(|c| c.is_ascii_digit()).count();
    if n == 0 || n > max {
        return None;
    }
    let v = b[i..i + n]
        .iter()
        .fold(0u32, |a, &c| a * 10 + u32::from(c - b'0'));
    Some((v, i + n))
}

/// A year from 1900 to 2099 at `i`.
fn year(b: &[u8], i: usize) -> Option<usize> {
    let (v, e) = number(b, i, 4)?;
    (e - i == 4 && (1900..=2099).contains(&v)).then_some(e)
}

fn skip_blank(b: &[u8], mut i: usize) -> usize {
    while matches!(b.get(i), Some(b' ' | b'\t')) {
        i += 1;
    }
    i
}

/// A date of the form `YYYY-MM-DD`, `YYYY/MM/DD`, `D.M.YYYY` or `D/M/YYYY`
/// at `i`. Returns its end.
fn numeric_date(b: &[u8], i: usize) -> Option<usize> {
    let (a, e) = number(b, i, 4)?;
    let sep = *b.get(e)?;
    let (m, e2) = number(b, e + 1, 2)?;
    if b.get(e2) != Some(&sep) {
        return None;
    }
    if e - i == 4 && matches!(sep, b'-' | b'/') {
        let (d, e3) = number(b, e2 + 1, 2)?;
        let ok = (1900..=2099).contains(&a) && (1..=12).contains(&m) && (1..=31).contains(&d);
        return (ok && end_ok(b, e3)).then_some(e3);
    }
    if e - i <= 2 && matches!(sep, b'.' | b'/') {
        let e3 = year(b, e2 + 1)?;
        let ok = (1..=31).contains(&a) && (1..=12).contains(&m);
        return (ok && end_ok(b, e3)).then_some(e3);
    }
    None
}

/// The end of an English month name at `i`.
fn month(b: &[u8], i: usize) -> Option<usize> {
    let n = b[i..]
        .iter()
        .take_while(|c| c.is_ascii_alphabetic())
        .count();
    let w = std::str::from_utf8(&b[i..i + n]).ok()?;
    MONTHS.contains(&w).then_some(i + n)
}

/// A date with a month name at `i`: `D Month YYYY`, `Month D, YYYY` or
/// `Month YYYY`. Returns its end.
fn named_date(b: &[u8], i: usize) -> Option<usize> {
    if b[i].is_ascii_digit() {
        let (d, e) = number(b, i, 2)?;
        let m = skip_blank(b, e);
        if m == e || !(1..=31).contains(&d) {
            return None;
        }
        let me = month(b, m)?;
        let y = skip_blank(b, me);
        let ye = year(b, y).filter(|_| y > me)?;
        return end_ok(b, ye).then_some(ye);
    }
    let me = month(b, i)?;
    let y = skip_blank(b, me);
    if y == me {
        return None;
    }
    if let Some(ye) = year(b, y) {
        return end_ok(b, ye).then_some(ye);
    }
    let (d, de) = number(b, y, 2)?;
    let after = skip_blank(b, de + usize::from(b.get(de) == Some(&b',')));
    let ye = year(b, after)?;
    ((1..=31).contains(&d) && end_ok(b, ye)).then_some(ye)
}

/// `round <n>` or `iteration <n>` at `i`. Returns its end.
fn counted(b: &[u8], i: usize) -> Option<usize> {
    for w in ["round", "iteration"] {
        if b[i..].starts_with(w.as_bytes()) {
            let s = skip_blank(b, i + w.len());
            if s == i + w.len() {
                return None;
            }
            let (_, e) = number(b, s, 9)?;
            return end_ok(b, e).then_some(e);
        }
    }
    None
}

/// The dates, rounds and iterations of a text. The scan goes from left to
/// right and a match ends before the next one starts, so "3 May 2024" does
/// not also give "May 2024".
fn history_numbers(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let b = lower.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let end = if start_ok(b, i) {
            if b[i].is_ascii_digit() {
                numeric_date(b, i).or_else(|| named_date(b, i))
            } else if b[i].is_ascii_alphabetic() {
                named_date(b, i).or_else(|| counted(b, i))
            } else {
                None
            }
        } else {
            None
        };
        match end {
            Some(e) => {
                let m = lower[i..e].split_whitespace().collect::<Vec<_>>().join(" ");
                if !out.contains(&m) {
                    out.push(m);
                }
                i = e;
            }
            None => i += 1,
        }
    }
    out
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

    fn hist(text: &str) -> Vec<String> {
        let p = Phrases::history(None);
        let t = prose(text);
        history(&t, &words(&t), &p)
    }

    #[test]
    fn every_date_form_is_history() {
        for (text, want) in [
            ("Set on 2024-03-05 by the team.", "2024-03-05"),
            ("Set on 2024/3/5.", "2024/3/5"),
            ("Set on 5.3.2024 by the team.", "5.3.2024"),
            ("Set on 05/03/2024.", "05/03/2024"),
            ("Added on 5 March 2024.", "5 march 2024"),
            ("Added on March 5, 2024.", "march 5, 2024"),
            ("Added on MARCH 5 2024.", "march 5 2024"),
            ("Added in March 2024.", "march 2024"),
            ("The last round 3 fixed it.", "round 3"),
            ("In iteration 12 the limit changed.", "iteration 12"),
        ] {
            assert_eq!(hist(text), vec![want.to_string()], "{text}");
        }
    }

    #[test]
    fn a_longer_date_form_hides_the_shorter_one() {
        assert_eq!(hist("Since 3 May 2024 only."), vec!["3 may 2024"]);
    }

    #[test]
    fn numbers_that_are_not_dates_give_nothing() {
        for text in [
            "The job SHALL keep logs for 30 days.",
            "The valve SHALL open within 2024 ms.",
            "Use version 1.2.3 of the protocol.",
            "Read https://example.org/2024-03-05/page.",
            "The year 2150 is out of range, and 2024-13-40 is no date.",
            "The sensor SHALL read 12.5.2150 as text.",
            "The tray SHALL hold the may flowers.",
            "The pump SHALL roundtrip 3 values.",
            "The pump SHALL stay on for 1900 seconds.",
        ] {
            assert!(hist(text).is_empty(), "{text}: {:?}", hist(text));
        }
    }

    #[test]
    fn a_history_phrase_matches_whole_words_in_any_case() {
        assert_eq!(hist("The pump Previously ran dry."), vec!["previously"]);
        assert_eq!(hist("The pump is no longer needed."), vec!["no longer"]);
        assert!(hist("The pump SHALL stay previouslyish.").is_empty());
    }

    #[test]
    fn used_to_after_be_is_not_history() {
        assert!(hist("The key SHALL be used to open the valve.").is_empty());
        assert!(hist("The key is used to open the valve.").is_empty());
        assert_eq!(hist("The key used to open the valve."), vec!["used to"]);
    }

    #[test]
    fn the_phrases_of_the_configuration_replace_the_default_ones() {
        let custom = Phrases::history(Some(&["superseded by".to_string()]));
        let none = Phrases::history(Some(&[]));
        let text = "Previously the key was superseded by the lock, on 2024-01-02.";
        assert_eq!(
            history(text, &words(text), &custom),
            vec!["2024-01-02", "superseded by"]
        );
        assert_eq!(history(text, &words(text), &none), vec!["2024-01-02"]);
    }

    #[test]
    fn prose_has_no_link_destination_code_span_or_reference() {
        assert_eq!(
            prose("See [the page](../2024-03-05-notes.md) now.").as_ref(),
            "See [the page] now."
        );
        assert!(hist("See [the page](../2024-03-05-notes.md) now.").is_empty());
        assert!(hist("Run `previously 2024-03-05` now.").is_empty());
        assert!(hist("Run ``a ` 2024-03-05`` now.").is_empty());
        assert!(hist("See [[decisions/2024-03-05-pump]] now.").is_empty());
        // An open backtick is text.
        assert_eq!(hist("Run `a on 2024-03-05"), vec!["2024-03-05"]);
    }

    #[test]
    fn a_forbidden_synonym_matches_a_whole_phrase() {
        let mut p = Phrases::default();
        p.add("tap", "valve");
        p.add("Shut-off device", "valve");
        let find = |t: &str| {
            p.find(&words(t))
                .into_iter()
                .map(|(s, v)| format!("{s}>{v}"))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            find("The controller SHALL close the TAP."),
            vec!["tap>valve"]
        );
        assert_eq!(
            find("Close the shut-off device, then the tap."),
            vec!["Shut-off device>valve", "tap>valve"]
        );
        assert!(find("The tape SHALL stay.").is_empty());
    }
}
