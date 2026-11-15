//! Front matter is a YAML subset that parses the same in YAML 1.1 and 1.2
//! (ADR 0001). Keys map to plain strings or lists of strings. Plain scalars
//! are strings. Anchors, aliases, tags, flow mappings, block scalars and
//! nested mappings are outside the subset. The parser keeps them as
//! `Unsupported`, so a key that spec-dna does not use can still contain them.

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    List(Vec<String>),
    Unsupported(&'static str),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrontMatter {
    /// Key, value and 1-based line of the key, in file order.
    pub fields: Vec<(String, Value, u32)>,
}

impl FrontMatter {
    pub fn get(&self, key: &str) -> Option<(&Value, u32)> {
        self.fields
            .iter()
            .find(|(k, _, _)| k == key)
            .map(|(_, v, l)| (v, *l))
    }
}

/// Splits a document into its front matter text and the byte offset where the
/// body starts. Front matter starts on the first line with `---`. It ends at
/// the next line that contains `---` or `...`. If no line ends it, the
/// document has no front matter.
pub fn split(text: &str) -> Option<(&str, usize)> {
    let rest = text.strip_prefix("---")?;
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))?;
    let start = text.len() - rest.len();
    let mut pos = start;
    for line in rest.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        if bare == "---" || bare == "..." {
            return Some((&text[start..pos], pos + line.len()));
        }
        pos += line.len();
    }
    None
}

/// Parses front matter text. `first_line` is the 1-based line of the first
/// front matter line in the file. An error contains a line number.
pub fn parse(src: &str, first_line: u32) -> Result<FrontMatter, (u32, String)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut fm = FrontMatter::default();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let lno = first_line + i as u32;
        if is_blank_or_comment(line) {
            i += 1;
            continue;
        }
        if line.starts_with([' ', '\t']) {
            return Err((lno, "unexpected indentation".into()));
        }
        let Some((key, rest)) = split_key(line) else {
            return Err((
                lno,
                format!("expected `key: value`, found `{}`", line.trim()),
            ));
        };
        if fm.fields.iter().any(|(k, _, _)| *k == key) {
            return Err((lno, format!("duplicate key `{key}`")));
        }
        let rest = strip_comment(rest).trim();
        i += 1;
        let value = if rest.is_empty() {
            // A block list, a nested mapping, or an empty value.
            let mut items = Vec::new();
            let mut nested = false;
            while i < lines.len() {
                let l = lines[i];
                if is_blank_or_comment(l) {
                    i += 1;
                    continue;
                }
                let t = l.trim_start();
                let indented = l.len() > t.len();
                if let Some(item) = t
                    .strip_prefix("- ")
                    .or(if t == "-" { Some("") } else { None })
                {
                    match scalar(strip_comment(item).trim()) {
                        Ok(Value::Str(s)) => items.push(s),
                        Ok(_) | Err(_) => nested = true,
                    }
                    if split_key(item.trim()).is_some() && !item.trim().starts_with(['"', '\'']) {
                        nested = true;
                    }
                    i += 1;
                } else if indented {
                    nested = true;
                    i += 1;
                } else {
                    break;
                }
            }
            if nested {
                Value::Unsupported("a nested mapping or a list of mappings")
            } else if items.is_empty() {
                Value::Str(String::new())
            } else {
                Value::List(items)
            }
        } else if rest.starts_with('|') || rest.starts_with('>') {
            while i < lines.len()
                && (lines[i].starts_with([' ', '\t']) || lines[i].trim().is_empty())
            {
                i += 1;
            }
            Value::Unsupported("a multi-line scalar")
        } else {
            match scalar(rest) {
                Ok(v) => v,
                Err(msg) => return Err((lno, msg)),
            }
        };
        fm.fields.push((key.to_string(), value, lno));
    }
    Ok(fm)
}

fn is_blank_or_comment(line: &str) -> bool {
    let t = line.trim();
    t.is_empty() || t.starts_with('#')
}

/// Splits `key: rest`, where the key is a plain or quoted scalar without `: `.
fn split_key(line: &str) -> Option<(&str, &str)> {
    let (key, rest) = if let Some(q) = line.strip_prefix('"') {
        let end = q.find('"')?;
        (&q[..end], q[end + 1..].strip_prefix(':')?)
    } else {
        let idx = line.find(':')?;
        let rest = &line[idx + 1..];
        if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
            return None;
        }
        (line[..idx].trim_end(), rest)
    };
    if key.is_empty() || key.contains(['#', '[', ']', '{', '}', ',']) {
        return None;
    }
    Some((key, rest))
}

/// Removes a ` #comment` outside quotes.
fn strip_comment(s: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut prev_space = true;
    for (i, ch) in s.char_indices() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => {}
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch == '#' && prev_space => return &s[..i],
            None => {}
        }
        prev_space = ch == ' ' || ch == '\t';
    }
    s
}

fn scalar(s: &str) -> Result<Value, String> {
    if s.is_empty() {
        return Ok(Value::Str(String::new()));
    }
    match s.as_bytes()[0] {
        b'[' => {
            let inner = s
                .strip_prefix('[')
                .and_then(|r| r.strip_suffix(']'))
                .ok_or("unclosed `[`")?;
            let mut items = Vec::new();
            for part in split_flow(inner)? {
                let part = part.trim();
                if part.is_empty() {
                    continue;
                }
                match scalar(part)? {
                    Value::Str(v) => items.push(v),
                    _ => return Ok(Value::Unsupported("a nested flow collection")),
                }
            }
            Ok(Value::List(items))
        }
        b'{' => Ok(Value::Unsupported("a flow mapping")),
        b'&' => Ok(Value::Unsupported("an anchor")),
        b'*' => Ok(Value::Unsupported("an alias")),
        b'!' => Ok(Value::Unsupported("a tag")),
        b'"' => {
            let inner = s
                .strip_prefix('"')
                .and_then(|r| r.strip_suffix('"'))
                .ok_or("unclosed double quote")?;
            let mut out = String::new();
            let mut chars = inner.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('t') => out.push('\t'),
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some('/') => out.push('/'),
                        Some(other) => return Err(format!("unsupported escape `\\{other}`")),
                        None => return Err("dangling backslash".into()),
                    }
                } else {
                    out.push(c);
                }
            }
            Ok(Value::Str(out))
        }
        b'\'' => {
            let inner = s
                .strip_prefix('\'')
                .and_then(|r| r.strip_suffix('\''))
                .ok_or("unclosed single quote")?;
            Ok(Value::Str(inner.replace("''", "'")))
        }
        _ => Ok(Value::Str(s.to_string())),
    }
}

fn split_flow(s: &str) -> Result<Vec<&str>, String> {
    let mut parts = Vec::new();
    let mut quote: Option<char> = None;
    let mut depth = 0i32;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => {}
            None => match ch {
                '"' | '\'' => quote = Some(ch),
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(&s[start..i]);
                    start = i + 1;
                }
                _ => {}
            },
        }
    }
    if quote.is_some() {
        return Err("unclosed quote in a flow list".into());
    }
    parts.push(&s[start..]);
    Ok(parts)
}

// The 3 edits of `upgrade` (design 0.6.0, D143): `set_field`, `insert_field`
// and `remove_field`. Each edits one line and keeps the order of the keys, the
// quotes, a trailing comment and the line endings.

/// A line of the front matter: its byte range in the text, and its text with
/// no line ending.
struct Line<'a> {
    start: usize,
    end: usize,
    bare: &'a str,
}

/// The lines of the front matter, and the offset where it starts. `None` if
/// the text has no front matter.
fn fm_lines(text: &str) -> Option<(usize, Vec<Line<'_>>)> {
    let (fm, _) = split(text)?;
    let rest = text.strip_prefix("---")?;
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))?;
    let start = text.len() - rest.len();
    let mut lines = Vec::new();
    let mut at = start;
    for line in fm.split_inclusive('\n') {
        lines.push(Line {
            start: at,
            end: at + line.len(),
            bare: line.trim_end_matches(['\n', '\r']),
        });
        at += line.len();
    }
    Some((start, lines))
}

/// The index of the line of a key. A line that starts with a space, a tab or
/// `-` belongs to the value of the line above.
fn key_line(lines: &[Line], key: &str) -> Option<usize> {
    lines.iter().position(|l| {
        !l.bare.starts_with([' ', '\t', '-', '#'])
            && split_key(l.bare).is_some_and(|(k, _)| k == key)
    })
}

/// The index after the last line of the value of the key line `i`.
fn value_end(lines: &[Line], i: usize) -> usize {
    let mut j = i + 1;
    while j < lines.len() && lines[j].bare.starts_with([' ', '\t', '-']) {
        j += 1;
    }
    j
}

/// Replaces the value of the key `key`. The result is `None` if the front
/// matter has no such key, or if the value is not on the line of the key.
pub fn set_field(text: &str, key: &str, value: &str) -> Option<String> {
    let (_, lines) = fm_lines(text)?;
    let i = key_line(&lines, key)?;
    let l = &lines[i];
    let (_, rest) = split_key(l.bare)?;
    let head = &l.bare[..l.bare.len() - rest.len()];
    let kept = strip_comment(rest);
    let comment = &rest[kept.len()..];
    let lead = &kept[..kept.len() - kept.trim_start().len()];
    let tail = &kept[kept.trim_end().len()..];
    let old = kept.trim();
    if old.starts_with(['[', '{', '&', '*', '!', '|', '>']) {
        return None;
    }
    if old.is_empty() && value_end(&lines, i) > i + 1 {
        // A block list or a block mapping starts here.
        return None;
    }
    let quoted = match old.chars().next() {
        Some('"') => format!("\"{value}\""),
        Some('\'') => format!("'{value}'"),
        _ => value.to_string(),
    };
    let lead = if old.is_empty() && lead.is_empty() {
        " "
    } else {
        lead
    };
    let tail = if old.is_empty() { "" } else { tail };
    let eol = &text[l.start + l.bare.len()..l.end];
    let mut out = String::with_capacity(text.len() + value.len());
    out.push_str(&text[..l.start]);
    out.push_str(&format!("{head}{lead}{quoted}{tail}{comment}{eol}"));
    out.push_str(&text[l.end..]);
    Some(out)
}

/// Inserts the line `key: value` after the line of the key `after`, or as the
/// first line of the front matter if `after` is `None` or has no line. A
/// document with no front matter gets one.
pub fn insert_field(text: &str, key: &str, value: &str, after: Option<&str>) -> String {
    let Some((start, lines)) = fm_lines(text) else {
        let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
        return format!("---{eol}{key}: {value}{eol}---{eol}{text}");
    };
    let eol = match lines.first() {
        Some(l) => &text[l.start + l.bare.len()..l.end],
        None => {
            if text[..start].ends_with("\r\n") {
                "\r\n"
            } else {
                "\n"
            }
        }
    };
    let at = after
        .and_then(|a| key_line(&lines, a))
        .map_or(start, |i| lines[value_end(&lines, i) - 1].end);
    format!("{}{key}: {value}{eol}{}", &text[..at], &text[at..])
}

/// Removes the key `key`, with the lines of its value. The result is `None` if
/// the front matter has no such key.
pub fn remove_field(text: &str, key: &str) -> Option<String> {
    let (_, lines) = fm_lines(text)?;
    let i = key_line(&lines, key)?;
    let end = lines[value_end(&lines, i) - 1].end;
    Some(format!("{}{}", &text[..lines[i].start], &text[end..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(src: &str) -> FrontMatter {
        parse(src, 2).unwrap()
    }

    #[test]
    fn splits_front_matter() {
        let text = "---\nstatus: active\n---\n# T\n";
        let (fm, body) = split(text).unwrap();
        assert_eq!(fm, "status: active\n");
        assert_eq!(&text[body..], "# T\n");
        assert!(split("---\nno end\n").is_none());
        assert!(split("# T\n---\n").is_none());
    }

    #[test]
    fn scalars_and_lists() {
        let fm = p(
            "status: no\ntype: \"spec\"\naliases: [a/b, 'c''d']\ndepends-on:\n  - x/y # why\n  - \"z\"\nowner:\n",
        );
        assert_eq!(fm.get("status").unwrap().0, &Value::Str("no".into()));
        assert_eq!(fm.get("type").unwrap().0, &Value::Str("spec".into()));
        assert_eq!(
            fm.get("aliases").unwrap().0,
            &Value::List(vec!["a/b".into(), "c'd".into()])
        );
        assert_eq!(
            fm.get("depends-on").unwrap(),
            (&Value::List(vec!["x/y".into(), "z".into()]), 5)
        );
        assert_eq!(fm.get("owner").unwrap().0, &Value::Str(String::new()));
    }

    #[test]
    fn outside_the_subset_is_kept_as_unsupported() {
        let fm = p("milestone:\n  alpha: v1\nnotes: |\n  text\nlist:\n- name: a\nok: yes\n");
        assert!(matches!(
            fm.get("milestone").unwrap().0,
            Value::Unsupported(_)
        ));
        assert!(matches!(fm.get("notes").unwrap().0, Value::Unsupported(_)));
        assert!(matches!(fm.get("list").unwrap().0, Value::Unsupported(_)));
        assert_eq!(fm.get("ok").unwrap().0, &Value::Str("yes".into()));
    }

    #[test]
    fn errors_have_lines() {
        assert_eq!(parse("a: 1\na: 2\n", 2).unwrap_err().0, 3);
        assert!(parse("just text\n", 2).is_err());
        assert!(parse("a: \"open\n", 2).is_err());
    }

    #[test]
    fn set_field_keeps_quotes_comment_and_order() {
        let t = |text: &str| set_field(text, "type", "requirements");
        assert_eq!(
            t("---\ntype: \"spec\"  # old name\nstatus: x\n---\n# T\n").unwrap(),
            "---\ntype: \"requirements\"  # old name\nstatus: x\n---\n# T\n"
        );
        assert_eq!(
            t("---\nowner: me\ntype: 'spec'\n---\n").unwrap(),
            "---\nowner: me\ntype: 'requirements'\n---\n"
        );
        assert_eq!(
            t("---\ntype:   spec\n---\n").unwrap(),
            "---\ntype:   requirements\n---\n"
        );
        assert_eq!(
            t("---\r\ntype: spec\r\nstatus: x\r\n---\r\n# T\r\n").unwrap(),
            "---\r\ntype: requirements\r\nstatus: x\r\n---\r\n# T\r\n"
        );
        assert_eq!(
            t("---\ntype:\n---\n").unwrap(),
            "---\ntype: requirements\n---\n"
        );
    }

    #[test]
    fn set_field_refuses_what_it_cannot_edit_on_one_line() {
        let t = |text: &str| set_field(text, "type", "x");
        assert!(t("---\nstatus: draft\n---\n").is_none());
        // A block list is not a value on the line of its key.
        assert!(set_field("---\naliases:\n  - a\n---\n", "aliases", "b").is_none());
        // A key in the body is not a key of the front matter.
        assert!(t("---\na: 1\n---\ntype: spec\n").is_none());
        assert!(t("# T\ntype: spec\n").is_none());
        // A key with a longer name is another key.
        assert!(t("---\ntype-of: spec\n---\n").is_none());
    }

    #[test]
    fn insert_field_goes_first_after_a_key_or_makes_the_front_matter() {
        assert_eq!(
            insert_field("---\nstatus: x\n---\n# T\n", "type", "research", None),
            "---\ntype: research\nstatus: x\n---\n# T\n"
        );
        assert_eq!(
            insert_field(
                "---\ntype: research\nstatus: x\n---\n",
                "was-type",
                "note",
                Some("type")
            ),
            "---\ntype: research\nwas-type: note\nstatus: x\n---\n"
        );
        // The key `after` has no line: the first line.
        assert_eq!(
            insert_field("---\nstatus: x\n---\n", "was-type", "note", Some("type")),
            "---\nwas-type: note\nstatus: x\n---\n"
        );
        assert_eq!(
            insert_field("# T\n", "was-type", "note", None),
            "---\nwas-type: note\n---\n# T\n"
        );
        assert_eq!(
            insert_field("# T\r\nx\r\n", "was-type", "note", None),
            "---\r\nwas-type: note\r\n---\r\n# T\r\nx\r\n"
        );
        assert_eq!(
            insert_field("---\r\nstatus: x\r\n---\r\n", "type", "research", None),
            "---\r\ntype: research\r\nstatus: x\r\n---\r\n"
        );
    }

    #[test]
    fn remove_field_removes_the_line_and_the_lines_of_its_value() {
        assert_eq!(
            remove_field(
                "---\ntype: change\nstatus: done\nowner: me\n---\n# T\n",
                "status"
            )
            .unwrap(),
            "---\ntype: change\nowner: me\n---\n# T\n"
        );
        assert_eq!(
            remove_field("---\r\nstatus: done\r\n---\r\n", "status").unwrap(),
            "---\r\n---\r\n"
        );
        assert_eq!(
            remove_field("---\naliases:\n  - a\n  - b\nstatus: x\n---\n", "aliases").unwrap(),
            "---\nstatus: x\n---\n"
        );
        assert!(remove_field("---\ntype: x\n---\n", "status").is_none());
        assert!(remove_field("# T\nstatus: x\n", "status").is_none());
    }
}
