//! scan is the Rust half of the spec-dna language benchmark.
//!
//! It walks a tree, parses Markdown front matter, links and id tokens, and
//! resolves them. The Go half in ../go uses the same rules. See ../README.md.
//! It uses only the standard library.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::UNIX_EPOCH;

const VERSION: &str = "0.1.0";
const INDEX_MAGIC: &str = "SDNAIDX1";

#[derive(Clone, Default)]
struct Doc {
    path: String,
    size: u64,
    mtime: i128,
    id: String,
    links: Vec<String>,
    idrefs: Vec<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() == 1 && args[0] == "--version" {
        println!("scan-rust {VERSION}");
        return ExitCode::SUCCESS;
    }
    let mut root = String::new();
    let mut index = String::new();
    let mut list = false;
    let mut excl: HashSet<String> = [".git", "node_modules"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--index" => {
                i += 1;
                index = args[i].clone();
            }
            "--exclude" => {
                i += 1;
                excl.insert(args[i].clone());
            }
            "--list" => list = true,
            a => root = a.to_string(),
        }
        i += 1;
    }
    if root.is_empty() {
        eprintln!("usage: scan [--index FILE] [--exclude NAME] [--list] ROOT");
        return ExitCode::from(2);
    }
    match run(&root, &index, &excl, list) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("scan: {e}");
            ExitCode::from(1)
        }
    }
}

fn walk(
    root: &Path,
    rel: &str,
    excl: &HashSet<String>,
    files: &mut HashSet<String>,
    dirs: &mut HashSet<String>,
    docs: &mut Vec<Doc>,
) -> io::Result<()> {
    let dir = if rel.is_empty() {
        root.to_path_buf()
    } else {
        root.join(rel)
    };
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let r = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let ft = entry.file_type()?;
        if ft.is_dir() {
            if excl.contains(&name) {
                continue;
            }
            dirs.insert(r.clone());
            walk(root, &r, excl, files, dirs, docs)?;
        } else if ft.is_file() {
            if r.ends_with(".md") {
                let md = entry.metadata()?;
                let mtime = md
                    .modified()?
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_nanos() as i128)
                    .unwrap_or(0);
                docs.push(Doc {
                    path: r.clone(),
                    size: md.len(),
                    mtime,
                    ..Default::default()
                });
            }
            files.insert(r);
        }
    }
    Ok(())
}

fn run(root: &str, index: &str, excl: &HashSet<String>, list: bool) -> io::Result<()> {
    let rootp = Path::new(root);
    let mut files = HashSet::new();
    let mut dirs: HashSet<String> = HashSet::new();
    dirs.insert(String::new());
    let mut docs = Vec::new();
    walk(rootp, "", excl, &mut files, &mut dirs, &mut docs)?;
    docs.sort_by(|a, b| a.path.cmp(&b.path));

    let mut cache = if index.is_empty() {
        HashMap::new()
    } else {
        load_index(index)
    };
    let mut todo = Vec::new();
    for (k, d) in docs.iter_mut().enumerate() {
        // Move the cached entry out, so that no clone is necessary.
        match cache.remove(&d.path) {
            Some(c) if c.size == d.size && c.mtime == d.mtime => {
                d.id = c.id;
                d.links = c.links;
                d.idrefs = c.idrefs;
            }
            _ => todo.push(k),
        }
    }
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let next = AtomicUsize::new(0);
    let parsed: Vec<(usize, io::Result<Doc>)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let (todo, docs, next) = (&todo, &docs, &next);
                s.spawn(move || {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= todo.len() {
                            break;
                        }
                        let k = todo[i];
                        let mut d = docs[k].clone();
                        let r = fs::read(rootp.join(&d.path)).map(|b| {
                            parse(&mut d, &String::from_utf8_lossy(&b));
                            d
                        });
                        out.push((k, r));
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    for (k, r) in parsed {
        docs[k] = r?;
    }

    // Resolve the links and the id tokens.
    let mut ids: HashMap<&str, u32> = HashMap::new();
    let mut prefixes: HashSet<&str> = HashSet::new();
    let mut bytes = 0u64;
    for d in &docs {
        bytes += d.size;
        if !d.id.is_empty() {
            *ids.entry(&d.id).or_insert(0) += 1;
            if let Some(p) = id_prefix(&d.id) {
                prefixes.insert(p);
            }
        }
    }
    let (mut links, mut internal, mut idrefs) = (0u64, 0u64, 0u64);
    let mut broken = Vec::new();
    let mut unresolved = Vec::new();
    for d in &docs {
        let dir = match d.path.rfind('/') {
            Some(k) => &d.path[..k],
            None => "",
        };
        for l in &d.links {
            links += 1;
            let Some(t) = internal_target(l) else {
                continue;
            };
            internal += 1;
            let full = if let Some(stripped) = t.strip_prefix('/') {
                clean_path(stripped.trim_start_matches('/'))
            } else if dir.is_empty() {
                clean_path(&t)
            } else {
                clean_path(&format!("{dir}/{t}"))
            };
            if !files.contains(&full) && !dirs.contains(&full) {
                broken.push(format!("{} -> {}", d.path, l));
            }
        }
        for r in &d.idrefs {
            match id_prefix(r) {
                Some(p) if prefixes.contains(p) => {}
                _ => continue,
            }
            idrefs += 1;
            if !ids.contains_key(r.as_str()) {
                unresolved.push(format!("{} -> {}", d.path, r));
            }
        }
    }
    let dups = ids.values().filter(|&&n| n > 1).count();
    if !index.is_empty() && !todo.is_empty() {
        save_index(index, &docs)?;
    }
    let stdout = io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    writeln!(
        w,
        "files_walked {}\ndocs {}\nbytes {}\nlinks {}\nlinks_internal {}\nlinks_broken {}",
        files.len(),
        docs.len(),
        bytes,
        links,
        internal,
        broken.len()
    )?;
    writeln!(
        w,
        "ids {}\nid_dups {}\nidrefs {}\nidrefs_unresolved {}\nparsed {}",
        ids.len(),
        dups,
        idrefs,
        unresolved.len(),
        todo.len()
    )?;
    if list {
        broken.sort();
        unresolved.sort();
        for b in &broken {
            writeln!(w, "broken {b}")?;
        }
        for u in &unresolved {
            writeln!(w, "unresolved {u}")?;
        }
    }
    Ok(())
}

/// Cleans a path lexically, like path.Clean in Go, but returns "" for the root.
fn clean_path(p: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(out.last(), Some(&s) if s != "..") {
                    out.pop();
                } else {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    out.join("/")
}

fn parse(d: &mut Doc, s: &str) {
    let lines: Vec<&str> = s.split('\n').collect();
    let mut start = 0;
    if !lines.is_empty() && lines[0].trim_end_matches('\r') == "---" {
        for j in 1..lines.len() {
            let l = lines[j].trim_end_matches('\r');
            if l == "---" || l == "..." {
                for fl in &lines[1..j] {
                    if let Some((k, v)) = fl.split_once(':')
                        && k == "id"
                    {
                        d.id = v.trim().trim_matches(|c| c == '"' || c == '\'').to_string();
                    }
                }
                start = j + 1;
                break;
            }
        }
    }
    if d.id.is_empty() {
        let base = d.path.rsplit('/').next().unwrap_or("");
        d.id = leading_id(base);
    }
    let (mut fence_ch, mut fence_n) = (0u8, 0usize);
    for raw in &lines[start..] {
        let l = raw.trim_end_matches('\r');
        let (ch, n) = fence(l);
        if n > 0 {
            if fence_n == 0 {
                fence_ch = ch;
                fence_n = n;
                continue;
            }
            if ch == fence_ch && n >= fence_n {
                fence_n = 0;
                continue;
            }
        }
        if fence_n > 0 {
            continue;
        }
        let l = strip_code(l);
        let prose = find_links(&l, &mut d.links);
        find_ids(&prose, &mut d.idrefs);
    }
}

fn fence(l: &str) -> (u8, usize) {
    let b = l.as_bytes();
    let mut i = 0;
    while i < b.len() && i < 3 && b[i] == b' ' {
        i += 1;
    }
    if i >= b.len() || (b[i] != b'`' && b[i] != b'~') {
        return (0, 0);
    }
    let ch = b[i];
    let mut n = 0;
    while i + n < b.len() && b[i + n] == ch {
        n += 1;
    }
    if n < 3 { (0, 0) } else { (ch, n) }
}

fn strip_code(l: &str) -> String {
    let b = l.as_bytes();
    if !b.contains(&b'`') {
        return l.to_string();
    }
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' {
            out.push(b[i]);
            i += 1;
            continue;
        }
        let mut n = 0;
        while i + n < b.len() && b[i + n] == b'`' {
            n += 1;
        }
        let mut end = None;
        let mut j = i + n;
        while j < b.len() {
            if b[j] != b'`' {
                j += 1;
                continue;
            }
            let mut m = 0;
            while j + m < b.len() && b[j + m] == b'`' {
                m += 1;
            }
            if m == n {
                end = Some(j + m);
                break;
            }
            j += m;
        }
        match end {
            None => {
                out.extend_from_slice(&b[i..i + n]);
                i += n;
            }
            Some(e) => i = e,
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Pushes inline link targets to `out`. Returns the line without them, for the id scan.
fn find_links(l: &str, out: &mut Vec<String>) -> String {
    let mut prose = String::with_capacity(l.len());
    let mut rest = l;
    while let Some(k) = rest.find("](") {
        prose.push_str(&rest[..k + 2]);
        let r = &rest[k + 2..];
        if let Some(stripped) = r.strip_prefix('<') {
            match stripped.find('>') {
                None => {
                    rest = r;
                    continue;
                }
                Some(e) => {
                    if e > 0 {
                        out.push(stripped[..e].to_string());
                    }
                    rest = &stripped[e + 1..];
                }
            }
        } else {
            let e = r.find([')', ' ', '\t']).unwrap_or(r.len());
            if e > 0 {
                out.push(r[..e].to_string());
            }
            rest = &r[e..];
        }
    }
    prose.push_str(rest);
    prose
}

fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}
fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}
fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-'
}

/// Returns the length of an id token PREFIX-NUM at s[i..], or 0. The Go half uses the same rule.
fn match_id(s: &[u8], i: usize) -> usize {
    let mut j = i;
    if j >= s.len() || !is_upper(s[j]) {
        return 0;
    }
    j += 1;
    while j < s.len() && j - i < 10 && (is_upper(s[j]) || is_digit(s[j])) {
        j += 1;
    }
    if j - i < 2 || j >= s.len() || s[j] != b'-' {
        return 0;
    }
    j += 1;
    let k = j;
    while j < s.len() && j - k < 7 && is_digit(s[j]) {
        j += 1;
    }
    if j == k || j - k > 6 {
        return 0;
    }
    if j < s.len() && is_word(s[j]) && s[j] != b'-' {
        return 0;
    }
    j - i
}

fn find_ids(l: &str, out: &mut Vec<String>) {
    let b = l.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if is_upper(b[i]) && !(i > 0 && is_word(b[i - 1])) {
            let n = match_id(b, i);
            if n > 0 {
                out.push(l[i..i + n].to_string());
                i += n;
                continue;
            }
        }
        i += 1;
    }
}

fn leading_id(name: &str) -> String {
    let n = match_id(name.as_bytes(), 0);
    name[..n].to_string()
}

fn id_prefix(id: &str) -> Option<&str> {
    match id.find('-') {
        Some(k) if k > 0 => Some(&id[..k]),
        _ => None,
    }
}

fn internal_target(t: &str) -> Option<String> {
    let b = t.as_bytes();
    if b.first() == Some(&b'#') {
        return None;
    }
    for (i, &c) in b.iter().enumerate() {
        if c == b':' && i > 0 {
            return None;
        }
        let ok = c.is_ascii_alphabetic()
            || (i > 0 && (is_digit(c) || c == b'+' || c == b'.' || c == b'-'));
        if !ok {
            break;
        }
    }
    let t = match t.find(['#', '?']) {
        Some(k) => &t[..k],
        None => t,
    };
    if t.is_empty() {
        return None;
    }
    if !t.contains('%') {
        return Some(t.to_string());
    }
    let b = t.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&t[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    Some(String::from_utf8_lossy(&out).into_owned())
}

fn load_index(p: &str) -> HashMap<String, Doc> {
    let mut out = HashMap::new();
    let Ok(s) = fs::read_to_string(p) else {
        return out;
    };
    let mut lines = s.split('\n');
    if lines.next() != Some(INDEX_MAGIC) {
        return out;
    }
    for l in lines {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() != 6 {
            continue;
        }
        let (Ok(size), Ok(mtime)) = (f[1].parse::<u64>(), f[2].parse::<i128>()) else {
            continue;
        };
        let split = |s: &str| {
            if s.is_empty() {
                Vec::new()
            } else {
                s.split('\x1f').map(String::from).collect()
            }
        };
        out.insert(
            f[0].to_string(),
            Doc {
                path: f[0].to_string(),
                size,
                mtime,
                id: f[3].to_string(),
                links: split(f[4]),
                idrefs: split(f[5]),
            },
        );
    }
    out
}

fn clean(s: &str) -> bool {
    !s.contains(['\t', '\n', '\x1f'])
}

fn save_index(p: &str, docs: &[Doc]) -> io::Result<()> {
    let tmp = format!("{p}.tmp");
    {
        let mut w = BufWriter::new(fs::File::create(&tmp)?);
        writeln!(w, "{INDEX_MAGIC}")?;
        for d in docs {
            if !(clean(&d.path) && clean(&d.id) && d.links.iter().all(|x| clean(x))) {
                continue;
            }
            writeln!(
                w,
                "{}\t{}\t{}\t{}\t{}\t{}",
                d.path,
                d.size,
                d.mtime,
                d.id,
                d.links.join("\x1f"),
                d.idrefs.join("\x1f")
            )?;
        }
        w.flush()?;
    }
    fs::rename(&tmp, p)
}
