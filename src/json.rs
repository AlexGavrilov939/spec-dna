//! A minimal JSON writer for `--json` output, and a minimal reader for agent
//! transcripts. The writer keeps the insertion order of keys.

use std::fmt::Write;

pub fn string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

pub struct Obj {
    buf: String,
    first: bool,
}

impl Obj {
    pub fn new() -> Self {
        Obj {
            buf: String::from("{"),
            first: true,
        }
    }

    fn key(&mut self, k: &str) {
        if !self.first {
            self.buf.push(',');
        }
        self.first = false;
        string(&mut self.buf, k);
        self.buf.push(':');
    }

    pub fn str(mut self, k: &str, v: &str) -> Self {
        self.key(k);
        string(&mut self.buf, v);
        self
    }

    pub fn opt_str(self, k: &str, v: Option<&str>) -> Self {
        match v {
            Some(v) => self.str(k, v),
            None => self.raw(k, "null"),
        }
    }

    pub fn num(mut self, k: &str, v: impl std::fmt::Display) -> Self {
        self.key(k);
        let _ = write!(self.buf, "{v}");
        self
    }

    pub fn bool(self, k: &str, v: bool) -> Self {
        self.raw(k, if v { "true" } else { "false" })
    }

    pub fn raw(mut self, k: &str, json: &str) -> Self {
        self.key(k);
        self.buf.push_str(json);
        self
    }

    pub fn strs(mut self, k: &str, items: &[String]) -> Self {
        self.key(k);
        self.buf.push('[');
        for (i, s) in items.iter().enumerate() {
            if i > 0 {
                self.buf.push(',');
            }
            string(&mut self.buf, s);
        }
        self.buf.push(']');
        self
    }

    pub fn end(mut self) -> String {
        self.buf.push('}');
        self.buf
    }
}

pub fn array(items: impl IntoIterator<Item = String>) -> String {
    let mut out = String::from("[");
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&item);
    }
    out.push(']');
    out
}

/// A parsed JSON value. Objects keep their keys in the source order.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    /// Returns the value of a key of an object. For any other value, or for a
    /// missing key, it returns `Null`.
    pub fn get(&self, key: &str) -> &Value {
        match self {
            Value::Obj(fields) => fields
                .iter()
                .find(|(k, _)| k == key)
                .map_or(&Value::Null, |(_, v)| v),
            _ => &Value::Null,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> &[Value] {
        match self {
            Value::Arr(a) => a,
            _ => &[],
        }
    }

    /// Returns a non-negative integer. Any other value gives 0.
    pub fn as_u64(&self) -> u64 {
        match self {
            Value::Num(n) if *n >= 0.0 => *n as u64,
            _ => 0,
        }
    }

    pub fn is_true(&self) -> bool {
        matches!(self, Value::Bool(true))
    }
}

/// Parses one JSON text (RFC 8259). Numbers become `f64`.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Reader {
        b: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("text after the value at byte {}", p.i));
    }
    Ok(v)
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
    depth: u32,
}

impl Reader<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn err<T>(&self, what: &str) -> Result<T, String> {
        Err(format!("{what} at byte {}", self.i))
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.b[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        // A limit on nesting keeps a damaged file from overflowing the stack.
        if self.depth > 256 {
            return self.err("nesting too deep");
        }
        match self.b.get(self.i) {
            None => self.err("unexpected end"),
            Some(b'{') => {
                self.i += 1;
                self.depth += 1;
                let mut fields = Vec::new();
                self.ws();
                if self.eat("}") {
                    self.depth -= 1;
                    return Ok(Value::Obj(fields));
                }
                loop {
                    self.ws();
                    if self.b.get(self.i) != Some(&b'"') {
                        return self.err("expected a key");
                    }
                    let k = self.string()?;
                    self.ws();
                    if !self.eat(":") {
                        return self.err("expected `:`");
                    }
                    self.ws();
                    let v = self.value()?;
                    fields.push((k, v));
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("}") {
                        self.depth -= 1;
                        return Ok(Value::Obj(fields));
                    }
                    return self.err("expected `,` or `}`");
                }
            }
            Some(b'[') => {
                self.i += 1;
                self.depth += 1;
                let mut items = Vec::new();
                self.ws();
                if self.eat("]") {
                    self.depth -= 1;
                    return Ok(Value::Arr(items));
                }
                loop {
                    self.ws();
                    items.push(self.value()?);
                    self.ws();
                    if self.eat(",") {
                        continue;
                    }
                    if self.eat("]") {
                        self.depth -= 1;
                        return Ok(Value::Arr(items));
                    }
                    return self.err("expected `,` or `]`");
                }
            }
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') if self.eat("true") => Ok(Value::Bool(true)),
            Some(b'f') if self.eat("false") => Ok(Value::Bool(false)),
            Some(b'n') if self.eat("null") => Ok(Value::Null),
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let start = self.i;
                while self.i < self.b.len()
                    && matches!(
                        self.b[self.i],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.i += 1;
                }
                let s = std::str::from_utf8(&self.b[start..self.i]).unwrap_or("");
                s.parse()
                    .map(Value::Num)
                    .or_else(|_| self.err("bad number"))
            }
            Some(_) => self.err("unexpected character"),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.i += 1; // the opening quote
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&c) = self.b.get(self.i) else {
                return self.err("unterminated string");
            };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.b.get(self.i) else {
                        return self.err("unterminated escape");
                    };
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut cp = self.hex4()?;
                            // A surrogate pair encodes one code point above U+FFFF.
                            if (0xD800..0xDC00).contains(&cp)
                                && self.b[self.i..].starts_with(b"\\u")
                                && let Some(lo) = self
                                    .b
                                    .get(self.i + 2..self.i + 6)
                                    .and_then(|h| std::str::from_utf8(h).ok())
                                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                                    .filter(|lo| (0xDC00..0xE000).contains(lo))
                            {
                                // Only a low surrogate completes the pair.
                                // Any other escape stays for the next step.
                                self.i += 6;
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                            }
                            let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return self.err("bad escape"),
                    }
                }
                _ => out.push(c),
            }
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let s = self
            .b
            .get(self.i..self.i + 4)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u32::from_str_radix(h, 16).ok());
        match s {
            Some(v) => {
                self.i += 4;
                Ok(v)
            }
            None => self.err("bad \\u escape"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_orders_keys() {
        let s = Obj::new()
            .str("b", "q\"\n")
            .num("a", 1)
            .bool("c", false)
            .end();
        assert_eq!(s, r#"{"b":"q\"\n","a":1,"c":false}"#);
        assert_eq!(array(vec!["1".into(), "2".into()]), "[1,2]");
    }

    #[test]
    fn reads_values() {
        let v = parse(r#" {"a": [1, -2.5e1, true, null], "s": "q\"é😀\n", "o": {}} "#).unwrap();
        assert_eq!(v.get("a").as_arr().len(), 4);
        assert_eq!(v.get("a").as_arr()[0].as_u64(), 1);
        assert_eq!(v.get("a").as_arr()[1], Value::Num(-25.0));
        assert!(v.get("a").as_arr()[2].is_true());
        assert_eq!(v.get("s").as_str(), Some("q\"é😀\n"));
        assert_eq!(v.get("o"), &Value::Obj(Vec::new()));
        // A lone high surrogate becomes U+FFFD and keeps the next escape.
        assert_eq!(
            parse(r#""\ud83d\u0041""#).unwrap().as_str(),
            Some("\u{FFFD}A")
        );
        assert_eq!(v.get("missing"), &Value::Null);
        for bad in ["{", "[1,]", "{\"a\" 1}", "\"x", "1 2", "nul"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        let deep = "[".repeat(300) + &"]".repeat(300);
        assert!(parse(&deep).is_err());
    }
}
