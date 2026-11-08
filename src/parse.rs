//! Parses one Markdown document into the data that the index stores: front
//! matter, sections with hashes, references, bare id tokens and requirement
//! candidates. The parser does not use the configuration, so the index caches
//! the result by git blob id alone.

use crate::frontmatter::{self, FrontMatter};
use crate::hash::{Hash, hash_normalized};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

/// Increase this number when the parsed data changes, so that the index
/// removes old entries.
pub const PARSER_VERSION: u32 = 4;

pub const FLAG_REQUIREMENT: u8 = 1;
pub const FLAG_BCP14: u8 = 2;
pub const FLAG_STATUS_WORD: u8 = 4;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParsedDoc {
    pub front_matter: Option<FrontMatter>,
    pub front_matter_error: Option<(u32, String)>,
    pub body_start: u32,
    pub title: String,
    pub chunks: Vec<Chunk>,
    pub refs: Vec<Ref>,
    pub tokens: Vec<Token>,
    pub reqs: Vec<Req>,
    /// The first word of each list item and each table body row, if the word
    /// contains a digit. The model compares these words with `legacy-keys` to
    /// find requirements in a syntax that it does not read (SD806).
    pub leads: Vec<Token>,
    /// Characters of the body without fenced or indented code blocks.
    pub body_chars: u32,
}

/// A section unit: one heading and its text up to the next heading of any
/// level. Level 0 is the text before the first heading.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Chunk {
    pub level: u8,
    pub heading: String,
    pub anchor: String,
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub chars: u32,
    pub hash: Hash,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefKind {
    Link = 0,
    Image = 1,
    Wiki = 2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ref {
    pub kind: RefKind,
    pub target: String,
    pub line: u32,
}

/// A bare `PREFIX-NUMBER` token in prose.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub text: String,
    pub line: u32,
}

/// A list item that starts with a bold one-word key. `colon` is false for the
/// legacy form `**key** sentence`, which a model accepts only for a key that
/// matches a `legacy-keys` pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct Req {
    pub key: String,
    pub colon: bool,
    pub line: u32,
    /// Byte range of the whole list item, with its nested content.
    pub start: u32,
    pub end: u32,
    pub chunk: u32,
    /// Hash of the sentence without the key and the alias marker.
    pub hash: Hash,
    pub aliases: Vec<String>,
    /// The sentence, whitespace-normalized, without the alias marker.
    pub text: String,
}

pub fn parse(text: &str) -> ParsedDoc {
    let mut doc = ParsedDoc::default();
    let lines = LineIndex::new(text);
    let mut body_start = 0usize;
    if let Some((fm, start)) = frontmatter::split(text) {
        body_start = start;
        match frontmatter::parse(fm, 2) {
            Ok(v) => doc.front_matter = Some(v),
            Err(e) => doc.front_matter_error = Some(e),
        }
    }
    doc.body_start = body_start as u32;
    let body = &text[body_start..];

    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_WIKILINKS;

    let mut code_chars = 0usize;
    let mut in_code = false;
    let mut heading: Option<(u8, usize, String)> = None;
    let mut chunk_flags: Vec<u8> = vec![0];
    // Chunk starts: (level, heading, start offset in body).
    let mut starts: Vec<(u8, String, usize)> = vec![(0, String::new(), 0)];
    let mut items: Vec<Item> = Vec::new();
    let mut cands: Vec<Cand> = Vec::new();

    for (ev, range) in Parser::new_ext(body, opts).into_offset_iter() {
        // Requirement detection looks at the first events of the innermost item.
        if let Some(item) = items.last_mut() {
            item.step(&ev, range.start, range.end);
        }
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((level as u8, range.start, String::new()));
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, start, text)) = heading.take() {
                    starts.push((level, text.trim().to_string(), start));
                    chunk_flags.push(0);
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code = true;
                code_chars += body[range.clone()].chars().count();
            }
            Event::End(TagEnd::CodeBlock) => in_code = false,
            Event::Start(Tag::Item) => {
                if let Some(w) = lead_word(strip_list_marker(&body[range.clone()])) {
                    doc.leads.push(Token {
                        text: w.to_string(),
                        line: lines.line(body_start + range.start),
                    });
                }
                items.push(Item::new(range.start, range.end))
            }
            Event::Start(Tag::TableRow) => {
                let row = body[range.clone()].trim_start();
                if let Some(w) = lead_word(row.strip_prefix('|').unwrap_or(row)) {
                    doc.leads.push(Token {
                        text: w.to_string(),
                        line: lines.line(body_start + range.start),
                    });
                }
            }
            Event::End(TagEnd::Item) => {
                if let Some(item) = items.pop()
                    && let Some(c) = item.finish()
                {
                    // A bold label without a colon is usual in prose. Only the
                    // model can tell if it is a legacy key.
                    if c.colon {
                        *chunk_flags.last_mut().unwrap() |= FLAG_REQUIREMENT;
                    }
                    cands.push(c);
                }
            }
            Event::Start(Tag::List(_)) => {
                // A nested list ends the sentence of the parent item.
                let n = items.len();
                if n >= 2 {
                    let parent = &mut items[n - 2];
                    if parent.first_list.is_none() {
                        parent.first_list = Some(range.start);
                    }
                } else if let Some(parent) = items.last_mut()
                    && parent.first_list.is_none()
                    && range.start > parent.start
                {
                    parent.first_list = Some(range.start);
                }
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                let kind = match link_type {
                    LinkType::Autolink | LinkType::Email => None,
                    LinkType::WikiLink { .. } => Some(RefKind::Wiki),
                    _ => Some(RefKind::Link),
                };
                if let Some(kind) = kind {
                    doc.refs.push(Ref {
                        kind,
                        target: dest_url.to_string(),
                        line: lines.line(body_start + range.start),
                    });
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                doc.refs.push(Ref {
                    kind: RefKind::Image,
                    target: dest_url.to_string(),
                    line: lines.line(body_start + range.start),
                });
            }
            Event::Text(t) if !in_code => {
                if let Some((_, _, h)) = heading.as_mut() {
                    h.push_str(&t);
                }
                *chunk_flags.last_mut().unwrap() |= word_flags(&t);
                scan_tokens(&t, lines.line(body_start + range.start), &mut doc.tokens);
            }
            Event::Code(t) => {
                if let Some((_, _, h)) = heading.as_mut() {
                    h.push_str(&t);
                }
            }
            _ => {}
        }
    }

    // Build the chunks. Keep the level-0 chunk only if it contains text.
    let mut seen: Vec<String> = Vec::new();
    for (i, (level, head, start)) in starts.iter().enumerate() {
        let end = starts.get(i + 1).map_or(body.len(), |s| s.2);
        let slice = &body[*start..end];
        if *level == 0 && slice.trim().is_empty() {
            continue;
        }
        let anchor = if *level == 0 {
            String::new()
        } else {
            unique_anchor(&slugify(head), &mut seen)
        };
        doc.chunks.push(Chunk {
            level: *level,
            heading: head.clone(),
            anchor,
            start: (body_start + start) as u32,
            end: (body_start + end) as u32,
            line: lines.line(body_start + start),
            chars: slice.chars().count() as u32,
            hash: hash_normalized(slice),
            flags: chunk_flags[i],
        });
    }
    doc.title = doc
        .chunks
        .iter()
        .find(|c| c.level == 1)
        .map(|c| c.heading.clone())
        .unwrap_or_default();

    // Attach each requirement to the chunk that contains it.
    for c in cands {
        let abs_start = body_start + c.start;
        let chunk = doc
            .chunks
            .iter()
            .rposition(|ch| ch.start as usize <= abs_start)
            .unwrap_or(0) as u32;
        let mut raw = &body[c.sentence_start..c.sentence_end];
        if !c.colon {
            // `**R-12** — text` and `**R-12**. text`: the separator is not
            // part of the sentence.
            raw = raw.trim_start_matches(|ch: char| {
                ch.is_whitespace() || matches!(ch, ':' | '.' | '-' | '–' | '—')
            });
        }
        let (clean, aliases) = strip_alias_marker(raw);
        doc.reqs.push(Req {
            key: c.key,
            colon: c.colon,
            line: lines.line(abs_start),
            start: abs_start as u32,
            end: (body_start + c.end) as u32,
            chunk,
            hash: hash_normalized(&clean),
            aliases,
            text: crate::hash::normalize(&clean),
        });
    }

    doc.body_chars = (body.chars().count() - code_chars) as u32;
    doc
}

/// Removes the marker of a list item (`-`, `*`, `+`, `1.` or `1)`) and a task
/// box (`[ ]` or `[x]`).
fn strip_list_marker(item: &str) -> &str {
    let s = item.trim_start();
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    let s = if s.starts_with(['-', '*', '+']) {
        &s[1..]
    } else if digits > 0 && s[digits..].starts_with(['.', ')']) {
        &s[digits + 1..]
    } else {
        s
    };
    let s = s.trim_start();
    ["[ ]", "[x]", "[X]"]
        .iter()
        .find_map(|b| s.strip_prefix(b))
        .unwrap_or(s)
        .trim_start()
}

/// Returns the first word of a list item or table cell without emphasis
/// marks and trailing punctuation, if it contains an ASCII digit. A word ends
/// at whitespace, `*`, `` ` ``, `|`, `(`, `)`, `[`, `]` or `:`.
fn lead_word(s: &str) -> Option<&str> {
    let s = s.trim_start().trim_start_matches(['*', '_', '~', '`', '[']);
    let end = s
        .find(|c: char| {
            c.is_whitespace() || matches!(c, '*' | '`' | '|' | '(' | ')' | '[' | ']' | ':')
        })
        .unwrap_or(s.len());
    let w = s[..end].trim_end_matches(['.', ',', ';', '_', '~']);
    (!w.is_empty() && w.len() <= 64 && w.bytes().any(|b| b.is_ascii_digit())).then_some(w)
}

struct Item {
    start: usize,
    end: usize,
    /// 0: wait for the bold key, 1: in the key, 2: wait for `:`, 3: done.
    phase: u8,
    key: String,
    /// The offset after the closing `**` of the key.
    key_end: usize,
    sentence_start: usize,
    first_list: Option<usize>,
    candidate: bool,
    colon: bool,
}

struct Cand {
    key: String,
    colon: bool,
    start: usize,
    end: usize,
    sentence_start: usize,
    sentence_end: usize,
}

impl Item {
    fn new(start: usize, end: usize) -> Self {
        Item {
            start,
            end,
            phase: 0,
            key: String::new(),
            key_end: 0,
            sentence_start: 0,
            first_list: None,
            candidate: false,
            colon: false,
        }
    }

    fn step(&mut self, ev: &Event, start: usize, end: usize) {
        match (self.phase, ev) {
            (0, Event::Start(Tag::Item)) if start == self.start => {}
            (0, Event::Start(Tag::Paragraph)) => {}
            (0, Event::Start(Tag::Strong)) => self.phase = 1,
            (1, Event::Text(t)) => self.key.push_str(t),
            (1, Event::End(TagEnd::Strong)) => {
                self.key_end = end;
                if let Some(k) = self.key.strip_suffix(':') {
                    // `**key:** sentence`
                    self.key = k.to_string();
                    self.finish_key(end, true);
                } else if let Some(k) = self.key.strip_suffix('.') {
                    // `**R-12.** sentence`: a numbered legacy form. Only a
                    // legacy key makes it a requirement.
                    self.key = k.to_string();
                    self.finish_key(end, false);
                } else {
                    self.phase = 2;
                }
            }
            (2, Event::Text(t)) if t.starts_with(':') => self.finish_key(start + 1, true),
            // `**key** sentence`: a legacy form without a colon.
            (2, _) => self.finish_key(self.key_end, false),
            (0..=1, _) => self.phase = 3,
            _ => {}
        }
    }

    fn finish_key(&mut self, sentence_start: usize, colon: bool) {
        self.phase = 3;
        let k = self.key.trim();
        if !k.is_empty() && !k.contains(char::is_whitespace) {
            self.key = k.to_string();
            self.sentence_start = sentence_start;
            self.candidate = true;
            self.colon = colon;
        }
    }

    fn finish(mut self) -> Option<Cand> {
        if self.phase == 2 {
            // The item has only the bold key.
            self.finish_key(self.key_end, false);
        }
        if !self.candidate {
            return None;
        }
        let sentence_end = self.first_list.unwrap_or(self.end).max(self.sentence_start);
        Some(Cand {
            key: self.key,
            colon: self.colon,
            start: self.start,
            end: self.end,
            sentence_start: self.sentence_start,
            sentence_end,
        })
    }
}

/// Removes `<!-- aliases: a#b, c#d -->` and returns the aliases it lists.
pub fn strip_alias_marker(raw: &str) -> (String, Vec<String>) {
    let mut aliases = Vec::new();
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(pos) = rest.find("<!--") {
        let after = &rest[pos + 4..];
        let Some(close) = after.find("-->") else {
            break;
        };
        let inner = after[..close].trim();
        out.push_str(&rest[..pos]);
        if let Some(list) = inner.strip_prefix("aliases:") {
            aliases.extend(
                list.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty()),
            );
        } else {
            out.push_str(&rest[pos..pos + 4 + close + 3]);
        }
        rest = &after[close + 3..];
    }
    out.push_str(rest);
    (out, aliases)
}

const BCP14: [&str; 7] = [
    "MUST",
    "SHALL",
    "SHOULD",
    "MAY",
    "REQUIRED",
    "RECOMMENDED",
    "OPTIONAL",
];
const STATUS_WORDS: [&str; 9] = [
    "accepted",
    "rejected",
    "superseded",
    "deprecated",
    "retired",
    "proposed",
    "active",
    "draft",
    "done",
];

pub fn has_bcp14(text: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .any(|w| BCP14.contains(&w))
}

fn word_flags(text: &str) -> u8 {
    let mut flags = 0;
    for w in text.split(|c: char| !c.is_ascii_alphabetic()) {
        if w.is_empty() {
            continue;
        }
        if BCP14.contains(&w) {
            flags |= FLAG_BCP14;
        } else if w.len() >= 4 && STATUS_WORDS.iter().any(|s| s.eq_ignore_ascii_case(w)) {
            flags |= FLAG_STATUS_WORD;
        }
    }
    flags
}

/// Collects `PREFIX-NUMBER` tokens: an uppercase ASCII prefix, a hyphen and
/// digits, at word boundaries.
fn scan_tokens(text: &str, line: u32, out: &mut Vec<Token>) {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_uppercase() && (i == 0 || !is_word_byte(b[i - 1])) {
            let start = i;
            let mut j = i;
            while j < b.len() && (b[j].is_ascii_uppercase() || b[j].is_ascii_digit()) {
                j += 1;
            }
            if j < b.len() && b[j] == b'-' && j + 1 < b.len() && b[j + 1].is_ascii_digit() {
                let mut k = j + 1;
                while k < b.len() && b[k].is_ascii_digit() {
                    k += 1;
                }
                if k == b.len() || !is_word_byte(b[k]) {
                    out.push(Token {
                        text: text[start..k].to_string(),
                        line,
                    });
                }
                i = k;
                continue;
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
}

fn is_word_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c >= 0x80
}

/// Makes a GitHub-style heading anchor. It converts the heading to lowercase,
/// keeps letters, digits, `-` and `_`, changes spaces to `-`, and removes all
/// other characters.
pub fn slugify(heading: &str) -> String {
    let mut out = String::with_capacity(heading.len());
    for ch in heading.trim().chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' {
            out.extend(ch.to_lowercase());
        } else if ch == ' ' {
            out.push('-');
        }
    }
    out
}

fn unique_anchor(base: &str, seen: &mut Vec<String>) -> String {
    let mut anchor = base.to_string();
    let mut n = 0;
    while seen.contains(&anchor) {
        n += 1;
        anchor = format!("{base}-{n}");
    }
    seen.push(anchor.clone());
    anchor
}

/// Maps byte offsets to 1-based line numbers.
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(memchr_iter(b'\n', text.as_bytes()).map(|i| i + 1));
        LineIndex { starts }
    }

    pub fn line(&self, offset: usize) -> u32 {
        match self.starts.binary_search(&offset) {
            Ok(i) => i as u32 + 1,
            Err(i) => i as u32,
        }
    }
}

fn memchr_iter(needle: u8, hay: &[u8]) -> impl Iterator<Item = usize> + '_ {
    hay.iter()
        .enumerate()
        .filter(move |(_, b)| **b == needle)
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "---\nstatus: active\n---\n# Session tokens\n\nIntro with ADR-12 and `ADR-13` in code.\n\n## Requirements\n\n- **token-expiry**: When a session token expires, the gateway SHALL reject the request. <!-- aliases: auth/old#ttl -->\n- **R-12**: The client MUST retry.\n  - nested note\n- plain item\n- **Note** this is not a key\n\n```md\n- **in-code**: ignored SHALL\n```\n\nSee [the guide](../guide.md#setup) and [[auth/session#token-expiry@3f2a9c1b7e0d4a11]].\n\n## Requirements\n\n## Amendments\n\nText.\n";

    #[test]
    fn sections_anchors_and_lines() {
        let d = parse(DOC);
        let heads: Vec<(&str, &str, u32)> = d
            .chunks
            .iter()
            .map(|c| (c.heading.as_str(), c.anchor.as_str(), c.line))
            .collect();
        assert_eq!(
            heads,
            vec![
                ("Session tokens", "session-tokens", 4),
                ("Requirements", "requirements", 8),
                ("Requirements", "requirements-1", 22),
                ("Amendments", "amendments", 24)
            ]
        );
        assert_eq!(d.title, "Session tokens");
        assert!(d.chunks[1].flags & FLAG_REQUIREMENT != 0);
        assert!(d.chunks[1].flags & FLAG_BCP14 != 0);
    }

    #[test]
    fn requirements_and_aliases() {
        let d = parse(DOC);
        let keys: Vec<(&str, bool)> = d.reqs.iter().map(|r| (r.key.as_str(), r.colon)).collect();
        // `**Note** this is not a key` is a candidate without a colon. The
        // model accepts it only if it matches a legacy key pattern.
        assert_eq!(
            keys,
            vec![("token-expiry", true), ("R-12", true), ("Note", false)]
        );
        let r = &d.reqs[0];
        assert_eq!(r.aliases, vec!["auth/old#ttl".to_string()]);
        assert_eq!(
            r.text,
            "When a session token expires, the gateway SHALL reject the request."
        );
        assert_eq!(r.line, 10);
        assert_eq!(d.reqs[1].text, "The client MUST retry.");
        // The alias marker does not change the hash.
        let moved = DOC.replace(" <!-- aliases: auth/old#ttl -->", "");
        assert_eq!(parse(&moved).reqs[0].hash, r.hash);
    }

    #[test]
    fn legacy_requirements_without_a_colon() {
        let d = parse(
            "# S\n\n## Rules\n\n- **R-1** The client MUST retry.\n- **R-2** — The server SHALL log.\n- **R-3:** The cache MAY drop entries.\n- **R-4**\n- See **R-5** in the text.\n- **R-6.** The proxy MUST strip the header.\n\n## Notes\n\n- **Note** prose with a bold label.\n",
        );
        let reqs: Vec<(&str, bool, &str)> = d
            .reqs
            .iter()
            .map(|r| (r.key.as_str(), r.colon, r.text.as_str()))
            .collect();
        assert_eq!(
            reqs,
            vec![
                ("R-1", false, "The client MUST retry."),
                ("R-2", false, "The server SHALL log."),
                ("R-3", true, "The cache MAY drop entries."),
                ("R-4", false, ""),
                ("R-6", false, "The proxy MUST strip the header."),
                ("Note", false, "prose with a bold label."),
            ]
        );
        // Only a candidate with a colon marks its section. The model marks the
        // others after it checks the key.
        assert_eq!(d.chunks[1].flags & FLAG_REQUIREMENT, FLAG_REQUIREMENT);
        assert_eq!(d.chunks[2].flags & FLAG_REQUIREMENT, 0);
        // The separator after the key does not change the hash.
        let plain = parse("- **R-2** The server SHALL log.\n");
        assert_eq!(plain.reqs[0].hash, d.reqs[1].hash);
    }

    #[test]
    fn references_and_tokens() {
        let d = parse(DOC);
        let refs: Vec<(RefKind, &str, u32)> = d
            .refs
            .iter()
            .map(|r| (r.kind, r.target.as_str(), r.line))
            .collect();
        assert_eq!(
            refs,
            vec![
                (RefKind::Link, "../guide.md#setup", 20),
                (
                    RefKind::Wiki,
                    "auth/session#token-expiry@3f2a9c1b7e0d4a11",
                    20
                )
            ]
        );
        let toks: Vec<&str> = d.tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(toks, vec!["ADR-12", "R-12"]);
    }

    #[test]
    fn lead_words() {
        let d = parse(
            "- R-1 plain key\n- **R-2.** bold with a period\n1. `R-3`: code\n- [x] R-4a done\n- See R-5\n- plain\n  - R-6) nested\n\n| Id | Text |\n| --- | --- |\n| R-7 | row |\n| **R-8** | bold row |\n| none | row |\n\n```md\n- R-9 in code\n```\n",
        );
        let leads: Vec<(&str, u32)> = d.leads.iter().map(|t| (t.text.as_str(), t.line)).collect();
        assert_eq!(
            leads,
            [
                ("R-1", 1),
                ("R-2", 2),
                ("R-3", 3),
                ("R-4a", 4),
                ("R-6", 7),
                ("R-7", 11),
                ("R-8", 12)
            ]
        );
    }

    #[test]
    fn slugs() {
        assert_eq!(
            slugify("Context and Problem Statement"),
            "context-and-problem-statement"
        );
        assert_eq!(slugify("Q&A: `foo()` / bar"), "qa-foo--bar");
        assert_eq!(slugify("Привет, мир"), "привет-мир");
    }

    #[test]
    fn body_chars_skip_code_blocks() {
        let d = parse("# T\n\n```\ncode\n```\n");
        // The fence lines are code. The final line end is not code.
        assert_eq!(d.body_chars, "# T\n\n\n".chars().count() as u32);
    }
}
