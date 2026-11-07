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
}
