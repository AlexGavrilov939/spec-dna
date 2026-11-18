//! Parses one Markdown document into the data that the index stores: front
//! matter, sections with hashes, references, bare id tokens and requirement
//! candidates. The parser does not use the configuration, so the index caches
//! the result by git blob id alone.

use crate::frontmatter::{self, FrontMatter};
use crate::hash::{Hash, hash_normalized};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// Increase this number when the parsed data changes, so that the index
/// removes old entries.
pub const PARSER_VERSION: u32 = 16;

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
    /// Inline code spans that can cite code: a path, a `path:line`, or an
    /// identifier (ADR 0001, amendment of 2026-11-08 "code citations"). The
    /// model resolves them against the tree.
    pub code_spans: Vec<Token>,
    /// Words of prose that contain a `/`. The model keeps a word as a citation
    /// only if it names a file or a directory of the tree.
    pub path_words: Vec<Token>,
    /// Template placeholders (`{{text}}`) in prose, outside code (SD306).
    pub placeholders: Vec<Token>,
    /// Characters of the body without fenced or indented code blocks.
    pub body_chars: u32,
    /// The rows of each table whose header has the cells `Term` and
    /// `Forbidden synonyms`, in any order and in any case. The parser does not
    /// know the type of the document, so it stores the rows of every document.
    /// Only a `terms` document uses them (SD904).
    pub terms: Vec<TermRow>,
    /// The top-level list items of the sections that carry a change (`Delta`,
    /// `Interfaces`, `Decisions`, `Concerns`, `Files`, `Follow-ups`, `Order`
    /// and `Open questions`), in the order of the file. The parser
    /// stores them for every document. Only the documents of a change and an
    /// incident use them.
    pub items: Vec<ListItem>,
    /// The first paragraph of the section `Files` that is not in a list: its
    /// text as it is in the source, on one line, and the line of its start
    /// (design 0.7.0, D28). The model reads it for a plan.
    pub files_paragraph: Option<Token>,
}

/// The sections whose list items the parser keeps. The heading text must be
/// the name. The case and the heading level do not matter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemSection {
    Delta = 0,
    Interfaces = 1,
    Decisions = 2,
    Concerns = 3,
    Files = 4,
    FollowUps = 5,
    Order = 6,
    OpenQuestions = 7,
}

impl ItemSection {
    pub const ALL: [ItemSection; 8] = [
        ItemSection::Delta,
        ItemSection::Interfaces,
        ItemSection::Decisions,
        ItemSection::Concerns,
        ItemSection::Files,
        ItemSection::FollowUps,
        ItemSection::Order,
        ItemSection::OpenQuestions,
    ];

    pub fn heading(self) -> &'static str {
        match self {
            ItemSection::Delta => "Delta",
            ItemSection::Interfaces => "Interfaces",
            ItemSection::Decisions => "Decisions",
            ItemSection::Concerns => "Concerns",
            ItemSection::Files => "Files",
            ItemSection::FollowUps => "Follow-ups",
            ItemSection::Order => "Order",
            ItemSection::OpenQuestions => "Open questions",
        }
    }

    /// Returns the section that a heading names, if it names one.
    pub fn from_heading(heading: &str) -> Option<ItemSection> {
        let h = heading.trim();
        ItemSection::ALL
            .into_iter()
            .find(|s| h.eq_ignore_ascii_case(s.heading()))
    }
}

/// A top-level list item of one of the sections of [`ItemSection`]. A nested
/// list belongs to its parent item: the range covers it, and `nested` is true.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    pub section: ItemSection,
    /// `Some(false)` for `- [ ]`, `Some(true)` for `- [x]`, and `None` for an
    /// item with no task box.
    pub task: Option<bool>,
    /// The item text as it is in the source: after the list marker and the
    /// task box, before a nested list, with no space at the start and at the
    /// end. Line ends and blank lines stay as they are.
    pub text: String,
    /// The inline code spans of the item text, in the order of the text,
    /// without a nested list and without an empty span.
    pub codes: Vec<String>,
    /// True if the item has a nested list.
    pub nested: bool,
    pub line: u32,
    /// Byte range in the source text: from the list marker to the end of the
    /// last line that has text, with its nested content and its line end. It
    /// has no blank line after the item.
    pub range: Range<u32>,
}

/// One row of a table of terms.
#[derive(Clone, Debug, PartialEq)]
pub struct TermRow {
    pub term: String,
    /// The forbidden synonyms: the cell split at `,` and `;`, each part
    /// trimmed, no empty part.
    pub forbidden: Vec<String>,
    pub line: u32,
}

/// The state of the table that the parser reads. It keeps the text of each
/// cell. Only a header with the 2 cells of a table of terms gives rows.
#[derive(Default)]
struct TableState {
    in_head: bool,
    in_cell: bool,
    cell: String,
    head: Vec<String>,
    row: Vec<String>,
    row_line: u32,
    /// The columns of `Term` and `Forbidden synonyms`, if the header has both.
    cols: Option<(usize, usize)>,
}

impl TableState {
    /// True if the text of a cell matters: a header cell, or a cell of a
    /// table of terms. Other tables thus need less work.
    fn wants_text(&self) -> bool {
        self.in_cell && (self.in_head || self.cols.is_some())
    }

    fn end_cell(&mut self) {
        self.in_cell = false;
        let cell = crate::hash::normalize(&self.cell);
        self.cell.clear();
        if self.in_head {
            self.head.push(cell);
        } else {
            self.row.push(cell);
        }
    }

    fn end_head(&mut self) {
        self.in_head = false;
        let col = |name: &str| self.head.iter().position(|h| h.eq_ignore_ascii_case(name));
        self.cols = col("Term").zip(col("Forbidden synonyms"));
    }

    fn end_row(&mut self, out: &mut Vec<TermRow>) {
        let row = std::mem::take(&mut self.row);
        let Some((t, f)) = self.cols else { return };
        // A row with a `{{...}}` placeholder is a row of a template. SD306
        // reports it. It is not a term.
        if row
            .iter()
            .any(|c| c.find("{{").is_some_and(|i| c[i + 2..].contains("}}")))
        {
            return;
        }
        let term = row.get(t).cloned().unwrap_or_default();
        let forbidden: Vec<String> = row
            .get(f)
            .map(|c| {
                c.split([',', ';'])
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if !term.is_empty() || !forbidden.is_empty() {
            out.push(TermRow {
                term,
                forbidden,
                line: self.row_line,
            });
        }
    }
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
    /// Hash of the text without link destinations. SD405 compares it, so a
    /// move that rewrites a link does not change the frozen text.
    pub frozen: Hash,
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
    /// Hash of the same sentence without link destinations. The
    /// preservation check of `mv` compares it, because `mv` rewrites the
    /// links to a moved key or section.
    pub frozen: Hash,
    pub aliases: Vec<String>,
    /// The paths of an inline `<!-- verified-by: <path>, ... -->` marker.
    pub verified_by: Vec<String>,
    /// The sentence, whitespace-normalized, without the alias marker.
    pub text: String,
    /// Byte range of the sentence in the source text: after the bold key and
    /// its colon, without the space at the start, and without the markers
    /// and the space at the end. A marker inside the sentence stays in the
    /// range. For an empty sentence the range is empty.
    pub sentence: Range<u32>,
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
    // The text of a link is not a code citation of its own: the link target
    // is the citation.
    let mut in_link = 0u32;
    let mut heading: Option<(u8, usize, String)> = None;
    let mut chunk_flags: Vec<u8> = vec![0];
    // Chunk starts: (level, heading, start offset in body).
    let mut starts: Vec<(u8, String, usize)> = vec![(0, String::new(), 0)];
    let mut items: Vec<Item> = Vec::new();
    let mut cands: Vec<Cand> = Vec::new();
    let mut table = TableState::default();
    let mut scan = ItemScan::default();

    for (ev, range) in Parser::new_ext(body, opts).into_offset_iter() {
        scan.step(&ev, &range, body, body_start, &lines, &mut doc.items);
        // Requirement detection looks at the first events of the innermost item.
        if let Some(item) = items.last_mut() {
            item.step(&ev, range.start, range.end);
        }
        match &ev {
            Event::Start(Tag::Table(_)) => table = TableState::default(),
            Event::Start(Tag::TableHead) => table.in_head = true,
            Event::End(TagEnd::TableHead) => table.end_head(),
            Event::Start(Tag::TableCell) => table.in_cell = true,
            Event::End(TagEnd::TableCell) => table.end_cell(),
            Event::End(TagEnd::TableRow) => table.end_row(&mut doc.terms),
            Event::Text(t) | Event::Code(t) if table.wants_text() => table.cell.push_str(t),
            Event::SoftBreak | Event::HardBreak if table.wants_text() => table.cell.push(' '),
            _ => {}
        }
        if let Event::Start(Tag::TableRow) = &ev {
            table.row_line = lines.line(body_start + range.start);
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
            Event::End(TagEnd::Link) => in_link = in_link.saturating_sub(1),
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                ..
            }) => {
                in_link += 1;
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
                let line = lines.line(body_start + range.start);
                scan_tokens(&t, line, &mut doc.tokens);
                scan_placeholders(&t, line, &mut doc.placeholders);
                if in_link == 0 {
                    scan_path_words(&t, line, &mut doc.path_words);
                }
            }
            Event::Code(t) => {
                if let Some((_, _, h)) = heading.as_mut() {
                    h.push_str(&t);
                }
                let span = t.trim();
                if in_link == 0 && is_code_candidate(span) {
                    doc.code_spans.push(Token {
                        text: span.to_string(),
                        line: lines.line(body_start + range.start),
                    });
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
            frozen: hash_normalized(&strip_link_destinations(slice)),
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
        let mut raw_start = c.sentence_start;
        if !c.colon {
            // `**R-12** — text` and `**R-12**. text`: the separator is not
            // part of the sentence.
            let trimmed = raw.trim_start_matches(|ch: char| {
                ch.is_whitespace() || matches!(ch, ':' | '.' | '-' | '–' | '—')
            });
            raw_start += raw.len() - trimmed.len();
            raw = trimmed;
        }
        let (clean, aliases, verified_by) = strip_markers(raw);
        let lead = raw.len() - raw.trim_start().len();
        let sentence = (body_start + raw_start + lead) as u32
            ..(body_start + raw_start + sentence_end(raw).max(lead)) as u32;
        doc.reqs.push(Req {
            key: c.key,
            colon: c.colon,
            line: lines.line(abs_start),
            start: abs_start as u32,
            end: (body_start + c.end) as u32,
            chunk,
            hash: hash_normalized(&clean),
            frozen: hash_normalized(&strip_link_destinations(&clean)),
            aliases,
            verified_by,
            text: crate::hash::normalize(&clean),
            sentence,
        });
    }

    doc.body_chars = (body.chars().count() - code_chars) as u32;
    doc.items.sort_by_key(|i| i.range.start);
    doc.files_paragraph = scan.files_paragraph.take();
    doc
}

/// The state of the scan for the list items of the sections of
/// [`ItemSection`].
#[derive(Default)]
struct ItemScan {
    /// The text of the heading that the parser reads now.
    heading: Option<String>,
    /// The section of the last heading.
    section: Option<ItemSection>,
    /// The level of the heading `Order` that is open now, from 1 to 6. Each
    /// heading of a lower level is inside `Order` until a heading of this
    /// level or a higher level ends it.
    order_level: Option<u8>,
    /// The level of the heading that the parser reads now.
    heading_level: u8,
    /// The list items that are open now, outer first.
    open: Vec<OpenItem>,
    /// The depth of the block quotes that are open now.
    quotes: u32,
    /// The start of the paragraph of `Files` that the parser reads now.
    paragraph: Option<usize>,
    /// The first paragraph of the section `Files` outside a list.
    files_paragraph: Option<Token>,
}

struct OpenItem {
    /// The section, if this is a top-level item of a section that matters.
    section: Option<ItemSection>,
    task: Option<bool>,
    codes: Vec<String>,
    first_list: Option<usize>,
}

impl ItemScan {
    fn step(
        &mut self,
        ev: &Event,
        range: &Range<usize>,
        body: &str,
        body_start: usize,
        lines: &LineIndex,
        out: &mut Vec<ListItem>,
    ) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                self.heading = Some(String::new());
                self.heading_level = *level as u8;
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(h) = self.heading.take() {
                    let named = ItemSection::from_heading(&h);
                    let level = self.heading_level;
                    self.order_level = match (named, self.order_level) {
                        (Some(ItemSection::Order), _) => Some(level),
                        (_, Some(order)) if level > order => Some(order),
                        _ => None,
                    };
                    // A heading below `Order` groups its tasks: the section
                    // goes on, also after a subsection with another name.
                    self.section = match (named, self.order_level) {
                        (None, Some(_)) => Some(ItemSection::Order),
                        _ => named,
                    };
                }
            }
            Event::Text(t) | Event::Code(t) if self.heading.is_some() => {
                if let Some(h) = self.heading.as_mut() {
                    h.push_str(t);
                }
            }
            Event::Start(Tag::BlockQuote(_)) => self.quotes += 1,
            Event::End(TagEnd::BlockQuote(_)) => self.quotes = self.quotes.saturating_sub(1),
            Event::Start(Tag::Paragraph) => {
                if self.section == Some(ItemSection::Files)
                    && self.open.is_empty()
                    && self.quotes == 0
                    && self.files_paragraph.is_none()
                {
                    self.paragraph = Some(range.start);
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some(start) = self.paragraph.take() {
                    let text = body[start..range.end]
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");
                    self.files_paragraph = Some(Token {
                        text,
                        line: lines.line(body_start + start),
                    });
                }
            }
            Event::Start(Tag::Item) => {
                let top = self.open.is_empty();
                self.open.push(OpenItem {
                    section: self.section.filter(|_| top),
                    task: None,
                    codes: Vec::new(),
                    first_list: None,
                });
            }
            Event::TaskListMarker(done) => {
                if let Some(item) = self.open.last_mut() {
                    item.task = Some(*done);
                }
            }
            Event::Start(Tag::List(_)) => {
                if let Some(item) = self.open.last_mut() {
                    item.first_list.get_or_insert(range.start);
                }
            }
            Event::Code(t) => {
                if let Some(item) = self.open.last_mut()
                    && item.first_list.is_none()
                    && !t.trim().is_empty()
                {
                    item.codes.push(t.trim().to_string());
                }
            }
            Event::End(TagEnd::Item) => {
                let Some(item) = self.open.pop() else { return };
                let Some(section) = item.section else { return };
                let slice = &body[range.clone()];
                let mut rest = strip_marker(slice);
                if item.task.is_some() {
                    rest = strip_task_box(rest);
                }
                let text_start = range.start + (slice.len() - rest.len());
                let text_end = item.first_list.unwrap_or(range.end).max(text_start);
                // The range ends after the last line that has text. A loose
                // list item can also own the blank lines that follow it.
                let kept = slice.trim_end().len();
                let end = range.start
                    + slice[kept..]
                        .find('\n')
                        .map_or(slice.len(), |i| kept + i + 1);
                out.push(ListItem {
                    section,
                    task: item.task,
                    text: body[text_start..text_end].trim().to_string(),
                    codes: item.codes,
                    nested: item.first_list.is_some(),
                    line: lines.line(body_start + range.start),
                    range: (body_start + range.start) as u32..(body_start + end) as u32,
                });
            }
            _ => {}
        }
    }
}

/// Returns the end of the sentence in `raw`: the end of the text without the
/// space and the markers (`aliases` and `verified-by`) at the end.
fn sentence_end(raw: &str) -> usize {
    let mut end = raw.trim_end().len();
    while raw[..end].ends_with("-->")
        && let Some(pos) = raw[..end].rfind("<!--")
    {
        // `<!-->` and `<!--->` end at once: they are comments with no marker.
        if pos + 4 > end - 3 {
            break;
        }
        let inner = raw[pos + 4..end - 3].trim();
        if !(inner.starts_with("aliases:") || inner.starts_with("verified-by:")) {
            break;
        }
        end = raw[..pos].trim_end().len();
    }
    end
}

/// Removes the marker of a list item (`-`, `*`, `+`, `1.` or `1)`) and the
/// space after it.
fn strip_marker(item: &str) -> &str {
    let s = item.trim_start();
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    let s = if s.starts_with(['-', '*', '+']) {
        &s[1..]
    } else if digits > 0 && s[digits..].starts_with(['.', ')']) {
        &s[digits + 1..]
    } else {
        s
    };
    s.trim_start()
}

/// Removes a task box (`[ ]` or `[x]`) and the space after it.
fn strip_task_box(s: &str) -> &str {
    ["[ ]", "[x]", "[X]"]
        .iter()
        .find_map(|b| s.strip_prefix(b))
        .unwrap_or(s)
        .trim_start()
}

/// Removes the marker of a list item and a task box.
fn strip_list_marker(item: &str) -> &str {
    strip_task_box(strip_marker(item))
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

/// Removes the markers `<!-- aliases: a#b, c#d -->` and
/// `<!-- verified-by: tests/a.rs, tests/b.rs -->` from a requirement, and
/// returns the aliases and the evidence paths that they list. The hash of a
/// requirement does not cover the markers.
pub fn strip_markers(raw: &str) -> (String, Vec<String>, Vec<String>) {
    let mut aliases = Vec::new();
    let mut verified_by = Vec::new();
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(pos) = rest.find("<!--") {
        let after = &rest[pos + 4..];
        let Some(close) = after.find("-->") else {
            break;
        };
        let inner = after[..close].trim();
        out.push_str(&rest[..pos]);
        let list = |l: &str| -> Vec<String> {
            l.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };
        if let Some(l) = inner.strip_prefix("aliases:") {
            aliases.extend(list(l));
        } else if let Some(l) = inner.strip_prefix("verified-by:") {
            verified_by.extend(list(l));
        } else {
            out.push_str(&rest[pos..pos + 4 + close + 3]);
        }
        rest = &after[close + 3..];
    }
    out.push_str(rest);
    (out, aliases, verified_by)
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

/// Collects template placeholders: `{{` and a character that is not a space,
/// then text without braces, then a character that is not a space and `}}`.
/// `${{ x }}` (GitHub Actions) and `{{ x }}` (Jinja) are not placeholders.
fn scan_placeholders(text: &str, line: u32, out: &mut Vec<Token>) {
    let mut rest = text;
    let mut offset = 0;
    while let Some(i) = rest.find("{{") {
        let start = offset + i;
        let after = &rest[i + 2..];
        let dollar = text[..start].ends_with('$');
        let end = after.find("}}");
        match end {
            Some(e)
                if !dollar
                    && e > 0
                    && !after[..e].contains(['{', '}'])
                    && !after.starts_with(char::is_whitespace)
                    && !after[..e].ends_with(char::is_whitespace) =>
            {
                out.push(Token {
                    text: format!("{{{{{}}}}}", &after[..e]),
                    line,
                });
                offset = start + 2 + e + 2;
                rest = &text[offset..];
            }
            _ => {
                offset = start + 2;
                rest = &text[offset..];
            }
        }
    }
}

/// True if `text` has a template placeholder (`{{text}}`).
pub fn has_placeholder(text: &str) -> bool {
    let mut found = Vec::new();
    scan_placeholders(text, 0, &mut found);
    !found.is_empty()
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

/// Removes the destinations of inline links and images (`](dest)`) and of
/// reference definitions (`[label]: dest`). The link text stays.
pub fn strip_link_destinations(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if t.starts_with('[')
            && let Some(i) = t.find("]: ")
        {
            out.push_str(&line[..line.len() - t.len() + i + 1]);
            if line.ends_with('\n') {
                out.push('\n');
            }
            continue;
        }
        let mut rest = line;
        while let Some(i) = rest.find("](") {
            out.push_str(&rest[..=i]);
            let after = &rest[i + 2..];
            let mut depth = 1;
            let close = after.char_indices().find_map(|(j, ch)| {
                match ch {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                (depth == 0).then_some(j)
            });
            match close {
                Some(j) => rest = &after[j + 1..],
                None => {
                    out.push('(');
                    rest = after;
                    break;
                }
            }
        }
        out.push_str(rest);
    }
    out
}

/// Returns true if an inline code span can be a path or an identifier. It has
/// 1 to 200 bytes with a letter, and no whitespace before the first `(`.
fn is_code_candidate(span: &str) -> bool {
    let head = span.split('(').next().unwrap_or("");
    !span.is_empty()
        && span.len() <= 200
        && span.bytes().any(|b| b.is_ascii_alphabetic())
        && !head.contains(char::is_whitespace)
}

/// Collects the words of prose that contain a `/` and a letter, without the
/// punctuation around them. A URL is not a word of this kind.
fn scan_path_words(text: &str, line: u32, out: &mut Vec<Token>) {
    for w in text.split_whitespace() {
        let w = w.trim_matches(|c: char| {
            matches!(
                c,
                '(' | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | '<'
                    | '>'
                    | ','
                    | ';'
                    | '"'
                    | '\''
                    | '!'
                    | '?'
                    | '*'
            )
        });
        let w = w.trim_end_matches(['.', ':']);
        if w.len() >= 3
            && w.len() <= 200
            && w.contains('/')
            && !w.contains("://")
            && w.bytes().any(|b| b.is_ascii_alphabetic())
        {
            out.push(Token {
                text: w.to_string(),
                line,
            });
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

    #[test]
    fn link_destinations_are_not_frozen_text() {
        assert_eq!(
            strip_link_destinations("See [a](../a.md#x) and ![i](i (1).png).\n[r]: ../r.md\n"),
            "See [a] and ![i].\n[r]\n"
        );
        let a = parse("# D\n\nUse [loans](../loans.md#k).\n");
        let b = parse("# D\n\nUse [loans](../circulation/loans.md#k).\n");
        let c = parse("# D\n\nUse [lending](../loans.md#k).\n");
        assert_ne!(a.chunks[0].hash, b.chunks[0].hash);
        assert_eq!(a.chunks[0].frozen, b.chunks[0].frozen);
        assert_ne!(a.chunks[0].frozen, c.chunks[0].frozen);
    }

    const DOC: &str = "---\nstatus: active\n---\n# Moisture sensors\n\nIntro with ADR-12 and `ADR-13` in code.\n\n## Requirements\n\n- **sensor-timeout**: When a moisture sensor times out, the controller SHALL close the valve. <!-- aliases: irrigation/old#ttl -->\n- **R-12**: The client MUST retry.\n  - nested note\n- plain item\n- **Note** this is not a key\n\n```md\n- **in-code**: ignored SHALL\n```\n\nSee [the guide](../guide.md#setup) and [[irrigation/valves#sensor-timeout@3f2a9c1b7e0d4a11]].\n\n## Requirements\n\n## Amendments\n\nText.\n";

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
                ("Moisture sensors", "moisture-sensors", 4),
                ("Requirements", "requirements", 8),
                ("Requirements", "requirements-1", 22),
                ("Amendments", "amendments", 24)
            ]
        );
        assert_eq!(d.title, "Moisture sensors");
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
            vec![("sensor-timeout", true), ("R-12", true), ("Note", false)]
        );
        let r = &d.reqs[0];
        assert_eq!(r.aliases, vec!["irrigation/old#ttl".to_string()]);
        assert_eq!(
            r.text,
            "When a moisture sensor times out, the controller SHALL close the valve."
        );
        assert_eq!(r.line, 10);
        assert_eq!(d.reqs[1].text, "The client MUST retry.");
        // The alias marker does not change the hash.
        let moved = DOC.replace(" <!-- aliases: irrigation/old#ttl -->", "");
        assert_eq!(parse(&moved).reqs[0].hash, r.hash);
    }

    #[test]
    fn legacy_requirements_without_a_colon() {
        let d = parse(
            "# S\n\n## Rules\n\n- **R-1** The client MUST retry.\n- **R-2** — The server SHALL log.\n- **R-3:** The cache MAY drop entries.\n- **R-4**\n- See **R-5** in the text.\n- **R-6.** The valve MUST close after the cycle.\n\n## Notes\n\n- **Note** prose with a bold label.\n",
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
                ("R-6", false, "The valve MUST close after the cycle."),
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
                    "irrigation/valves#sensor-timeout@3f2a9c1b7e0d4a11",
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

    #[test]
    fn template_placeholders() {
        let d = parse(
            "# T\n\n- **{{key}}**: When {{trigger}}, x.\n\nUse `{{code}}` and ${{ github.ref }} and {{ jinja }}.\n\n```\n{{block}}\n```\n",
        );
        let p: Vec<&str> = d.placeholders.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(p, ["{{key}}", "{{trigger}}"]);
    }

    fn rows(text: &str) -> Vec<(String, Vec<String>, u32)> {
        parse(text)
            .terms
            .into_iter()
            .map(|r| (r.term, r.forbidden, r.line))
            .collect()
    }

    fn row(term: &str, forbidden: &[&str], line: u32) -> (String, Vec<String>, u32) {
        (
            term.to_string(),
            forbidden.iter().map(|f| f.to_string()).collect(),
            line,
        )
    }

    #[test]
    fn a_table_of_terms_gives_its_rows() {
        let text = "# Terms\n\n| Term | Meaning | Forbidden synonyms |\n| --- | --- | --- |\n| Valve | A device that stops a flow. | gate, shutter ; door;, |\n| Zone | An area. | |\n";
        assert_eq!(
            rows(text),
            [
                row("Valve", &["gate", "shutter", "door"], 5),
                row("Zone", &[], 6)
            ]
        );
        // The columns can come in any order and in any case. The column of the
        // meaning is ignored, also if it mentions a synonym.
        let text = "| FORBIDDEN synonyms | meaning | term |\n| --- | --- | --- |\n| gate | Not a gate. | Valve |\n";
        assert_eq!(rows(text), [row("Valve", &["gate"], 3)]);
        // Inline code and emphasis in a cell are text. A cell has one space
        // for each run of white space.
        let text = "| Term | Forbidden synonyms |\n| --- | --- |\n| **Sensor head** | `probe`,  the   *tip* |\n";
        assert_eq!(rows(text), [row("Sensor head", &["probe", "the tip"], 3)]);
    }

    #[test]
    fn a_table_with_other_headers_gives_no_term() {
        for text in [
            "| Term | Meaning |\n| --- | --- |\n| Valve | A device. |\n",
            "| Name | Forbidden synonyms |\n| --- | --- |\n| Valve | gate |\n",
            "| Terms | Forbidden synonym |\n| --- | --- |\n| Valve | gate |\n",
            "| Term and Forbidden synonyms |\n| --- |\n| Valve |\n",
            "Term and Forbidden synonyms: Valve, gate\n",
        ] {
            assert!(rows(text).is_empty(), "{text}");
        }
        // A table of terms does not give its columns to the next table.
        let text = "| Term | Forbidden synonyms |\n| --- | --- |\n| Valve | gate |\n\n| A | B |\n| --- | --- |\n| x | y |\n";
        assert_eq!(rows(text), [row("Valve", &["gate"], 3)]);
    }

    #[test]
    fn each_table_of_terms_gives_rows_and_a_short_row_is_safe() {
        let text = "| Term | Forbidden synonyms |\n| --- | --- |\n| Valve | gate |\n| Pump |\n\n| Forbidden synonyms | Term |\n| --- | --- |\n| duct | Pipe |\n| | |\n";
        assert_eq!(
            rows(text),
            [
                row("Valve", &["gate"], 3),
                row("Pump", &[], 4),
                row("Pipe", &["duct"], 8)
            ]
        );
    }

    fn items(src: &str) -> Vec<(ItemSection, Option<bool>, String, u32, String)> {
        parse(src)
            .items
            .iter()
            .map(|i| {
                (
                    i.section,
                    i.task,
                    i.text.clone(),
                    i.line,
                    src[i.range.start as usize..i.range.end as usize].to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn items_under_each_of_the_6_headings() {
        let src = "# Change\n\n## Delta\n\n- ADD a#b: Text.\n- CHANGE a#c@0123456789abcdef: Text.\n\n## Interfaces\n\n- CHANGE api/http: Text.\n\n## Decisions\n\n- [[decisions/pump]]\n\n## Concerns\n\n- [ ] open concern\n- [x] resolved concern, so the author did it\n\n## Files\n\n- `src/valve.py`\n\n## Follow-ups\n\n- [ ] action\n- [X] done action\n\n## Other\n\n- not kept\n";
        let got = items(src);
        let want: Vec<(ItemSection, Option<bool>, &str, u32)> = vec![
            (ItemSection::Delta, None, "ADD a#b: Text.", 5),
            (
                ItemSection::Delta,
                None,
                "CHANGE a#c@0123456789abcdef: Text.",
                6,
            ),
            (ItemSection::Interfaces, None, "CHANGE api/http: Text.", 10),
            (ItemSection::Decisions, None, "[[decisions/pump]]", 14),
            (ItemSection::Concerns, Some(false), "open concern", 18),
            (
                ItemSection::Concerns,
                Some(true),
                "resolved concern, so the author did it",
                19,
            ),
            (ItemSection::Files, None, "`src/valve.py`", 23),
            (ItemSection::FollowUps, Some(false), "action", 27),
            (ItemSection::FollowUps, Some(true), "done action", 28),
        ];
        let got: Vec<_> = got
            .iter()
            .map(|(s, t, x, l, _)| (*s, *t, x.as_str(), *l))
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn items_under_order_and_open_questions() {
        let src = "# Plan\n\n## Order\n\n- [ ] t1: Step. Files: `a`.\n1. numbered\n\n### Open questions\n\n- [ ] one\n- [x] two, so the answer\n\n## Open question\n\n- not kept\n\n## Orders\n\n- not kept\n";
        let got: Vec<_> = items(src)
            .into_iter()
            .map(|(s, t, x, l, _)| (s, t, x, l))
            .collect();
        assert_eq!(
            got,
            [
                (
                    ItemSection::Order,
                    Some(false),
                    "t1: Step. Files: `a`.".to_string(),
                    5
                ),
                (ItemSection::Order, None, "numbered".to_string(), 6),
                (
                    ItemSection::OpenQuestions,
                    Some(false),
                    "one".to_string(),
                    10
                ),
                (
                    ItemSection::OpenQuestions,
                    Some(true),
                    "two, so the answer".to_string(),
                    11
                ),
            ]
        );
    }

    #[test]
    fn a_heading_names_a_section_by_its_exact_text_at_any_level() {
        let src = "# Delta\n\n- a\n\n### DELTA\n\n- b\n\n#### files\n\n- c\n\n## Delta notes\n\n- not kept\n\n## Follow ups\n\n- not kept\n\n## `Files`\n\n- d\n\n## Delta\n\n- e\n\n### Details\n\n- not kept, a heading ends the section\n\n## Plan\n\n1. not kept\n";
        let d = parse(src);
        let texts: Vec<&str> = d.items.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(texts, ["a", "b", "c", "d", "e"]);
        // Items before the first heading, and items in a table, are not kept.
        assert!(parse("- a\n\n| x |\n| - |\n| y |\n").items.is_empty());
    }

    #[test]
    fn a_nested_list_belongs_to_its_item_and_markers_can_differ() {
        let src = "## Concerns\n\n- [ ] first\n  - [ ] child one\n  - [x] child two\n    1. grandchild\n* [x] second\n+ third\n\n1. fourth\n2) fifth\n   continued on a line\n\n   a second paragraph\n";
        let got = items(src);
        let texts: Vec<(&str, Option<bool>)> = got.iter().map(|g| (g.2.as_str(), g.1)).collect();
        assert_eq!(
            texts,
            [
                ("first", Some(false)),
                ("second", Some(true)),
                ("third", None),
                ("fourth", None),
                (
                    "fifth\n   continued on a line\n\n   a second paragraph",
                    None
                ),
            ]
        );
        assert_eq!(
            got[0].4,
            "- [ ] first\n  - [ ] child one\n  - [x] child two\n    1. grandchild\n"
        );
        assert!(parse(src).items[0].nested && !parse(src).items[1].nested);
        // No range has a blank line at its end, and no range overlaps the next.
        for w in got.windows(2) {
            assert!(!w[0].4.ends_with("\n\n"), "{:?}", w[0].4);
        }
    }

    #[test]
    fn every_code_span_of_an_item_is_kept() {
        let src = "## Files\n\n- `a/b.rs` and `c.rs`\n- `package.json`, `pnpm-workspace.yaml`: the workspace.\n- see [`d.rs`](x.md)\n- plain\n- ``\n- `` and `f.rs`\n- text, then:\n  - `nested.rs`\n- one `g.rs`, then:\n  - `nested.rs`\n- ```double `tick` ```\n\n```md\n- in a fence `e.rs`\n```\n";
        let d = parse(src);
        let codes: Vec<Vec<&str>> = d
            .items
            .iter()
            .map(|i| i.codes.iter().map(String::as_str).collect())
            .collect();
        assert_eq!(
            codes,
            [
                vec!["a/b.rs", "c.rs"],
                vec!["package.json", "pnpm-workspace.yaml"],
                vec!["d.rs"],
                vec![],
                vec![],
                vec!["f.rs"],
                vec![],
                vec!["g.rs"],
                vec!["double `tick`"]
            ]
        );
    }

    #[test]
    fn a_heading_inside_order_groups_its_tasks() {
        let src = "# Plan\n\n## Order\n\n- [ ] t1: One.\n\n### Part two\n\n- [ ] t2: Two.\n\n#### Deeper\n\n- [x] t3: Three.\n\n### Open questions\n\n- [ ] q\n\n### Part three\n\n- [ ] t4: Four.\n\n## Risks\n\n- [ ] not kept\n\n### Under risks\n\n- [ ] not kept\n";
        let got: Vec<_> = items(src)
            .into_iter()
            .map(|(s, t, x, _, _)| (s, t, x))
            .collect();
        assert_eq!(
            got,
            [
                (ItemSection::Order, Some(false), "t1: One.".to_string()),
                (ItemSection::Order, Some(false), "t2: Two.".to_string()),
                (ItemSection::Order, Some(true), "t3: Three.".to_string()),
                (ItemSection::OpenQuestions, Some(false), "q".to_string()),
                (ItemSection::Order, Some(false), "t4: Four.".to_string()),
            ]
        );
        // A heading of the same level or above ends `Order`. A section other
        // than `Order` ends at any heading.
        let src = "## Order\n\n- [ ] t1: a\n\n## Notes\n\n- [ ] not kept\n\n## Order\n\n- [ ] t2: b\n\n## Files\n\n- `x`\n\n### Details\n\n- not kept\n";
        let got: Vec<(ItemSection, String)> = items(src)
            .into_iter()
            .map(|(s, _, x, _, _)| (s, x))
            .collect();
        assert_eq!(
            got,
            [
                (ItemSection::Order, "t1: a".to_string()),
                (ItemSection::Order, "t2: b".to_string()),
                (ItemSection::Files, "`x`".to_string()),
            ]
        );
    }

    #[test]
    fn item_ranges_are_byte_ranges_with_crlf_and_a_bom() {
        let lf = "# C\n\n## Delta\n\n- ADD a#b: Wörter.\n  more\n- [ ] REMOVE a#c@0123456789abcdef\n  - nested\n- last, no final newline";
        let crlf = lf.replace('\n', "\r\n");
        let bom = format!("\u{feff}{lf}");
        for src in [lf.to_string(), crlf, bom] {
            let d = parse(&src);
            assert_eq!(d.items.len(), 3, "{src:?}");
            let slices: Vec<&str> = d
                .items
                .iter()
                .map(|i| &src[i.range.start as usize..i.range.end as usize])
                .collect();
            let eol = if src.contains('\r') { "\r\n" } else { "\n" };
            assert_eq!(
                slices[0],
                format!("- ADD a#b: Wörter.{eol}  more{eol}"),
                "{src:?}"
            );
            assert_eq!(
                slices[1],
                format!("- [ ] REMOVE a#c@0123456789abcdef{eol}  - nested{eol}")
            );
            assert_eq!(slices[2], "- last, no final newline");
            // The ranges tile the list: each starts where the previous ended.
            assert_eq!(d.items[0].range.end, d.items[1].range.start);
            assert_eq!(d.items[1].range.end, d.items[2].range.start);
            assert_eq!(d.items[2].range.end as usize, src.len());
            let lines: Vec<u32> = d.items.iter().map(|i| i.line).collect();
            assert_eq!(lines, [5, 7, 9]);
            // The text has no marker, no task box and no nested list.
            assert_eq!(d.items[1].text, "REMOVE a#c@0123456789abcdef");
            assert!(d.items[0].text.starts_with("ADD a#b: Wörter."));
        }
    }

    #[test]
    fn a_range_with_front_matter_is_a_range_of_the_whole_file() {
        let src = "---\r\nstatus: draft\r\n---\r\n# S\r\n\r\n## Concerns\r\n\r\n- [ ] one\r\n- [x] two\r\n";
        let d = parse(src);
        let got: Vec<&str> = d
            .items
            .iter()
            .map(|i| &src[i.range.start as usize..i.range.end as usize])
            .collect();
        assert_eq!(got, ["- [ ] one\r\n", "- [x] two\r\n"]);
        assert_eq!(d.items[0].line, 8);
    }

    fn sentences(src: &str) -> Vec<(String, String)> {
        parse(src)
            .reqs
            .iter()
            .map(|r| {
                (
                    r.key.clone(),
                    src[r.sentence.start as usize..r.sentence.end as usize].to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn the_sentence_span_has_no_key_no_marker_and_no_edge_space() {
        let src = "## Requirements\n\n- **a**: The valve SHALL close.\n- **b**:   The valve SHALL open. <!-- aliases: x#y -->\n- **c**: Text. <!-- verified-by: tests/a.rs, tests/b.rs --> <!-- aliases: x#z -->  \n- **d:** Key with the colon inside.\n- **e** Legacy form.\n  - nested note\n- **f**: First line\n  second line. <!-- aliases: x#w -->\n  - nested\n- **g**:\n- **h**: <!-- aliases: x#v -->\n- **i**: Keeps <!-- other --> comment. <!-- aliases: x#u -->\n";
        let got = sentences(src);
        let want = [
            ("a", "The valve SHALL close."),
            ("b", "The valve SHALL open."),
            ("c", "Text."),
            ("d", "Key with the colon inside."),
            ("e", "Legacy form."),
            ("f", "First line\n  second line."),
            ("g", ""),
            ("h", ""),
            ("i", "Keeps <!-- other --> comment."),
        ];
        assert_eq!(
            got,
            want.map(|(k, s)| (k.to_string(), s.to_string())).to_vec()
        );
    }

    #[test]
    fn the_sentence_span_has_the_hash_of_the_requirement() {
        let lf = "## Requirements\n\n- **a**: The valve SHALL close. <!-- aliases: x#y -->\n- **b**: Wörter [a](../a.md#x)\n  wrap. <!-- verified-by: t.rs --> <!-- aliases: x#z -->\n  - nested\n- **c**: Plain.\n";
        for src in [lf.to_string(), lf.replace('\n', "\r\n")] {
            let d = parse(&src);
            assert_eq!(d.reqs.len(), 3);
            for r in &d.reqs {
                let span = &src[r.sentence.start as usize..r.sentence.end as usize];
                assert_eq!(hash_normalized(span), r.hash, "{span:?}");
                assert_eq!(
                    hash_normalized(&strip_link_destinations(span)),
                    r.frozen,
                    "{span:?}"
                );
                // The span is inside the item.
                assert!(r.start <= r.sentence.start && r.sentence.end <= r.end);
            }
        }
    }

    #[test]
    fn a_broken_comment_at_the_end_does_not_stop_the_parse() {
        for tail in [
            "<!-->",
            "<!--->",
            "-->",
            " -->",
            "<!-- aliases: a <!-- b -->",
            "<!-- aliases: a -->  -->",
            "<!-- aliases: a",
            "<!-- verified-by: -->",
            "<!----> <!-- aliases: a -->",
            "<!-- <!-- aliases: a -->",
        ] {
            let src = format!("## Requirements\n\n- **k**: Text {tail}\n- **l**: Other.\n");
            let d = parse(&src);
            assert_eq!(d.reqs.len(), 2, "{tail}");
            for r in &d.reqs {
                assert!(r.sentence.start <= r.sentence.end, "{tail}");
                assert!(
                    r.start <= r.sentence.start && r.sentence.end <= r.end,
                    "{tail}"
                );
                let _ = &src[r.sentence.start as usize..r.sentence.end as usize];
            }
        }
        let d = parse("- **k**: Text <!-->\n");
        assert_eq!(
            &"- **k**: Text <!-->\n"
                [d.reqs[0].sentence.start as usize..d.reqs[0].sentence.end as usize],
            "Text <!-->"
        );
    }

    #[test]
    fn a_front_matter_offset_moves_the_sentence_span() {
        let src = "---\nstatus: active\n---\n# T\n\n- **k**: Text.\n";
        let r = &parse(src).reqs[0];
        assert_eq!(
            &src[r.sentence.start as usize..r.sentence.end as usize],
            "Text."
        );
    }

    fn paragraph(src: &str) -> Option<(String, u32)> {
        parse(src).files_paragraph.map(|t| (t.text, t.line))
    }

    #[test]
    fn the_first_paragraph_of_files_is_kept_on_one_line_with_its_line() {
        assert_eq!(
            paragraph("# P\n\n## Files\n\nNone.\n"),
            Some(("None.".to_string(), 5))
        );
        // Lines join with one space. The text is as it is in the source.
        assert_eq!(
            paragraph("# P\n\n## files\n\nThe `dose` code,\n   and the test.\n\nMore.\n"),
            Some(("The `dose` code, and the test.".to_string(), 5))
        );
        // The text is in the file with its front matter.
        assert_eq!(
            paragraph("---\nstatus: draft\n---\n# P\n\n## Files\n\nnone\n"),
            Some(("none".to_string(), 8))
        );
        // A list item is no paragraph, also a loose item. A paragraph after
        // the list is the first paragraph that is not in a list.
        assert_eq!(paragraph("## Files\n\n- `a`\n\n  more\n"), None);
        assert_eq!(paragraph("## Files\n\n1. `a`\n- `b`\n"), None);
        assert_eq!(
            paragraph("## Files\n\n- `a`\n\nAfter the list.\n"),
            Some(("After the list.".to_string(), 5))
        );
        // A block quote, another section, a table and a section with no
        // paragraph give none.
        assert_eq!(paragraph("## Files\n\n> Note.\n"), None);
        assert_eq!(paragraph("## Risks\n\nText.\n\n## Files\n"), None);
        assert_eq!(paragraph("## Order\n\nText.\n"), None);
        assert_eq!(paragraph("## Files\n\n| a |\n| - |\n| b |\n"), None);
        assert_eq!(paragraph("Text.\n\n## Files and paths\n\nText.\n"), None);
        // Only the first paragraph, and the first section.
        assert_eq!(
            paragraph("## Files\n\nOne.\n\nTwo.\n\n## Files\n\nThree.\n"),
            Some(("One.".to_string(), 3))
        );
    }

    #[test]
    fn items_under_hours_are_not_kept() {
        let d =
            parse("# P\n\n## Hours\n\n- 1.5: t1 code\n- 2: review\n- [ ] 0.5\n\n## Other\n\n- x\n");
        assert!(d.items.is_empty());
        assert_eq!(ItemSection::from_heading("hours"), None);
        assert_eq!(ItemSection::ALL.len(), 8);
    }

    #[test]
    fn parser_version_is_16() {
        assert_eq!(PARSER_VERSION, 16);
    }
}
