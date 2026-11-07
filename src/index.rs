//! The index contains parsed documents, keyed by git blob id (ADR 0001,
//! "Concurrency"). It is a disposable cache in the git directory, so it is
//! never part of a commit. A write goes to a temporary file, and then a rename
//! puts that file in place. Thus a reader sees the old or the new index, never
//! a partial one.

use crate::frontmatter::{FrontMatter, Value};
use crate::parse::{Chunk, PARSER_VERSION, ParsedDoc, Ref, RefKind, Req, Token};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAGIC: &[u8; 8] = b"SPECDNA\0";
const FORMAT: u32 = 1;

pub struct Index {
    pub path: PathBuf,
    pub entries: HashMap<String, Arc<ParsedDoc>>,
}

impl Index {
    pub fn empty(path: PathBuf) -> Index {
        Index {
            path,
            entries: HashMap::new(),
        }
    }

    /// Loads the index. A missing, old or damaged file gives an empty index.
    pub fn load(path: PathBuf) -> Index {
        let entries = std::fs::read(&path)
            .ok()
            .and_then(|b| decode(&b))
            .unwrap_or_default();
        Index { path, entries }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = self.path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir)?;
        let tmp = dir.join(format!(".index.{}.tmp", std::process::id()));
        std::fs::write(&tmp, encode(&self.entries))?;
        std::fs::rename(&tmp, &self.path).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }
}

fn encode(entries: &HashMap<String, Arc<ParsedDoc>>) -> Vec<u8> {
    let mut w = W(Vec::with_capacity(entries.len() * 512));
    w.0.extend_from_slice(MAGIC);
    w.u32(FORMAT);
    w.u32(PARSER_VERSION);
    w.u32(entries.len() as u32);
    let mut keys: Vec<&String> = entries.keys().collect();
    keys.sort();
    for k in keys {
        w.str(k);
        doc(&mut w, &entries[k]);
    }
    w.0
}

fn decode(b: &[u8]) -> Option<HashMap<String, Arc<ParsedDoc>>> {
    let mut r = R { b, i: 0 };
    if r.take(8)? != MAGIC || r.u32()? != FORMAT || r.u32()? != PARSER_VERSION {
        return None;
    }
    let n = r.u32()? as usize;
    let mut out = HashMap::with_capacity(n);
    for _ in 0..n {
        let k = r.str()?;
        out.insert(k, Arc::new(read_doc(&mut r)?));
    }
    (r.i == b.len()).then_some(out)
}

struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }
    fn strs(&mut self, v: &[String]) {
        self.u32(v.len() as u32);
        for s in v {
            self.str(s);
        }
    }
}

struct R<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> R<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.i..self.i.checked_add(n)?)?;
        self.i += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn str(&mut self) -> Option<String> {
        let n = self.u32()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }
    fn strs(&mut self) -> Option<Vec<String>> {
        let n = self.u32()? as usize;
        (0..n).map(|_| self.str()).collect()
    }
    fn hash(&mut self) -> Option<[u8; 8]> {
        self.take(8)?.try_into().ok()
    }
}

const UNSUPPORTED: [&str; 7] = [
    "a nested mapping or a list of mappings",
    "a multi-line scalar",
    "a nested flow collection",
    "a flow mapping",
    "an anchor",
    "an alias",
    "a tag",
];

fn doc(w: &mut W, d: &ParsedDoc) {
    match &d.front_matter {
        None => w.u8(0),
        Some(fm) => {
            w.u8(1);
            w.u32(fm.fields.len() as u32);
            for (k, v, line) in &fm.fields {
                w.str(k);
                w.u32(*line);
                match v {
                    Value::Str(s) => {
                        w.u8(0);
                        w.str(s);
                    }
                    Value::List(l) => {
                        w.u8(1);
                        w.strs(l);
                    }
                    Value::Unsupported(what) => {
                        w.u8(2);
                        w.u8(UNSUPPORTED.iter().position(|u| u == what).unwrap_or(0) as u8);
                    }
                }
            }
        }
    }
    match &d.front_matter_error {
        None => w.u8(0),
        Some((line, msg)) => {
            w.u8(1);
            w.u32(*line);
            w.str(msg);
        }
    }
    w.u32(d.body_start);
    w.str(&d.title);
    w.u32(d.chunks.len() as u32);
    for c in &d.chunks {
        w.u8(c.level);
        w.str(&c.heading);
        w.str(&c.anchor);
        for v in [c.start, c.end, c.line, c.chars] {
            w.u32(v);
        }
        w.0.extend_from_slice(&c.hash);
        w.u8(c.flags);
    }
    w.u32(d.refs.len() as u32);
    for r in &d.refs {
        w.u8(r.kind as u8);
        w.str(&r.target);
        w.u32(r.line);
    }
    w.u32(d.tokens.len() as u32);
    for t in &d.tokens {
        w.str(&t.text);
        w.u32(t.line);
    }
    w.u32(d.reqs.len() as u32);
    for r in &d.reqs {
        w.str(&r.key);
        for v in [r.line, r.start, r.end, r.chunk] {
            w.u32(v);
        }
        w.0.extend_from_slice(&r.hash);
        w.strs(&r.aliases);
        w.str(&r.text);
    }
    w.u32(d.body_chars);
    w.0.extend_from_slice(&d.frozen_hash);
    w.u32(d.amendments.unwrap_or(u32::MAX));
}

fn read_doc(r: &mut R) -> Option<ParsedDoc> {
    let mut d = ParsedDoc::default();
    if r.u8()? == 1 {
        let n = r.u32()? as usize;
        let mut fm = FrontMatter::default();
        for _ in 0..n {
            let k = r.str()?;
            let line = r.u32()?;
            let v = match r.u8()? {
                0 => Value::Str(r.str()?),
                1 => Value::List(r.strs()?),
                _ => Value::Unsupported(
                    UNSUPPORTED
                        .get(r.u8()? as usize)
                        .copied()
                        .unwrap_or(UNSUPPORTED[0]),
                ),
            };
            fm.fields.push((k, v, line));
        }
        d.front_matter = Some(fm);
    }
    if r.u8()? == 1 {
        d.front_matter_error = Some((r.u32()?, r.str()?));
    }
    d.body_start = r.u32()?;
    d.title = r.str()?;
    for _ in 0..r.u32()? {
        d.chunks.push(Chunk {
            level: r.u8()?,
            heading: r.str()?,
            anchor: r.str()?,
            start: r.u32()?,
            end: r.u32()?,
            line: r.u32()?,
            chars: r.u32()?,
            hash: r.hash()?,
            flags: r.u8()?,
        });
    }
    for _ in 0..r.u32()? {
        let kind = match r.u8()? {
            0 => RefKind::Link,
            1 => RefKind::Image,
            _ => RefKind::Wiki,
        };
        d.refs.push(Ref {
            kind,
            target: r.str()?,
            line: r.u32()?,
        });
    }
    for _ in 0..r.u32()? {
        d.tokens.push(Token {
            text: r.str()?,
            line: r.u32()?,
        });
    }
    for _ in 0..r.u32()? {
        d.reqs.push(Req {
            key: r.str()?,
            line: r.u32()?,
            start: r.u32()?,
            end: r.u32()?,
            chunk: r.u32()?,
            hash: r.hash()?,
            aliases: r.strs()?,
            text: r.str()?,
        });
    }
    d.body_chars = r.u32()?;
    d.frozen_hash = r.hash()?;
    let a = r.u32()?;
    d.amendments = (a != u32::MAX).then_some(a);
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let text = "---\nstatus: active\nx: {a: 1}\n---\n# T\n\n- **k**: The system SHALL run. <!-- aliases: a#b -->\n\n[l](x.md) [[a/b]] ADR-1\n\n## Amendments\n";
        let mut m = HashMap::new();
        m.insert("abc".to_string(), Arc::new(crate::parse::parse(text)));
        m.insert(
            "bad".to_string(),
            Arc::new(crate::parse::parse("---\n: x\n---\n")),
        );
        let back = decode(&encode(&m)).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(*back["abc"], *m["abc"]);
        assert_eq!(*back["bad"], *m["bad"]);
        let mut bytes = encode(&m);
        bytes.pop();
        assert!(decode(&bytes).is_none());
    }
}
