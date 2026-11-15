//! The edits of the text of `spec-dna.toml`, for `upgrade` (D138). Each edit is
//! a splice at a span that the `toml` crate gives, so each byte outside the
//! splice stays: each comment, the order of the keys and the line endings. The
//! module has no new dependency.
//!
//! An edit reads the new text again, and the table that it gives must be the
//! table that the edit expects. A form that an edit cannot change, or a result
//! that differs from the expected one, stops with `Fatal::Config` (exit code 3)
//! and the message of D138: the person writes that form by hand, then runs
//! `spec-dna upgrade` again. The functions never write a file.
//!
//! A path names a key: `[k("budgets"), k("guide")]` is `[budgets.guide]`, and
//! `[k("roots"), at(1), k("type")]` is the key `type` of the second `[[roots]]`.

use crate::repo::Fatal;
use toml::de::{DeTable, DeValue};
use toml::{Table, Value};

/// An insertion into the text of the configuration.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Splice {
    pub(crate) at: usize,
    pub(crate) text: String,
}

/// Writes a TOML string with double quotes.
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The line ending of a text: `\r\n` if its first line ends so, else `\n`.
pub(crate) fn line_ending(text: &str) -> &'static str {
    match text.find('\n') {
        Some(i) if text[..i].ends_with('\r') => "\r\n",
        _ => "\n",
    }
}

/// Appends tables at the end of a text, after one empty line. The tables
/// have `\n` line endings. This function gives them the ending `eol`.
pub(crate) fn append_tables(out: &mut String, tables: &str, eol: &str) {
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(eol);
    }
    let blank = format!("{eol}{eol}");
    if !out.is_empty() && !out.ends_with(&blank) && !out.ends_with("\n\n") {
        out.push_str(eol);
    }
    out.push_str(&tables.replace('\n', eol));
}

/// One step of a path: a key, or the index of an entry of an array of tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seg<'a> {
    Key(&'a str),
    At(usize),
}

/// The step of a key.
pub fn k(name: &str) -> Seg<'_> {
    Seg::Key(name)
}

/// The step of an entry of an array of tables.
pub fn at(index: usize) -> Seg<'static> {
    Seg::At(index)
}

// --- edits ---

/// A replacement of the bytes `start..end` of the text.
#[derive(Debug)]
struct Edit {
    start: usize,
    end: usize,
    text: String,
}

impl Edit {
    fn insert(at: usize, text: String) -> Edit {
        Edit {
            start: at,
            end: at,
            text,
        }
    }
}

/// The text with the edits applied. The edits must not overlap.
fn apply(text: &str, mut edits: Vec<Edit>) -> Option<String> {
    edits.sort_by_key(|e| (e.start, e.end));
    if edits.windows(2).any(|w| w[0].end > w[1].start) {
        return None;
    }
    let mut out = text.to_string();
    for e in edits.iter().rev() {
        out.replace_range(e.start..e.end, &e.text);
    }
    Some(out)
}

/// A path as text, for a message: `roots[1].type`.
fn show(path: &[Seg]) -> String {
    let mut out = String::new();
    for seg in path {
        match seg {
            Seg::Key(n) if out.is_empty() => out.push_str(n),
            Seg::Key(n) => out.push_str(&format!(".{n}")),
            Seg::At(i) => out.push_str(&format!("[{i}]")),
        }
    }
    out
}

/// The refusal of D138: the person writes the form by hand.
fn by_hand(path: &[Seg], form: &str) -> Fatal {
    Fatal::Config(format!(
        "`{}`: upgrade cannot edit this form; write it as {form} by hand, commit the change, then run `spec-dna upgrade` again",
        show(path)
    ))
}

fn missing(path: &[Seg]) -> Fatal {
    Fatal::Config(format!("`{}`: no such key in spec-dna.toml", show(path)))
}

fn exists(path: &[Seg], name: &str) -> Fatal {
    Fatal::Config(format!(
        "`{}`: `{name}` exists already; remove one of them by hand, commit the change, then run `spec-dna upgrade` again",
        show(path)
    ))
}

const FORM_LINE: &str = "`key = value` on a line of its own";
const FORM_ARRAY_LINE: &str =
    "an array on one line, with the same names that it has now and no comment inside it";
const FORM_INLINE: &str = "an inline table, such as `key = { a = 1 }`";

/// The form of a table with a header, with the header of `path`: `[roots.x]`,
/// or `[[roots]]` for an entry of an array of tables.
fn form_header(path: &[Seg]) -> String {
    let keys: Vec<&str> = path
        .iter()
        .filter_map(|s| match s {
            Seg::Key(n) => Some(*n),
            Seg::At(_) => None,
        })
        .collect();
    let name = keys.join(".");
    let header = if matches!(path.last(), Some(Seg::At(_))) {
        format!("[[{name}]]")
    } else {
        format!("[{name}]")
    };
    format!("a table with a header, such as `{header}`")
}

fn parse_doc(text: &str) -> Result<toml::Spanned<DeTable<'_>>, Fatal> {
    DeTable::parse(text).map_err(|e| Fatal::Config(format!("spec-dna.toml does not parse: {e}")))
}

/// A key of the file: the span of its name and its value.
struct Found<'d, 'a> {
    key: Option<std::ops::Range<usize>>,
    value: &'d toml::Spanned<DeValue<'a>>,
}

/// Finds the node at `path`. The empty path has no node.
fn locate<'d, 'a>(root: &'d DeTable<'a>, path: &[Seg]) -> Option<Found<'d, 'a>> {
    let mut cur: Option<Found<'d, 'a>> = None;
    for seg in path {
        match *seg {
            Seg::Key(n) => {
                let table = match &cur {
                    None => root,
                    Some(f) => match f.value.get_ref() {
                        DeValue::Table(t) => t,
                        _ => return None,
                    },
                };
                let (key, value) = table.iter().find(|(k, _)| k.get_ref().as_ref() == n)?;
                cur = Some(Found {
                    key: Some(key.span()),
                    value,
                });
            }
            Seg::At(i) => {
                let f = cur.as_ref()?;
                let DeValue::Array(items) = f.value.get_ref() else {
                    return None;
                };
                cur = Some(Found {
                    key: None,
                    value: items.get(i)?,
                });
            }
        }
    }
    cur
}

fn line_start(text: &str, pos: usize) -> usize {
    text[..pos].rfind('\n').map_or(0, |i| i + 1)
}

/// The position after the line that holds `pos`, with its line ending.
fn line_end(text: &str, pos: usize) -> usize {
    text[pos..].find('\n').map_or(text.len(), |i| pos + i + 1)
}

/// True if the line of `pos` is a header line (`[x]` or `[[x]]`).
fn in_header_line(text: &str, pos: usize) -> bool {
    text[line_start(text, pos)..].trim_start().starts_with('[')
}

/// True if the value is a table that a header writes.
fn is_header_table(text: &str, f: &Found) -> bool {
    matches!(f.value.get_ref(), DeValue::Table(_)) && text[f.value.span()].starts_with('[')
}

/// True if the value is an array of tables (`[[x]]`).
fn is_table_array(text: &str, f: &Found) -> bool {
    let DeValue::Array(items) = f.value.get_ref() else {
        return false;
    };
    items
        .iter()
        .any(|e| matches!(e.get_ref(), DeValue::Table(_)) && text[e.span()].starts_with("[["))
}

/// The key as a part of a header or a dotted key: bare if it can be, else quoted.
pub(crate) fn key_text(name: &str) -> String {
    let bare = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    if bare { name.to_string() } else { quote(name) }
}

/// The new name in the quote style of the old text of the key.
fn key_like(old: &str, new: &str) -> String {
    let plain = |c: char| !c.is_control() && c != '\'';
    match old.chars().next() {
        Some('"') => quote(new),
        Some('\'') if new.chars().all(plain) => format!("'{new}'"),
        _ => key_text(new),
    }
}

/// The text of a value in the inline form.
pub(crate) fn render_value(v: &Value) -> String {
    match v {
        Value::String(s) => quote(s),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) if f.is_nan() => "nan".into(),
        Value::Float(f) if f.is_infinite() => if *f > 0.0 { "inf" } else { "-inf" }.into(),
        Value::Float(f) => format!("{f:?}"),
        Value::Boolean(b) => b.to_string(),
        Value::Datetime(d) => d.to_string(),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(render_value).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Table(t) if t.is_empty() => "{}".into(),
        Value::Table(t) => {
            let parts: Vec<String> = t
                .iter()
                .map(|(k, v)| format!("{} = {}", key_text(k), render_value(v)))
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
    }
}

/// The value at `path` of a parsed table.
fn value_mut<'t>(root: &'t mut Table, path: &[Seg]) -> Option<&'t mut Value> {
    let (first, rest) = path.split_first()?;
    let Seg::Key(n) = first else {
        return None;
    };
    let mut cur = root.get_mut(*n)?;
    for seg in rest {
        cur = match (seg, cur) {
            (Seg::Key(n), Value::Table(t)) => t.get_mut(*n)?,
            (Seg::At(i), Value::Array(a)) => a.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// The table at `path` of a parsed table (the empty path is the table).
fn table_mut<'t>(root: &'t mut Table, path: &[Seg]) -> Option<&'t mut Table> {
    if path.is_empty() {
        return Some(root);
    }
    match value_mut(root, path)? {
        Value::Table(t) => Some(t),
        _ => None,
    }
}

/// Applies the edits, reads the text again and compares it with the table that
/// the edit expects. One of `expected` must be the table that the new text
/// gives. If not, the result is not what the edit meant, and the person edits
/// the form by hand.
fn finish(
    text: &str,
    edits: Vec<Edit>,
    path: &[Seg],
    form: &str,
    expected: &[Table],
) -> Result<String, Fatal> {
    let out = apply(text, edits).ok_or_else(|| by_hand(path, form))?;
    match out.parse::<Table>() {
        Ok(t) if expected.contains(&t) => Ok(out),
        _ => Err(by_hand(path, form)),
    }
}

/// The table of `text`, to change into the expected table.
fn base(text: &str) -> Result<Table, Fatal> {
    text.parse::<Table>()
        .map_err(|e| Fatal::Config(format!("spec-dna.toml does not parse: {e}")))
}

/// Removes the empty tables that a deleted path leaves above it: a table that
/// only dotted keys or the headers of its sub-tables define goes with its last
/// key.
fn prune(root: &mut Table, parent: &[Seg]) {
    for n in (1..=parent.len()).rev() {
        let path = &parent[..n];
        let empty = matches!(value_mut(root, path), Some(Value::Table(t)) if t.is_empty());
        if !empty {
            return;
        }
        let (last, above) = path.split_last().expect("a path has a step");
        let Seg::Key(name) = last else {
            return;
        };
        if let Some(t) = table_mut(root, above) {
            t.remove(*name);
        }
    }
}

/// Renames the last key of `path` to `new`, at its span. The key is a name in a
/// header, a dotted key, an inline table or a line of keys.
pub fn rename_key(text: &str, path: &[Seg], new: &str) -> Result<String, Fatal> {
    let doc = parse_doc(text)?;
    let Some((Seg::Key(old), parent)) = path.split_last() else {
        return Err(missing(path));
    };
    let found = locate(doc.get_ref(), path).ok_or_else(|| missing(path))?;
    let span = found.key.clone().ok_or_else(|| missing(path))?;
    let mut expected = base(text)?;
    let table = table_mut(&mut expected, parent).ok_or_else(|| missing(path))?;
    if table.contains_key(new) {
        return Err(exists(path, new));
    }
    let value = table.remove(*old).ok_or_else(|| missing(path))?;
    table.insert(new.to_string(), value);
    let edit = Edit {
        start: span.start,
        end: span.end,
        text: key_like(&text[span.clone()], new),
    };
    // The name of a key in 2 places has one span: the check below sees it. A
    // table is the only value that can have 2 places. The person writes it
    // with a header and the name that it has now, because the 0.5 reader of
    // `upgrade` refuses a new name.
    let form = if matches!(found.value.get_ref(), DeValue::Table(_)) {
        format!(
            "{}, with each key of the table under that header and with the name `{old}` as it is now",
            form_header(path)
        )
    } else {
        format!("`{new}` in each place that has the name `{old}`")
    };
    finish(text, vec![edit], path, &form, &[expected])
}

/// Replaces the value at `path`. A scalar, an inline table and an array of one
/// line are written whole. An array of many lines gets one splice for each
/// element, so each comment stays: the new array has the same length, and no
/// element is a table or an array.
pub fn set_value(text: &str, path: &[Seg], new: &Value) -> Result<String, Fatal> {
    let doc = parse_doc(text)?;
    let found = locate(doc.get_ref(), path).ok_or_else(|| missing(path))?;
    let span = found.value.span();
    let old = &text[span.clone()];
    let inline = matches!(found.value.get_ref(), DeValue::Table(_)) && old.starts_with('{');
    if matches!(found.value.get_ref(), DeValue::Table(_)) && !inline {
        return Err(by_hand(path, FORM_INLINE));
    }
    if is_table_array(text, &found) {
        let form = format!("an array of tables, such as `[[{}]]`", show(path));
        return Err(by_hand(path, &form));
    }
    let edits = match (found.value.get_ref(), new) {
        (DeValue::Array(items), Value::Array(values)) if old.contains('\n') => {
            let simple = |v: &Value| !matches!(v, Value::Table(_) | Value::Array(_));
            let flat = items
                .iter()
                .all(|e| !matches!(e.get_ref(), DeValue::Table(_) | DeValue::Array(_)));
            if items.len() != values.len() || !values.iter().all(simple) || !flat {
                return Err(by_hand(path, FORM_ARRAY_LINE));
            }
            items
                .iter()
                .zip(values)
                .map(|(e, v)| Edit {
                    start: e.span().start,
                    end: e.span().end,
                    text: render_value(v),
                })
                .collect()
        }
        _ if old.contains('\n') => return Err(by_hand(path, FORM_LINE)),
        _ => vec![Edit {
            start: span.start,
            end: span.end,
            text: render_value(new),
        }],
    };
    let mut expected = base(text)?;
    *value_mut(&mut expected, path).ok_or_else(|| missing(path))? = new.clone();
    finish(text, edits, path, FORM_LINE, &[expected])
}

/// The text of a string with the quote style of its old text.
fn string_like(old: &str, new: &str) -> String {
    if old.starts_with('\'') && !new.contains('\'') && !new.chars().any(char::is_control) {
        format!("'{new}'")
    } else {
        quote(new)
    }
}

/// Replaces each string of the value at `path`, a string or an array of
/// strings, for which `f` gives a new text. A literal string stays literal if
/// it can. The text keeps each comment and each other byte.
pub fn replace_strings(
    text: &str,
    path: &[Seg],
    f: &dyn Fn(&str) -> Option<String>,
) -> Result<String, Fatal> {
    let doc = parse_doc(text)?;
    let found = locate(doc.get_ref(), path).ok_or_else(|| missing(path))?;
    let mut edits = Vec::new();
    let mut strings: Vec<(&toml::Spanned<DeValue>, usize)> = Vec::new();
    match found.value.get_ref() {
        DeValue::String(_) => strings.push((found.value, 0)),
        DeValue::Array(items) if !is_table_array(text, &found) => {
            for (i, e) in items.iter().enumerate() {
                if !matches!(e.get_ref(), DeValue::String(_)) {
                    return Err(by_hand(path, "an array of strings"));
                }
                strings.push((e, i));
            }
        }
        _ => return Err(by_hand(path, "a string, or an array of strings")),
    }
    let is_array = matches!(found.value.get_ref(), DeValue::Array(_));
    let mut expected = base(text)?;
    for (value, index) in strings {
        let DeValue::String(s) = value.get_ref() else {
            continue;
        };
        let span = value.span();
        let raw = &text[span.clone()];
        let Some(new) = f(s).filter(|n| n != s.as_ref()) else {
            continue;
        };
        if raw.starts_with("\"\"\"") || raw.starts_with("'''") {
            return Err(by_hand(path, "a string on one line"));
        }
        edits.push(Edit {
            start: span.start,
            end: span.end,
            text: string_like(raw, &new),
        });
        let slot = value_mut(&mut expected, path).ok_or_else(|| missing(path))?;
        match (is_array, slot) {
            (true, Value::Array(a)) => a[index] = Value::String(new),
            (false, slot) => *slot = Value::String(new),
            _ => return Err(missing(path)),
        }
    }
    finish(text, edits, path, FORM_LINE, &[expected])
}

/// Deletes the key at `path`: its lines, from the key to the end of its value
/// and of its comment. A comment line above the key stays. A key that shares
/// its line with another key, a table with a header and a name that many
/// lines write are forms for a hand edit. For a table, use `delete_table`.
pub fn delete_key(text: &str, path: &[Seg]) -> Result<String, Fatal> {
    let doc = parse_doc(text)?;
    let found = locate(doc.get_ref(), path).ok_or_else(|| missing(path))?;
    let key = found.key.clone().ok_or_else(|| missing(path))?;
    let value = found.value.span();
    let table = matches!(found.value.get_ref(), DeValue::Table(_));
    if is_table_array(text, &found) || is_header_table(text, &found) {
        return Err(Fatal::Config(format!(
            "`{}`: is a table with a header, not a key with a value; delete it with its header",
            show(path)
        )));
    }
    if table && !text[value.clone()].starts_with('{') {
        // A table that dotted keys write: the person writes it with a header.
        return Err(by_hand(path, &form_header(path)));
    }
    let start = line_start(text, key.start);
    let end = line_end(text, value.end);
    // The part before the key is a dotted prefix or white space, and the part
    // after the value is white space or a comment.
    let before = &text[start..key.start];
    let prefix = |c: char| c.is_whitespace() || c.is_alphanumeric() || "-_.\"'".contains(c);
    let after = text[value.end..end].trim();
    if !before.chars().all(prefix) || !(after.is_empty() || after.starts_with('#')) {
        return Err(by_hand(path, FORM_LINE));
    }
    let (last, parent) = path.split_last().expect("a path has a step");
    let Seg::Key(name) = last else {
        return Err(missing(path));
    };
    let mut kept = base(text)?;
    table_mut(&mut kept, parent)
        .ok_or_else(|| missing(path))?
        .remove(*name);
    let mut pruned = kept.clone();
    prune(&mut pruned, parent);
    let edit = Edit {
        start,
        end,
        text: String::new(),
    };
    finish(text, vec![edit], path, FORM_LINE, &[kept, pruned])
}

/// A header of the file: the path of its table and its span.
struct Header {
    path: Vec<OwnedSeg>,
    span: std::ops::Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum OwnedSeg {
    Key(String),
    At(usize),
}

impl OwnedSeg {
    fn is(&self, seg: &Seg) -> bool {
        match (self, seg) {
            (OwnedSeg::Key(a), Seg::Key(b)) => a == b,
            (OwnedSeg::At(a), Seg::At(b)) => a == b,
            _ => false,
        }
    }
}

fn collect_headers(text: &str, table: &DeTable, prefix: &mut Vec<OwnedSeg>, out: &mut Vec<Header>) {
    for (key, value) in table.iter() {
        prefix.push(OwnedSeg::Key(key.get_ref().to_string()));
        match value.get_ref() {
            DeValue::Table(inner) => {
                if text[value.span()].starts_with('[') {
                    out.push(Header {
                        path: prefix.clone(),
                        span: value.span(),
                    });
                }
                collect_headers(text, inner, prefix, out);
            }
            DeValue::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    if let DeValue::Table(inner) = item.get_ref() {
                        prefix.push(OwnedSeg::At(i));
                        if text[item.span()].starts_with("[[") {
                            out.push(Header {
                                path: prefix.clone(),
                                span: item.span(),
                            });
                        }
                        collect_headers(text, inner, prefix, out);
                        prefix.pop();
                    }
                }
            }
            _ => {}
        }
        prefix.pop();
    }
}

/// Deletes the table at `path` with its sub-tables: from its header to the
/// line before the next header of another table. The comment lines right above
/// that next header stay, because they describe it. A comment above the
/// deleted header stays too. For the last table of a file, the empty lines
/// before it go. The table must be written with headers in one range.
pub fn delete_table(text: &str, path: &[Seg]) -> Result<String, Fatal> {
    let doc = parse_doc(text)?;
    let mut headers = Vec::new();
    collect_headers(text, doc.get_ref(), &mut Vec::new(), &mut headers);
    headers.sort_by_key(|h| h.span.start);
    let inside: Vec<usize> = headers
        .iter()
        .enumerate()
        .filter(|(_, h)| {
            h.path.len() >= path.len() && h.path.iter().zip(path).all(|(a, b)| a.is(b))
        })
        .map(|(i, _)| i)
        .collect();
    if locate(doc.get_ref(), path).is_none() {
        return Err(missing(path));
    }
    let (Some(&first), Some(&last)) = (inside.first(), inside.last()) else {
        return Err(by_hand(path, &form_header(path)));
    };
    let range_form = format!(
        "the headers of `{}` together, with no other header between them",
        show(path)
    );
    if last - first + 1 != inside.len() {
        return Err(by_hand(path, &range_form));
    }
    let head = headers[first].span.start;
    let mut start = line_start(text, head);
    if !text[start..head].trim().is_empty() {
        return Err(by_hand(path, &range_form));
    }
    let mut end;
    match headers.get(last + 1) {
        Some(next) => {
            end = line_start(text, next.span.start);
            while end > start {
                let previous = line_start(text, end - 1);
                if text[previous..end].trim_start().starts_with('#') {
                    end = previous;
                } else {
                    break;
                }
            }
        }
        None => {
            end = text.len();
            while start > 0 {
                let previous = line_start(text, start - 1);
                if text[previous..start].trim().is_empty() {
                    start = previous;
                } else {
                    break;
                }
            }
        }
    }
    let (last_seg, parent) = path.split_last().ok_or_else(|| missing(path))?;
    let mut kept = base(text)?;
    match last_seg {
        Seg::Key(name) => {
            table_mut(&mut kept, parent)
                .ok_or_else(|| missing(path))?
                .remove(*name);
        }
        Seg::At(i) => match value_mut(&mut kept, parent) {
            Some(Value::Array(a)) if *i < a.len() => {
                a.remove(*i);
            }
            _ => return Err(missing(path)),
        },
    }
    let mut pruned = kept.clone();
    prune(&mut pruned, parent);
    let edit = Edit {
        start,
        end,
        text: String::new(),
    };
    finish(text, vec![edit], path, &range_form, &[kept, pruned])
}

/// The end of the last key that a table has on lines of its own, and no
/// sub-table with a header has. A dotted key counts.
fn last_key_end(text: &str, table: &DeTable) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (key, value) in table.iter() {
        if in_header_line(text, key.span().start) {
            continue;
        }
        let end = match value.get_ref() {
            DeValue::Table(inner) if !text[value.span()].starts_with('{') => {
                last_key_end(text, inner)
            }
            _ => Some(value.span().end),
        };
        best = best.max(end);
    }
    best
}

/// Inserts keys into the table at `path`; the empty path is the top level. The
/// new lines come after the last key of the table, or after its header, and
/// before its first sub-table header. They end as the lines of the file end.
/// The table is the top level or a table with a header, also an entry of an
/// array of tables.
pub fn insert_keys(text: &str, path: &[Seg], entries: &[(&str, Value)]) -> Result<String, Fatal> {
    let doc = parse_doc(text)?;
    let eol = line_ending(text);
    let (last, after_header, first_header) = if path.is_empty() {
        let mut headers = Vec::new();
        collect_headers(text, doc.get_ref(), &mut Vec::new(), &mut headers);
        let first = headers.iter().map(|h| h.span.start).min();
        (last_key_end(text, doc.get_ref()), None, first)
    } else {
        let found = locate(doc.get_ref(), path).ok_or_else(|| missing(path))?;
        let DeValue::Table(inner) = found.value.get_ref() else {
            return Err(by_hand(path, &form_header(path)));
        };
        if !text[found.value.span()].starts_with('[') {
            return Err(by_hand(path, &form_header(path)));
        }
        (
            last_key_end(text, inner),
            Some(found.value.span().end),
            None,
        )
    };
    let mut expected = base(text)?;
    let table = table_mut(&mut expected, path).ok_or_else(|| missing(path))?;
    for (name, value) in entries {
        if table.contains_key(*name) {
            return Err(exists(path, name));
        }
        table.insert((*name).to_string(), value.clone());
    }
    let mut lines = String::new();
    for (name, value) in entries {
        lines.push_str(&format!(
            "{} = {}{eol}",
            key_text(name),
            render_value(value)
        ));
    }
    let edit = match (last, after_header, first_header) {
        (Some(end), _, _) | (None, Some(end), _) => {
            let at = line_end(text, end);
            let lead = if text[..at].ends_with('\n') { "" } else { eol };
            Edit::insert(at, format!("{lead}{lines}"))
        }
        (None, None, Some(header)) => {
            // Before the first header and the comment lines right above it.
            let mut at = line_start(text, header);
            while at > 0 {
                let previous = line_start(text, at - 1);
                if text[previous..at].trim_start().starts_with('#') {
                    at = previous;
                } else {
                    break;
                }
            }
            Edit::insert(at, format!("{lines}{eol}"))
        }
        (None, None, None) => {
            let lead = if text.is_empty() || text.ends_with('\n') {
                ""
            } else {
                eol
            };
            Edit::insert(text.len(), format!("{lead}{lines}"))
        }
    };
    finish(text, vec![edit], path, &form_header(path), &[expected])
}

// --- end of edits ---

#[cfg(test)]
mod tests {
    use super::*;

    /// The message of a refusal: it is `Fatal::Config` (exit code 3).
    fn refusal(r: Result<String, Fatal>) -> String {
        match r {
            Err(Fatal::Config(m)) => m,
            Err(e) => panic!("not a configuration error: {e:?}"),
            Ok(t) => panic!("no refusal: {t:?}"),
        }
    }

    /// The message of D138: the key, the form and the next step.
    fn assert_by_hand(m: &str, key: &str, form: &str) {
        assert_eq!(
            m,
            format!(
                "`{key}`: upgrade cannot edit this form; write it as {form} by hand, commit the change, then run `spec-dna upgrade` again"
            )
        );
    }

    fn parses_as(text: &str) -> Table {
        text.parse().unwrap_or_else(|e| panic!("{e}: {text:?}"))
    }

    const SAMPLE: &str = "spec-dna = \"0.5.0\" # the pin\n\
exclude = [\"docs/vendor/**\"]\n\
\n\
# The fixture shows the lint.\n\
[rules]\n\
lint = \"warning\"\n\
\n\
[[roots]]\n\
path = \"docs\"\n\
type = \"mixed\" # one root\n\
default-type = \"spec\"\n\
\n\
# A root of one type.\n\
[[roots]]\n\
path = \"docs/field-notes\"\n\
type = \"note\"\n\
required-sections = { runbook = [\"Restart\"] }\n\
\n\
[roots.status-map]\n\
done = \"active\" # end\n\
\n\
# The budget of the notes.\n\
[budgets.note]  # soft and hard\n\
soft = 300\n\
hard = 600\n\
\n\
[budgets.guide]\n\
soft = 3000\n\
\n\
[context]\n\
budget = 2500\n";

    // --- rename_key ---

    #[test]
    fn a_rename_of_a_header_key_keeps_each_comment_byte_for_byte() {
        let out = rename_key(SAMPLE, &[k("budgets"), k("note")], "research").unwrap();
        assert_eq!(out, SAMPLE.replace("[budgets.note]", "[budgets.research]"));
        assert!(out.contains("# The budget of the notes.\n[budgets.research]  # soft and hard\n"));
        let t = parses_as(&out);
        assert!(t["budgets"].as_table().unwrap().contains_key("research"));
        assert!(!t["budgets"].as_table().unwrap().contains_key("note"));
    }

    #[test]
    fn a_rename_keeps_the_quote_style_of_the_key() {
        let t = "[\"a\"]\nx = 1\n['b']\nx = 1\n[c]\nx = 1\n";
        assert_eq!(
            rename_key(t, &[k("a")], "n-1").unwrap(),
            "[\"n-1\"]\nx = 1\n['b']\nx = 1\n[c]\nx = 1\n"
        );
        assert_eq!(
            rename_key(t, &[k("b")], "n-2").unwrap(),
            "[\"a\"]\nx = 1\n['n-2']\nx = 1\n[c]\nx = 1\n"
        );
        assert_eq!(
            rename_key(t, &[k("c")], "n-3").unwrap(),
            "[\"a\"]\nx = 1\n['b']\nx = 1\n[n-3]\nx = 1\n"
        );
        // A name that a bare key cannot hold gets quotes.
        assert_eq!(
            rename_key(t, &[k("c")], "n 3").unwrap(),
            "[\"a\"]\nx = 1\n['b']\nx = 1\n[\"n 3\"]\nx = 1\n"
        );
    }

    #[test]
    fn a_rename_reaches_a_key_in_an_entry_an_inline_table_and_a_dotted_key() {
        // A key of the second `[[roots]]`, and a key of its inline table.
        let out = rename_key(SAMPLE, &[k("roots"), at(1), k("type")], "kind").unwrap();
        assert_eq!(out, SAMPLE.replace("type = \"note\"", "kind = \"note\""));
        let out = rename_key(
            SAMPLE,
            &[k("roots"), at(1), k("required-sections"), k("runbook")],
            "procedure",
        )
        .unwrap();
        assert_eq!(out, SAMPLE.replace("{ runbook =", "{ procedure ="));
        // A key of the sub-table of an entry.
        let out = rename_key(
            SAMPLE,
            &[k("roots"), at(1), k("status-map"), k("done")],
            "closed",
        )
        .unwrap();
        assert_eq!(out, SAMPLE.replace("done = ", "closed = "));
        // The last key of a dotted key, and its first key once.
        let d = "b.guide.soft = 1 # c\nb.guide.hard = 2\n";
        assert_eq!(
            rename_key(d, &[k("b"), k("guide"), k("soft")], "min").unwrap(),
            "b.guide.min = 1 # c\nb.guide.hard = 2\n"
        );
        let one = "b.guide.soft = 1 # c\nz = 1\n";
        assert_eq!(
            rename_key(one, &[k("b"), k("guide")], "g").unwrap(),
            "b.g.soft = 1 # c\nz = 1\n"
        );
    }

    #[test]
    fn a_rename_keeps_crlf_and_a_missing_final_newline() {
        let t = "# c\r\n[budgets.note] # k\r\nsoft = 1\r\n";
        assert_eq!(
            rename_key(t, &[k("budgets"), k("note")], "research").unwrap(),
            "# c\r\n[budgets.research] # k\r\nsoft = 1\r\n"
        );
        let t = "[a]\nx = 1";
        assert_eq!(rename_key(t, &[k("a")], "b").unwrap(), "[b]\nx = 1");
    }

    #[test]
    fn a_rename_that_cannot_hold_stops_with_the_message_of_d138() {
        // The name of a key that 2 lines write: one span, 2 places.
        let d = "b.guide.soft = 1\nb.guide.hard = 2\n";
        let m = refusal(rename_key(d, &[k("b"), k("guide")], "g"));
        assert_by_hand(
            &m,
            "b.guide",
            "a table with a header, such as `[b.guide]`, with each key of the table under that header and with the name `guide` as it is now",
        );
        // The same for 2 headers with one prefix.
        let h = "[a.b]\nx = 1\n[a.c]\ny = 1\n";
        let m = refusal(rename_key(h, &[k("a")], "z"));
        assert_by_hand(
            &m,
            "a",
            "a table with a header, such as `[a]`, with each key of the table under that header and with the name `a` as it is now",
        );
        // The new name exists.
        let m = refusal(rename_key(SAMPLE, &[k("budgets"), k("note")], "guide"));
        assert!(
            m.contains("`budgets.note`") && m.contains("`guide` exists already"),
            "{m}"
        );
        // No such key.
        let m = refusal(rename_key(SAMPLE, &[k("budgets"), k("none")], "x"));
        assert!(
            m.contains("`budgets.none`") && m.contains("no such key"),
            "{m}"
        );
        // A text that does not parse.
        let m = refusal(rename_key("a = \n", &[k("a")], "b"));
        assert!(m.contains("does not parse"), "{m}");
    }

    // --- set_value ---

    #[test]
    fn set_value_replaces_a_scalar_and_keeps_its_comment() {
        let out = set_value(SAMPLE, &[k("spec-dna")], &Value::String("0.6.0".into())).unwrap();
        assert_eq!(
            out,
            SAMPLE.replace("\"0.5.0\" # the pin", "\"0.6.0\" # the pin")
        );
        let out = set_value(
            SAMPLE,
            &[k("budgets"), k("guide"), k("soft")],
            &Value::Integer(12),
        )
        .unwrap();
        assert_eq!(out, SAMPLE.replace("soft = 3000", "soft = 12"));
        let out = set_value(
            SAMPLE,
            &[k("roots"), at(0), k("type")],
            &Value::String("a\"b".into()),
        )
        .unwrap();
        assert_eq!(
            out,
            SAMPLE.replace("\"mixed\" # one root", "\"a\\\"b\" # one root")
        );
    }

    #[test]
    fn set_value_writes_an_inline_table_and_a_one_line_array_whole() {
        let mut t = Table::new();
        t.insert(
            "procedure".into(),
            Value::Array(vec![Value::String("Restart".into())]),
        );
        t.insert("n".into(), Value::Integer(2));
        let out = set_value(
            SAMPLE,
            &[k("roots"), at(1), k("required-sections")],
            &Value::Table(t),
        )
        .unwrap();
        assert_eq!(
            out,
            SAMPLE.replace(
                "{ runbook = [\"Restart\"] }",
                "{ n = 2, procedure = [\"Restart\"] }"
            )
        );
        let out = set_value(
            SAMPLE,
            &[k("exclude")],
            &Value::Array(vec![Value::String("a".into()), Value::String("b".into())]),
        )
        .unwrap();
        assert_eq!(
            out,
            SAMPLE.replace("[\"docs/vendor/**\"]", "[\"a\", \"b\"]")
        );
        // An empty array and an empty table.
        let out = set_value(SAMPLE, &[k("exclude")], &Value::Array(vec![])).unwrap();
        assert_eq!(out, SAMPLE.replace("[\"docs/vendor/**\"]", "[]"));
        let out = set_value(
            SAMPLE,
            &[k("roots"), at(1), k("required-sections")],
            &Value::Table(Table::new()),
        )
        .unwrap();
        assert_eq!(out, SAMPLE.replace("{ runbook = [\"Restart\"] }", "{}"));
    }

    #[test]
    fn set_value_splices_each_element_of_a_multi_line_array() {
        let t = "a = [\n  \"x\", # first\n  \"y\",\n  # last\n  \"z\"\n]\n";
        let new = Value::Array(vec![
            Value::String("x".into()),
            Value::String("Y".into()),
            Value::String("Z".into()),
        ]);
        assert_eq!(
            set_value(t, &[k("a")], &new).unwrap(),
            "a = [\n  \"x\", # first\n  \"Y\",\n  # last\n  \"Z\"\n]\n"
        );
        // Another length: no one splice per element.
        let short = Value::Array(vec![Value::String("x".into())]);
        let m = refusal(set_value(t, &[k("a")], &short));
        assert_by_hand(&m, "a", FORM_ARRAY_LINE);
        // An element that is a table: no splice.
        let tables = Value::Array(vec![
            Value::Table(Table::new()),
            Value::Table(Table::new()),
            Value::Table(Table::new()),
        ]);
        assert_by_hand(
            &refusal(set_value(t, &[k("a")], &tables)),
            "a",
            FORM_ARRAY_LINE,
        );
    }

    #[test]
    fn set_value_stops_on_a_table_with_a_header_and_on_a_dotted_key() {
        let m = refusal(set_value(
            SAMPLE,
            &[k("budgets"), k("note")],
            &Value::Table(Table::new()),
        ));
        assert_by_hand(
            &m,
            "budgets.note",
            "an inline table, such as `key = { a = 1 }`",
        );
        let m = refusal(set_value(
            SAMPLE,
            &[k("budgets")],
            &Value::Table(Table::new()),
        ));
        assert_by_hand(&m, "budgets", "an inline table, such as `key = { a = 1 }`");
        let m = refusal(set_value(SAMPLE, &[k("roots")], &Value::Array(vec![])));
        assert_by_hand(&m, "roots", "an array of tables, such as `[[roots]]`");
        let m = refusal(set_value(SAMPLE, &[k("nope")], &Value::Integer(1)));
        assert!(m.contains("no such key"), "{m}");
    }

    // --- replace_strings ---

    #[test]
    fn replace_strings_renames_a_string_and_each_string_of_an_array() {
        let map = |s: &str| match s {
            "note" => Some("research".to_string()),
            "guide" | "runbook" => Some("procedure".to_string()),
            _ => None,
        };
        let out = replace_strings(SAMPLE, &[k("roots"), at(1), k("type")], &map).unwrap();
        assert_eq!(
            out,
            SAMPLE.replace("type = \"note\"", "type = \"research\"")
        );
        // An array, one line, with a literal string. Each quote stays.
        let t = "types = [\"guide\", 'runbook', \"keep\", \"note\"]\n";
        assert_eq!(
            replace_strings(t, &[k("types")], &map).unwrap(),
            "types = [\"procedure\", 'procedure', \"keep\", \"research\"]\n"
        );
        // An array of many lines: each comment stays.
        let t = "types = [\n  \"guide\", # g\n  # between\n  \"keep\",\n  \"note\" # n\n]\n";
        assert_eq!(
            replace_strings(t, &[k("types")], &map).unwrap(),
            "types = [\n  \"procedure\", # g\n  # between\n  \"keep\",\n  \"research\" # n\n]\n"
        );
        // A value in an inline table.
        let t = "status-map = { done = \"note\", other = \"x\" } # c\n";
        assert_eq!(
            replace_strings(t, &[k("status-map"), k("done")], &map).unwrap(),
            "status-map = { done = \"research\", other = \"x\" } # c\n"
        );
        // No string changes: the text is the same.
        assert_eq!(
            replace_strings(SAMPLE, &[k("roots"), at(0), k("type")], &map).unwrap(),
            SAMPLE
        );
        // A name that a literal string cannot hold gets a basic string.
        let q = |_: &str| Some("it's".to_string());
        assert_eq!(
            replace_strings("a = 'x'\n", &[k("a")], &q).unwrap(),
            "a = \"it's\"\n"
        );
    }

    #[test]
    fn replace_strings_stops_on_a_value_that_is_not_a_string() {
        let map = |_: &str| Some("x".to_string());
        let m = refusal(replace_strings(
            SAMPLE,
            &[k("budgets"), k("note"), k("soft")],
            &map,
        ));
        assert_by_hand(&m, "budgets.note.soft", "a string, or an array of strings");
        let m = refusal(replace_strings("a = \"\"\"y\"\"\"\n", &[k("a")], &map));
        assert_by_hand(&m, "a", "a string on one line");
        let m = refusal(replace_strings("a = [\"x\", 1]\n", &[k("a")], &map));
        assert_by_hand(&m, "a", "an array of strings");
    }

    // --- delete_key ---

    #[test]
    fn delete_key_removes_the_lines_of_the_key_and_its_comment() {
        let out = delete_key(SAMPLE, &[k("roots"), at(0), k("default-type")]).unwrap();
        assert_eq!(out, SAMPLE.replace("default-type = \"spec\"\n", ""));
        // A trailing comment goes with the key. A comment line above stays.
        let t = "# above\nx = 1 # tail\ny = 2\n";
        assert_eq!(delete_key(t, &[k("x")]).unwrap(), "# above\ny = 2\n");
        // A value of many lines.
        let t = "x = [\n  1, # a\n  2,\n] # tail\ny = 2\n";
        assert_eq!(delete_key(t, &[k("x")]).unwrap(), "y = 2\n");
        // An inline table, the last line of a file with no final newline, CRLF.
        let t = "y = 1\nx = { a = 1 }";
        assert_eq!(delete_key(t, &[k("x")]).unwrap(), "y = 1\n");
        let t = "y = 1\r\nx = 2 # c\r\nz = 3\r\n";
        assert_eq!(delete_key(t, &[k("x")]).unwrap(), "y = 1\r\nz = 3\r\n");
        // A last dotted key.
        let t = "b.guide.soft = 1\nb.guide.hard = 2\n";
        assert_eq!(
            delete_key(t, &[k("b"), k("guide"), k("soft")]).unwrap(),
            "b.guide.hard = 2\n"
        );
        let t = "b.guide.soft = 1\nz = 1\n";
        assert_eq!(
            delete_key(t, &[k("b"), k("guide"), k("soft")]).unwrap(),
            "z = 1\n"
        );
    }

    #[test]
    fn delete_key_stops_on_a_form_with_no_lines_of_its_own() {
        // A key that shares its line with another key.
        let t = "x = { a = 1, b = 2 }\n";
        assert_by_hand(
            &refusal(delete_key(t, &[k("x"), k("a")])),
            "x.a",
            "`key = value` on a line of its own",
        );
        // A table with a header: it goes with `delete_table`.
        assert_eq!(
            refusal(delete_key(SAMPLE, &[k("budgets"), k("note")])),
            "`budgets.note`: is a table with a header, not a key with a value; delete it with its header"
        );
        // A name that many dotted lines write.
        let t = "b.guide.soft = 1\nb.guide.hard = 2\n";
        assert_by_hand(
            &refusal(delete_key(t, &[k("b"), k("guide")])),
            "b.guide",
            "a table with a header, such as `[b.guide]`",
        );
        let m = refusal(delete_key(SAMPLE, &[k("nope")]));
        assert!(m.contains("no such key"), "{m}");
    }

    // --- delete_table ---

    #[test]
    fn delete_table_removes_the_header_to_the_next_header() {
        let out = delete_table(SAMPLE, &[k("budgets"), k("note")]).unwrap();
        // The comment above the next header and the one above the deleted
        // header stay: the first is in the removed text only if it is above
        // the next header.
        let want = SAMPLE.replace(
            "# The budget of the notes.\n[budgets.note]  # soft and hard\nsoft = 300\nhard = 600\n\n[budgets.guide]\n",
            "# The budget of the notes.\n[budgets.guide]\n",
        );
        assert_eq!(out, want);
        assert!(
            !parses_as(&out)["budgets"]
                .as_table()
                .unwrap()
                .contains_key("note")
        );
    }

    #[test]
    fn delete_table_keeps_the_comment_of_the_next_header() {
        let t = "[a]\nx = 1\n\n# about b\n[b]\ny = 1\n";
        assert_eq!(
            delete_table(t, &[k("a")]).unwrap(),
            "# about b\n[b]\ny = 1\n"
        );
        // The last table: the blank line before it goes too.
        let t = "z = 1\n\n[a]\nx = 1\n\n[b]\ny = 1\n";
        assert_eq!(delete_table(t, &[k("b")]).unwrap(), "z = 1\n\n[a]\nx = 1\n");
        let t = "[a]\nx = 1\n";
        assert_eq!(delete_table(t, &[k("a")]).unwrap(), "");
        // A header with no final newline.
        let t = "z = 1\n[a]\nx = 1";
        assert_eq!(delete_table(t, &[k("a")]).unwrap(), "z = 1\n");
    }

    #[test]
    fn delete_table_removes_the_sub_tables_and_an_entry_of_an_array() {
        let t = "[a]\nx = 1\n[a.b]\ny = 1\n[a.b.c]\nz = 1\n[d]\nw = 1\n";
        assert_eq!(delete_table(t, &[k("a")]).unwrap(), "[d]\nw = 1\n");
        // A table that only its sub-table headers define.
        let t = "[a.b.c]\nz = 1\n[d]\nw = 1\n";
        assert_eq!(delete_table(t, &[k("a"), k("b")]).unwrap(), "[d]\nw = 1\n");
        // The entry 1 of 2, with its sub-table.
        let out = delete_table(SAMPLE, &[k("roots"), at(1)]).unwrap();
        let want = SAMPLE.replace(
            "# A root of one type.\n[[roots]]\npath = \"docs/field-notes\"\ntype = \"note\"\nrequired-sections = { runbook = [\"Restart\"] }\n\n[roots.status-map]\ndone = \"active\" # end\n\n",
            "# A root of one type.\n",
        );
        // The comment above the deleted header stays, and so the comment of the
        // next header joins it.
        assert_eq!(out, want);
        assert_eq!(parses_as(&out)["roots"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn delete_table_stops_on_a_table_that_has_no_header_range() {
        // A table that is not in one range of headers.
        let t = "[a]\nx = 1\n[b]\ny = 1\n[a.c]\nz = 1\n";
        assert_by_hand(
            &refusal(delete_table(t, &[k("a")])),
            "a",
            "the headers of `a` together, with no other header between them",
        );
        // A table with a dotted key or an inline table.
        let header = "a table with a header, such as `[a]`";
        let t = "a.b = 1\n[c]\nx = 1\n";
        assert_by_hand(&refusal(delete_table(t, &[k("a")])), "a", header);
        let t = "a = { b = 1 }\n";
        assert_by_hand(&refusal(delete_table(t, &[k("a")])), "a", header);
        // An entry of an array of tables names its header.
        let t = "[[r]]\nx = 1\n[[r]]\ny = 1\n[a]\n[r.s]\n";
        let m = refusal(delete_table(t, &[k("r"), at(1)]));
        assert_by_hand(
            &m,
            "r[1]",
            "the headers of `r[1]` together, with no other header between them",
        );
        let m = refusal(delete_table(SAMPLE, &[k("nope")]));
        assert!(m.contains("no such key"), "{m}");
    }

    // --- insert_keys ---

    fn entries() -> Vec<(&'static str, Value)> {
        vec![
            ("live-sections", Value::Table(Table::new())),
            ("skills", Value::Array(vec![])),
            ("review-md", Value::Boolean(false)),
            ("n", Value::Integer(1)),
        ]
    }

    #[test]
    fn insert_keys_goes_after_the_last_key_and_before_the_first_sub_table() {
        let out = insert_keys(SAMPLE, &[k("roots"), at(1)], &entries()).unwrap();
        let block = "live-sections = {}\nskills = []\nreview-md = false\nn = 1\n";
        let want = SAMPLE.replace(
            "required-sections = { runbook = [\"Restart\"] }\n",
            &format!("required-sections = {{ runbook = [\"Restart\"] }}\n{block}"),
        );
        assert_eq!(out, want);
        // The entry 0 has a comment on its last line, and is followed by a blank line.
        let out = insert_keys(SAMPLE, &[k("roots"), at(0)], &entries()).unwrap();
        let want = SAMPLE.replace(
            "default-type = \"spec\"\n",
            &format!("default-type = \"spec\"\n{block}"),
        );
        assert_eq!(out, want);
        let out = insert_keys(SAMPLE, &[k("roots"), at(0)], &[("n", Value::Integer(1))]).unwrap();
        assert_eq!(
            out,
            SAMPLE.replace(
                "default-type = \"spec\"\n",
                "default-type = \"spec\"\nn = 1\n"
            )
        );
    }

    #[test]
    fn insert_keys_after_a_value_with_a_comment_or_many_lines() {
        let t = "[a]\nx = [\n  1,\n] # tail\n\n[b]\ny = 1\n";
        assert_eq!(
            insert_keys(t, &[k("a")], &[("z", Value::Integer(2))]).unwrap(),
            "[a]\nx = [\n  1,\n] # tail\nz = 2\n\n[b]\ny = 1\n"
        );
        // A table with no key: after its header line, before its sub-table.
        let t = "[a] # h\n[a.b]\ny = 1\n";
        assert_eq!(
            insert_keys(t, &[k("a")], &[("z", Value::Integer(2))]).unwrap(),
            "[a] # h\nz = 2\n[a.b]\ny = 1\n"
        );
        // The last line has no final newline, and the file is CRLF.
        let t = "[a]\nx = 1";
        assert_eq!(
            insert_keys(t, &[k("a")], &[("z", Value::Integer(2))]).unwrap(),
            "[a]\nx = 1\nz = 2\n"
        );
        let t = "[a]\r\nx = 1\r\n[b]\r\n";
        assert_eq!(
            insert_keys(t, &[k("a")], &[("z", Value::Integer(2))]).unwrap(),
            "[a]\r\nx = 1\r\nz = 2\r\n[b]\r\n"
        );
        // A dotted key counts as a key of the table.
        let t = "[a]\nb.c = 1\n[d]\n";
        assert_eq!(
            insert_keys(t, &[k("a")], &[("z", Value::Integer(2))]).unwrap(),
            "[a]\nb.c = 1\nz = 2\n[d]\n"
        );
    }

    #[test]
    fn insert_keys_into_the_top_level() {
        // After the last key of the top level.
        let out = insert_keys(SAMPLE, &[], &[("language", Value::String("en".into()))]).unwrap();
        assert_eq!(
            out,
            SAMPLE.replace(
                "exclude = [\"docs/vendor/**\"]\n",
                "exclude = [\"docs/vendor/**\"]\nlanguage = \"en\"\n"
            )
        );
        // No key: before the first header and its comment, with an empty line.
        let t = "# the file\n\n# about a\n[a]\nx = 1\n";
        assert_eq!(
            insert_keys(t, &[], &[("v", Value::Integer(1))]).unwrap(),
            "# the file\n\nv = 1\n\n# about a\n[a]\nx = 1\n"
        );
        // A file with no key and no header.
        assert_eq!(
            insert_keys("# only\n", &[], &[("v", Value::Integer(1))]).unwrap(),
            "# only\nv = 1\n"
        );
        assert_eq!(
            insert_keys("", &[], &[("v", Value::Integer(1))]).unwrap(),
            "v = 1\n"
        );
    }

    #[test]
    fn insert_keys_writes_each_form_of_a_value() {
        let mut inner = Table::new();
        inner.insert("claude-code".into(), Value::String("none".into()));
        let e = [
            ("a", Value::Table(inner)),
            (
                "b",
                Value::Array(vec![Value::Integer(1), Value::String("x\"y".into())]),
            ),
            ("c", Value::Float(1.0)),
            ("d", Value::Boolean(true)),
            ("a b", Value::String("q".into())),
        ];
        let out = insert_keys("[t]\n", &[k("t")], &e).unwrap();
        assert_eq!(
            out,
            "[t]\na = { claude-code = \"none\" }\nb = [1, \"x\\\"y\"]\nc = 1.0\nd = true\n\"a b\" = \"q\"\n"
        );
    }

    #[test]
    fn insert_keys_stops_on_a_key_that_exists_and_on_a_table_with_no_header() {
        let m = refusal(insert_keys(
            SAMPLE,
            &[k("roots"), at(0)],
            &[("path", Value::Integer(1))],
        ));
        assert!(
            m.contains("`roots[0]`") && m.contains("`path` exists already"),
            "{m}"
        );
        let t = "a = { b = 1 }\n";
        let header = "a table with a header, such as `[a]`";
        let new = [("c", Value::Integer(1))];
        assert_by_hand(&refusal(insert_keys(t, &[k("a")], &new)), "a", header);
        let t = "a.b = 1\n";
        assert_by_hand(&refusal(insert_keys(t, &[k("a")], &new)), "a", header);
        let t = "[[r]]\nx = 1\n";
        let m = refusal(insert_keys(t, &[k("r")], &new));
        assert_by_hand(&m, "r", "a table with a header, such as `[r]`");
        let m = refusal(insert_keys(
            SAMPLE,
            &[k("nope")],
            &[("c", Value::Integer(1))],
        ));
        assert!(m.contains("no such key"), "{m}");
    }

    // --- the moved helpers ---

    #[test]
    fn append_tables_adds_one_blank_line_and_the_line_ending_of_the_text() {
        let mut out = String::from("a = 1\n");
        append_tables(&mut out, "[b]\nx = 1\n", "\n");
        assert_eq!(out, "a = 1\n\n[b]\nx = 1\n");
        let mut out = String::from("a = 1\r\n\r\n");
        append_tables(&mut out, "[b]\nx = 1\n", "\r\n");
        assert_eq!(out, "a = 1\r\n\r\n[b]\r\nx = 1\r\n");
        let mut out = String::from("a = 1");
        append_tables(&mut out, "[b]\n", "\n");
        assert_eq!(out, "a = 1\n\n[b]\n");
        assert_eq!(line_ending("a\r\nb\n"), "\r\n");
        assert_eq!(line_ending("a\nb\r\n"), "\n");
        assert_eq!(quote("a\"\\\n"), "\"a\\\"\\\\\\n\"");
    }

    // --- the golden pairs ---

    /// The 0.5 name of a type or a status, and its name in 0.6: a table of the
    /// script of the golden files, not the table of `upgrade`.
    fn new_name(old: &str) -> Option<String> {
        let name = match old {
            "note" | "reference" => "research",
            "guide" | "runbook" => "procedure",
            "spec" => "requirements",
            "contract" => "interface",
            "invariant" => "rule",
            "change" => "intent",
            _ => return None,
        };
        Some(name.to_string())
    }

    /// A script of the edits that the pass 1 of `upgrade` needs, on the text of
    /// a 0.4 or 0.5 `spec-dna.toml`. It uses each function of the module. It
    /// does what the text allows: the golden files show the result.
    fn script(text: &str) -> Result<String, Fatal> {
        let mut text = set_value(text, &[k("spec-dna")], &Value::String("0.6.0".into()))?;
        let roots = base(&text)?
            .get("roots")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        for i in 0..roots {
            let root = base(&text)?["roots"][i].clone();
            for key in ["type", "default-type"] {
                if root.get(key).is_some() {
                    text = replace_strings(&text, &[k("roots"), at(i), k(key)], &new_name)?;
                }
            }
            if let Some(sections) = root.get("required-sections").and_then(Value::as_table) {
                for old in ["guide", "runbook"] {
                    if sections.contains_key(old) {
                        let path = [k("roots"), at(i), k("required-sections"), k(old)];
                        text = rename_key(&text, &path, "procedure")?;
                    }
                }
            }
            if root.get("status-map").is_some() {
                text = delete_key(&text, &[k("roots"), at(i), k("status-map")])?;
            }
            if root.get("live-sections").is_none() {
                let live = [("live-sections", Value::Table(Table::new()))];
                text = insert_keys(&text, &[k("roots"), at(i)], &live)?;
            }
        }
        let budgets = base(&text)?
            .get("budgets")
            .and_then(Value::as_table)
            .cloned();
        if let Some(budgets) = budgets {
            for (old, new) in [("note", "research"), ("guide", "procedure")] {
                if budgets.contains_key(old) {
                    text = rename_key(&text, &[k("budgets"), k(old)], new)?;
                }
            }
            if budgets.contains_key("runbook") {
                text = delete_table(&text, &[k("budgets"), k("runbook")])?;
            }
        }
        if base(&text)?.contains_key("harness") {
            let parts = [
                ("skills", Value::Array(vec![])),
                ("review-md", Value::Boolean(false)),
                ("protect-tests", Value::String("off".into())),
            ];
            text = insert_keys(&text, &[k("harness")], &parts)?;
        }
        let eol = line_ending(&text);
        append_tables(&mut text, "[change]\npolicy = \"none\"\n", eol);
        Ok(text)
    }

    /// The golden files of the script: the text before and after, for the
    /// `spec-dna.toml` of 4 fixtures. `UPDATE_GOLDEN=1` writes them again.
    #[test]
    fn the_script_gives_the_golden_files() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        let fixture = |name: &str| {
            std::fs::read_to_string(
                root.join("fixtures/upgrade")
                    .join(name)
                    .join("spec-dna.toml"),
            )
            .unwrap()
        };
        let append = std::fs::read_to_string(
            root.join("fixtures/upgrade/harness-full/spec-dna.toml.append"),
        )
        .unwrap();
        let cases = [
            ("v04", fixture("v04")),
            ("v05", fixture("v05")),
            ("brownfield", fixture("brownfield")),
            ("harness-full", fixture("v05") + &append),
        ];
        let dir = root.join("golden/upgrade/config/edits");
        let update = std::env::var_os("UPDATE_GOLDEN").is_some();
        for (name, before) in cases {
            let after = script(&before).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            // The new text is a configuration file.
            parses_as(&after);
            let (b, a) = (
                dir.join(format!("{name}.before.toml")),
                dir.join(format!("{name}.after.toml")),
            );
            if update {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(&b, &before).unwrap();
                std::fs::write(&a, &after).unwrap();
            }
            let want_before = std::fs::read_to_string(&b)
                .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_GOLDEN=1"));
            assert_eq!(
                before, want_before,
                "{name}: the before file is the fixture"
            );
            let want_after = std::fs::read_to_string(&a).unwrap();
            assert_eq!(
                after, want_after,
                "{name}: run with UPDATE_GOLDEN=1 to accept"
            );
        }
    }

    #[test]
    fn the_script_keeps_each_comment_of_the_fixtures() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade");
        for name in ["v04", "v05", "brownfield"] {
            let before = std::fs::read_to_string(root.join(name).join("spec-dna.toml")).unwrap();
            let after = script(&before).unwrap();
            for line in before.lines().filter(|l| l.trim_start().starts_with('#')) {
                assert!(after.contains(&format!("{line}\n")), "{name}: {line}");
            }
        }
    }

    #[test]
    fn a_header_with_spaces_and_a_nested_inline_table_rename_at_the_span() {
        let t = "[ budgets . note ]  # c\nsoft = 1\n";
        assert_eq!(
            rename_key(t, &[k("budgets"), k("note")], "research").unwrap(),
            "[ budgets . research ]  # c\nsoft = 1\n"
        );
        let t = "x = { a = { b = 1 }, c = 2 }\n";
        assert_eq!(
            rename_key(t, &[k("x"), k("a"), k("b")], "d").unwrap(),
            "x = { a = { d = 1 }, c = 2 }\n"
        );
    }

    #[test]
    fn the_script_gives_the_same_result_for_a_file_with_crlf() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade");
        for name in ["v04", "v05", "brownfield"] {
            let before = std::fs::read_to_string(root.join(name).join("spec-dna.toml")).unwrap();
            let lf = script(&before).unwrap();
            let crlf = script(&before.replace('\n', "\r\n")).unwrap();
            assert_eq!(crlf, lf.replace('\n', "\r\n"), "{name}");
        }
    }
}
