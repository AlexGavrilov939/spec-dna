//! A minimal JSON writer for `--json` output. Keys keep insertion order.

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

/// Builds one JSON object.
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
}
