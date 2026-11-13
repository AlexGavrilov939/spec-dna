//! Path globs for the configuration and for `applies-to`:
//!
//! - `**` matches any number of path segments
//! - `*` matches any run of characters in one segment
//! - `?` matches one character
//!
//! spec-dna matches a pattern against the whole `/`-separated path.

pub fn is_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?'])
}

pub fn matches(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = pattern.trim_matches('/').split('/').collect();
    let segs: Vec<&str> = path.trim_matches('/').split('/').collect();
    match_segments(&pat, &segs)
}

/// A glob matches the path. A plain value matches the path itself or any
/// path below it (a namespace or a directory).
pub fn matches_scope(pattern: &str, path: &str) -> bool {
    if is_glob(pattern) {
        return matches(pattern, path);
    }
    let p = pattern.trim_matches('/');
    p.is_empty()
        || path == p
        || (path.starts_with(p) && path.as_bytes().get(p.len()) == Some(&b'/'))
}

fn match_segments(pat: &[&str], segs: &[&str]) -> bool {
    match pat.split_first() {
        None => segs.is_empty(),
        Some((&"**", rest)) => (0..=segs.len()).any(|i| match_segments(rest, &segs[i..])),
        Some((p, rest)) => match segs.split_first() {
            Some((s, srest)) => {
                match_segment(p.as_bytes(), s.as_bytes()) && match_segments(rest, srest)
            }
            None => false,
        },
    }
}

fn match_segment(p: &[u8], s: &[u8]) -> bool {
    // This is an iterative wildcard match with one backtrack point.
    let (mut pi, mut si) = (0, 0);
    let (mut star, mut mark) = (usize::MAX, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == s[si]) {
            if p[pi] == b'?' {
                si += utf8_len(s[si]);
            } else {
                si += 1;
            }
            pi += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = pi;
            mark = si;
            pi += 1;
        } else if star != usize::MAX {
            pi = star + 1;
            mark += utf8_len(s[mark]);
            si = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

fn utf8_len(first: u8) -> usize {
    match first {
        0xF0..=0xFF => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_wildcards() {
        assert!(matches("docs/*.md", "docs/a.md"));
        assert!(!matches("docs/*.md", "docs/x/a.md"));
        assert!(matches("docs/?.md", "docs/a.md"));
        assert!(matches("a*b*c", "aXXbYYc"));
        assert!(!matches("a*b*c", "aXXbYY"));
    }

    #[test]
    fn double_star_spans_segments() {
        assert!(matches("**/vendor/**", "x/vendor/y/z.md"));
        assert!(matches("**/vendor/**", "vendor/z.md"));
        assert!(matches("src/**", "src/a/b.rs"));
        assert!(matches("src/**/*.rs", "src/a.rs"));
        assert!(!matches("src/**", "lib/a.rs"));
    }

    #[test]
    fn scopes() {
        assert!(matches_scope("auth", "auth/session"));
        assert!(matches_scope("auth", "auth"));
        assert!(!matches_scope("auth", "authz/session"));
        assert!(matches_scope("auth/*", "auth/session"));
        assert!(matches_scope("", "anything"));
    }
}
