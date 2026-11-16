//! A small Markdown lint for the files that spec-dna generates.
//!
//! `md_lint` checks 9 rules of markdownlint: MD009, MD010, MD012, MD022,
//! MD031, MD032, MD040, MD047 and MD058. The pre-commit hook of this
//! repository runs `markdownlint-cli2` on the files that are in git. The files
//! that `init`, `new`, `mv` and `index` write at run time are not in git, so
//! the hook does not see them. A test runs this helper on them.
//!
//! Mapping to `.markdownlint-cli2.yaml`: `default: true` enables each rule.
//! The file turns off only MD013 and MD041, so the helper does not check
//! them. MD024 and MD033 have options, and the helper does not check them.
//! The helper keeps the default option of each rule that it checks. For
//! MD009 this means 0 or 2 trailing spaces. For MD012 it means at most 1
//! blank line.
//!
//! Rules that need a full Markdown parser are not checked: MD001 to MD005,
//! MD007, MD011, MD014, MD018 to MD021, MD023 to MD030, MD033 to MD039, MD042
//! to MD046, MD048 to MD057, MD059 and MD060. MD060 (table column style) broke
//! the table of `spec-dna index` when a row had an empty cell written as
//! `|  |`. The index now writes `| |`, and `tests/generated_lint.rs` checks
//! that no generated table row has `|  |`. Run `markdownlint-cli2` on a sample
//! when you add a new kind of text.
//!
//! The helper has these limits. A limit can hide a problem or give a false
//! alarm, and no generated file shows either today:
//!
//! - It reads ATX headings only (`# Title`), not setext headings.
//! - It does not read a block quote, so it misses a list in a block quote.
//! - It does not read an indented code block or an HTML block, so it reads a
//!   list marker inside an HTML block (`<div>`) as a list item. The linter
//!   does not.
//!
//! What counts as a blank line. markdownlint reads a line that holds only an
//! HTML comment as a blank line for MD022, MD031, MD032 and MD058. A comment
//! can span many lines, and each line of it is blank for these rules. A line
//! that holds only whitespace is blank for all the rules. MD012 counts only
//! the lines that hold only whitespace. A comment line is not blank for MD012.
//! The marker lines of spec-dna (`<!-- spec-dna:begin ... -->`) are such
//! comments. A marker directly above a heading gives no MD022. A check with
//! markdownlint-cli2 v0.24.0 (markdownlint v0.42.0) shows this, with the
//! other rules of this page, on 119 small texts.
//!
//! The front matter and the lines of a fenced code block are skipped, with
//! these exceptions: the fence lines are read for MD031 and MD040, and a hard
//! tab in code is a MD010 problem (the rule has `code_blocks: true` by
//! default). MD009 and MD012 skip the code. MD009 and MD010 skip the text of
//! an HTML comment. MD012 skips the blank lines right after the front matter.
//! Line numbers count from 1, and they count the front matter, as the linter
//! does.

/// What a line is, after the pass that reads fences and comments.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    /// A line of the front matter.
    Front,
    /// A line that holds only whitespace.
    Blank,
    /// A line that holds only an HTML comment, or a line inside a comment.
    Comment,
    /// A line inside a fenced code block.
    Code,
    /// The line that opens a fenced code block. `bare` is true if it has no
    /// info string.
    FenceOpen { bare: bool },
    /// The line that closes a fenced code block.
    FenceClose,
    /// An ATX heading.
    Heading,
    /// A thematic break, or a block quote: it ends a list, and it is not text.
    Other,
    /// A list item.
    Item(Item),
    /// A line of text. It can be a row of a table.
    Text,
}

/// The facts of a list item that the list pass needs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Item {
    /// The number of spaces before the marker.
    indent: usize,
    /// The column where the text of the item starts.
    content: usize,
    /// The bullet character, or the delimiter of an ordered marker.
    marker: char,
    /// True for an ordered marker.
    ordered: bool,
    /// The number of an ordered marker.
    number: u64,
    /// True if the item has no text.
    empty: bool,
}

/// Returns each problem of `text` as the line number and the rule name,
/// sorted by line and without duplicates. An empty list means that the text
/// passes.
pub fn md_lint(text: &str) -> Vec<(u32, &'static str)> {
    let lines: Vec<&str> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    let kinds = classify(&lines);
    let first = kinds.iter().position(|k| *k != Kind::Front).unwrap_or(0);
    let mut out: Vec<(u32, &'static str)> = Vec::new();
    let mut add = |index: usize, rule: &'static str| out.push((index as u32 + 1, rule));

    // MD047: the last line is empty, so the text ends with 1 newline.
    if lines.last().is_some_and(|l| !l.trim().is_empty()) {
        add(lines.len() - 1, "MD047");
    }
    for (i, line) in lines.iter().enumerate() {
        let kind = kinds[i];
        // The linter does not read the text of a comment for MD009 and MD010.
        if matches!(kind, Kind::Front | Kind::Comment) {
            continue;
        }
        // MD010: a hard tab. The rule reads the lines of code too.
        if line.contains('\t') {
            add(i, "MD010");
        }
        // MD009: trailing spaces, other than 0 or 2. A fenced code block has
        // no check.
        if kind != Kind::Code {
            let trailing = line.len() - line.trim_end_matches([' ', '\t']).len();
            if trailing != 0 && trailing != 2 {
                add(i, "MD009");
            }
        }
        // MD012: 2 blank lines in a row, outside code. The linter does not
        // count the blank lines right after the front matter.
        if kind == Kind::Blank && i > 0 && kinds[i - 1] == Kind::Blank {
            let run_start = (0..i).rev().take_while(|j| kinds[*j] == Kind::Blank).last();
            if !(first > 0 && run_start == Some(first)) {
                add(i, "MD012");
            }
        }
        // MD040: a fence with no language.
        if kind == (Kind::FenceOpen { bare: true }) {
            add(i, "MD040");
        }
        let blank_before = i == first || is_blank_like(kinds[i - 1]);
        let blank_after = i + 1 >= lines.len() || is_blank_like(kinds[i + 1]);
        match kind {
            // MD022: a heading has a blank line above and below.
            Kind::Heading if !blank_before || !blank_after => add(i, "MD022"),
            // MD031: a fence has a blank line above (open) and below (close).
            Kind::FenceOpen { .. } if !blank_before => add(i, "MD031"),
            Kind::FenceClose if !blank_after => add(i, "MD031"),
            _ => {}
        }
    }
    lists(&lines, &kinds, first, &mut add);
    tables(&lines, &kinds, first, &mut add);
    out.sort_unstable();
    out.dedup();
    out
}

/// True for a line that markdownlint reads as blank around a block.
fn is_blank_like(kind: Kind) -> bool {
    matches!(kind, Kind::Blank | Kind::Comment)
}

/// Reads the front matter, the fenced code blocks and the HTML comments, then
/// gives each line its kind.
fn classify(lines: &[&str]) -> Vec<Kind> {
    let mut kinds = Vec::with_capacity(lines.len());
    let mut start = 0;
    if lines.first() == Some(&"---")
        && let Some(end) = lines.iter().skip(1).position(|l| *l == "---")
    {
        start = end + 2;
        kinds.extend(std::iter::repeat_n(Kind::Front, start));
    }
    // The open fence: its character and its length.
    let mut fence: Option<(char, usize)> = None;
    let mut in_comment = false;
    for line in &lines[start..] {
        let trimmed = line.trim();
        if let Some((ch, len)) = fence {
            let run = trimmed.chars().take_while(|c| *c == ch).count();
            if run >= len && trimmed.chars().skip(run).all(char::is_whitespace) {
                fence = None;
                kinds.push(Kind::FenceClose);
            } else {
                kinds.push(Kind::Code);
            }
            continue;
        }
        if in_comment {
            match line.rfind("-->") {
                Some(at) => {
                    in_comment = false;
                    let rest = &line[at + 3..];
                    kinds.push(if rest.trim().is_empty() {
                        Kind::Comment
                    } else {
                        Kind::Text
                    });
                }
                None => kinds.push(Kind::Comment),
            }
            continue;
        }
        if trimmed.is_empty() {
            kinds.push(Kind::Blank);
            continue;
        }
        if let Some((ch, len, info)) = fence_open(line) {
            fence = Some((ch, len));
            kinds.push(Kind::FenceOpen {
                bare: info.is_empty(),
            });
            continue;
        }
        match strip_comments(line) {
            Stripped::Whole => kinds.push(Kind::Comment),
            Stripped::Open(blank_before) => {
                in_comment = true;
                kinds.push(if blank_before {
                    Kind::Comment
                } else {
                    Kind::Text
                });
            }
            Stripped::Text => kinds.push(block_kind(line)),
        }
    }
    kinds
}

/// What is left of a line when the HTML comments go.
enum Stripped {
    /// The line has text outside of a comment, or no comment.
    Text,
    /// The line has only comments (at least 1) and whitespace.
    Whole,
    /// A comment starts and does not end on the line. The flag is true if
    /// only whitespace is before the comment.
    Open(bool),
}

fn strip_comments(line: &str) -> Stripped {
    let mut rest = line;
    let mut left = String::new();
    let mut seen = false;
    while let Some(at) = rest.find("<!--") {
        left.push_str(&rest[..at]);
        match rest[at + 4..].find("-->") {
            Some(end) => {
                seen = true;
                rest = &rest[at + 4 + end + 3..];
            }
            None => return Stripped::Open(left.trim().is_empty()),
        }
    }
    left.push_str(rest);
    if seen && left.trim().is_empty() {
        Stripped::Whole
    } else {
        Stripped::Text
    }
}

/// Reads the line that opens a fence: the fence character, its length and the
/// info string. A backtick fence cannot have a backtick in its info string.
fn fence_open(line: &str) -> Option<(char, usize, &str)> {
    let t = line.trim_start_matches(' ');
    let ch = t.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let len = t.chars().take_while(|c| *c == ch).count();
    let info = t[len..].trim();
    if len < 3 || (ch == '`' && info.contains('`')) {
        return None;
    }
    Some((ch, len, info))
}

/// The kind of a line that is not blank, a comment or a fence.
fn block_kind(line: &str) -> Kind {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    let t = line.trim_start_matches(' ');
    if indent <= 3 && t.starts_with('#') {
        let hashes = t.chars().take_while(|c| *c == '#').count();
        let after = &t[hashes..];
        if hashes <= 6 && (after.is_empty() || after.starts_with([' ', '\t'])) {
            return Kind::Heading;
        }
    }
    if indent <= 3 {
        let marks: String = t.chars().filter(|c| !c.is_whitespace()).collect();
        let first = marks.chars().next().unwrap_or(' ');
        if marks.len() >= 3 && matches!(first, '-' | '*' | '_') && marks.chars().all(|c| c == first)
        {
            return Kind::Other;
        }
        if t.starts_with('>') {
            return Kind::Other;
        }
    }
    item(indent, t).map_or(Kind::Text, Kind::Item)
}

/// Reads a list marker at the start of `t`, which has `indent` spaces before.
fn item(indent: usize, t: &str) -> Option<Item> {
    let mut chars = t.chars();
    let first = chars.next()?;
    let (width, marker, ordered, number) = if matches!(first, '-' | '+' | '*') {
        (1, first, false, 0)
    } else if first.is_ascii_digit() {
        let digits = t.chars().take_while(char::is_ascii_digit).count();
        let delimiter = t[digits..].chars().next()?;
        if digits > 9 || !matches!(delimiter, '.' | ')') {
            return None;
        }
        (digits + 1, delimiter, true, t[..digits].parse().ok()?)
    } else {
        return None;
    };
    let after = &t[width..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    let spaces = after.chars().take_while(|c| *c == ' ').count();
    Some(Item {
        indent,
        content: indent + width + if spaces == 0 || spaces > 4 { 1 } else { spaces },
        marker,
        ordered,
        number,
        empty: after.trim().is_empty(),
    })
}

/// A list that the pass has open.
struct Open {
    base: Item,
    /// The index of the last line of the list.
    last: usize,
    /// True if a blank line came after the last line of the list.
    gap: bool,
    /// True while an indented fence of the list is open.
    in_fence: bool,
}

/// MD032: a list has a blank line above and below. A line of text right
/// after a list item is part of the item (a lazy continuation), so only a
/// block of another kind ends the list with no blank line. The problem is on
/// the first line of the list (above) or on its last line (below).
fn lists(lines: &[&str], kinds: &[Kind], first: usize, add: &mut impl FnMut(usize, &'static str)) {
    let mut open: Option<Open> = None;
    for i in first..kinds.len() {
        let kind = kinds[i];
        if let Some(list) = open.as_mut() {
            if list.in_fence {
                list.last = i;
                list.in_fence = kind != Kind::FenceClose;
                continue;
            }
            if is_blank_like(kind) {
                list.gap = true;
                continue;
            }
        }
        let mut ended = None;
        if let Some(list) = open.as_mut() {
            let content = list.base.content;
            let indent = lines[i].chars().take_while(|c| *c == ' ').count();
            let continues = match kind {
                Kind::Item(it) => {
                    // A marker of another kind at the same level starts a
                    // new list.
                    it.indent >= content
                        || (it.indent >= list.base.indent
                            && it.marker == list.base.marker
                            && it.ordered == list.base.ordered)
                }
                _ if indent >= content => true,
                Kind::Text => !list.gap,
                _ => false,
            };
            if continues {
                list.last = i;
                list.gap = false;
                list.in_fence = matches!(kind, Kind::FenceOpen { .. });
                continue;
            }
            ended = Some((list.last, list.gap));
        }
        if let Some((last, gap)) = ended {
            open = None;
            if !gap {
                add(last, "MD032");
            }
        }
        if let Kind::Item(it) = kind {
            let prev_is_text = i > first && kinds[i - 1] == Kind::Text;
            // Only an item with a number 1 and with text can interrupt a
            // paragraph. Else the line is text.
            let interrupts = !prev_is_text || (!it.empty && (!it.ordered || it.number == 1));
            if it.indent <= 3 && interrupts {
                if i > first && !is_blank_like(kinds[i - 1]) {
                    add(i, "MD032");
                }
                open = Some(Open {
                    base: it,
                    last: i,
                    gap: false,
                    in_fence: false,
                });
            }
        }
    }
}

/// MD058: a table has a blank line above and below. A table starts at a
/// header row that has a delimiter row after it. A line of text after the
/// delimiter row is a row too. Another block ends the table.
fn tables(lines: &[&str], kinds: &[Kind], first: usize, add: &mut impl FnMut(usize, &'static str)) {
    let mut i = first;
    while i + 1 < lines.len() {
        let is_header = kinds[i] == Kind::Text
            && kinds[i + 1] == Kind::Text
            && lines[i].contains('|')
            && delimiter_row(lines[i + 1], cells(lines[i]));
        if !is_header {
            i += 1;
            continue;
        }
        if i > first && !is_blank_like(kinds[i - 1]) {
            add(i, "MD058");
        }
        let mut end = i + 1;
        while end + 1 < lines.len() && kinds[end + 1] == Kind::Text {
            end += 1;
        }
        if end + 1 < lines.len() && !is_blank_like(kinds[end + 1]) {
            add(end, "MD058");
        }
        i = end + 1;
    }
}

/// The number of cells of a table row.
fn cells(row: &str) -> usize {
    let t = row.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').count()
}

/// True if `row` is a delimiter row with `count` cells: each cell is dashes
/// with an optional colon at each end.
fn delimiter_row(row: &str, count: usize) -> bool {
    if !row.contains('|') || cells(row) != count {
        return false;
    }
    let t = row.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').all(|cell| {
        let cell = cell.trim();
        let cell = cell.strip_prefix(':').unwrap_or(cell);
        let cell = cell.strip_suffix(':').unwrap_or(cell);
        !cell.is_empty() && cell.chars().all(|c| c == '-')
    })
}
