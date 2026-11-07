//! `spec-dna mv`: moves a document or one requirement and records the old
//! id as an alias, so existing references keep resolving (ADR 0001,
//! "Identifiers" and "References and requirements").

use crate::config::Config;
use crate::frontmatter::{self, Value};
use crate::model::{Corpus, is_doc_path};
use crate::repo::{Fatal, Repo};
use std::path::Path;

pub struct Moved {
    pub kind: &'static str,
    pub from: String,
    pub to: String,
    pub alias: String,
    pub files: Vec<String>,
}

pub fn run(repo: &Repo, config: &Config, c: &Corpus, from: &str, to: &str) -> Result<Moved, Fatal> {
    let (d, frag) = crate::context::resolve_target(c, from)
        .ok_or_else(|| Fatal::NotFound(format!("`{from}` names no document")))?;
    match frag {
        Some(key) => move_requirement(repo, c, d, &key, to),
        None => move_document(repo, config, c, d, to),
    }
}

fn move_document(
    repo: &Repo,
    config: &Config,
    c: &Corpus,
    d: u32,
    to: &str,
) -> Result<Moved, Fatal> {
    let doc = &c.docs[d as usize];
    let root = &config.roots[doc.root];
    let to_path = if is_doc_path(to) {
        to.trim_start_matches("./").to_string()
    } else if root.path.is_empty() {
        format!("{to}.md")
    } else {
        format!("{}/{to}.md", root.path)
    };
    let new_root = config
        .root_of(&to_path)
        .filter(|r| !config.is_excluded(*r, &to_path))
        .ok_or_else(|| {
            Fatal::Usage(format!(
                "`{to_path}` is not under a document root of spec-dna.toml"
            ))
        })?;
    let new_id = crate::model::root_doc_id(&config.roots[new_root], &to_path);
    if new_id == doc.id {
        return Err(Fatal::Usage(format!(
            "`{to_path}` has the same id `{new_id}`"
        )));
    }
    if repo.root.join(&to_path).exists() || c.lookup.contains_key(&crate::model::canonical(&new_id))
    {
        return Err(Fatal::Usage(format!(
            "`{to_path}` (id `{new_id}`) already exists"
        )));
    }
    // Keep the old id, and legacy aliases the new file name would lose.
    let new_slug = new_id.rsplit('/').next().unwrap_or(&new_id);
    let regained: Vec<String> = config.roots[new_root]
        .legacy_ids
        .iter()
        .filter_map(|l| l.alias_for(new_slug))
        .collect();
    let mut add = vec![doc.id.clone()];
    add.extend(
        doc.legacy_aliases
            .iter()
            .filter(|a| !regained.contains(a))
            .cloned(),
    );
    let text = read(repo, &doc.path)?;
    let old_dir = parent(&doc.path);
    let updated = rewrite_links(
        &add_aliases(&text, &add),
        old_dir,
        parent(&to_path),
        &doc.path,
        &to_path,
        c,
    );
    // Relative links from other documents to the moved file.
    let mut others = Vec::new();
    for o in c.docs.iter().filter(|o| o.path != doc.path) {
        let points_here = o.parsed.refs.iter().any(|r| {
            r.kind != crate::parse::RefKind::Wiki
                && matches!(c.resolve_link(&o.path, &r.target), crate::model::LinkRes::File(ref p, _, _) if *p == doc.path)
        });
        if points_here {
            let t = read(repo, &o.path)?;
            let dir = parent(&o.path);
            let nt = rewrite_links(&t, dir, dir, &doc.path, &to_path, c);
            if nt != t {
                others.push((o.path.clone(), nt));
            }
        }
    }

    let dest = repo.root.join(&to_path);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| Fatal::Io(e.to_string()))?;
    }
    if repo.git.is_tracked(&doc.path) {
        repo.git.mv(&doc.path, &to_path).map_err(Fatal::Io)?;
    } else {
        std::fs::rename(repo.root.join(&doc.path), &dest).map_err(|e| Fatal::Io(e.to_string()))?;
    }
    write(&dest, &updated)?;
    let mut files = vec![doc.path.clone(), to_path];
    for (p, t) in others {
        write(&repo.root.join(&p), &t)?;
        files.push(p);
    }
    Ok(Moved {
        kind: "document",
        from: doc.id.clone(),
        to: new_id,
        alias: add.join(", "),
        files,
    })
}

fn move_requirement(repo: &Repo, c: &Corpus, d: u32, key: &str, to: &str) -> Result<Moved, Fatal> {
    let src = &c.docs[d as usize];
    let ri = src
        .req(key)
        .ok_or_else(|| Fatal::NotFound(format!("`{}` has no requirement `{key}`", src.id)))?;
    let req = &src.parsed.reqs[ri];
    let (t, tfrag) = crate::context::resolve_target(c, to)
        .ok_or_else(|| Fatal::NotFound(format!("`{to}` names no document")))?;
    if tfrag.is_some() {
        return Err(Fatal::Usage(
            "the destination of a requirement is a document id, without `#`".into(),
        ));
    }
    let dst = &c.docs[t as usize];
    if t == d {
        return Err(Fatal::Usage(
            "the requirement is already in that document".into(),
        ));
    }
    if !dst.ty.has_requirements() {
        return Err(Fatal::Usage(format!(
            "`{}` is a {}; requirements live in a spec or contract",
            dst.id,
            dst.ty.name()
        )));
    }
    if dst.req(&req.key).is_some() {
        return Err(Fatal::Usage(format!(
            "`{}` already has a requirement `{}`",
            dst.id, req.key
        )));
    }
    let src_text = read(repo, &src.path)?;
    let (a, b) = line_span(&src_text, req.start as usize, req.end as usize);
    let item = &src_text[a..b];
    let old = format!("{}#{}", src.id, req.key);
    let item = add_req_alias(&dedent(item), &old);
    let new_src = format!("{}{}", &src_text[..a], &src_text[b..]);

    let dst_text = read(repo, &dst.path)?;
    let new_dst = match dst
        .reqs
        .iter()
        .map(|&i| &dst.parsed.reqs[i])
        .max_by_key(|r| r.end)
    {
        Some(last) => {
            let (_, end) = line_span(&dst_text, last.start as usize, last.end as usize);
            format!("{}{}{}", &dst_text[..end], item, &dst_text[end..])
        }
        None => {
            let sep = if dst_text.ends_with("\n\n") {
                ""
            } else if dst_text.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            format!("{dst_text}{sep}## Requirements\n\n{item}")
        }
    };
    write(&repo.root.join(&src.path), &new_src)?;
    write(&repo.root.join(&dst.path), &new_dst)?;
    Ok(Moved {
        kind: "requirement",
        from: old.clone(),
        to: format!("{}#{}", dst.id, req.key),
        alias: old,
        files: vec![src.path.clone(), dst.path.clone()],
    })
}

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

/// Rewrites relative Markdown link destinations of a document that moves
/// from `old_dir` to `new_dir`, or that links to a moved file. Links that do
/// not resolve, external links and code blocks are left alone.
fn rewrite_links(
    text: &str,
    old_dir: &str,
    new_dir: &str,
    moved_from: &str,
    moved_to: &str,
    c: &Corpus,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut fence: Option<&str> = None;
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if let Some(f) = fence {
            if t.starts_with(f) {
                fence = None;
            }
            out.push_str(line);
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = Some(&t[..3]);
            out.push_str(line);
            continue;
        }
        let mut rest = line;
        // Reference definitions: `[label]: dest`.
        if t.starts_with('[')
            && let Some(i) = t.find("]: ")
        {
            let lead = line.len() - t.len() + i + 3;
            let dest_end = line[lead..]
                .find(char::is_whitespace)
                .map_or(line.len(), |j| lead + j);
            out.push_str(&line[..lead]);
            out.push_str(&new_dest(
                &line[lead..dest_end],
                old_dir,
                new_dir,
                moved_from,
                moved_to,
                c,
            ));
            rest = &line[dest_end..];
        }
        while let Some(i) = rest.find("](") {
            out.push_str(&rest[..i + 2]);
            rest = &rest[i + 2..];
            let end = rest
                .find(|ch: char| ch == ')' || ch.is_whitespace())
                .unwrap_or(rest.len());
            out.push_str(&new_dest(
                &rest[..end],
                old_dir,
                new_dir,
                moved_from,
                moved_to,
                c,
            ));
            rest = &rest[end..];
        }
        out.push_str(rest);
    }
    out
}

fn new_dest(
    dest: &str,
    old_dir: &str,
    new_dir: &str,
    moved_from: &str,
    moved_to: &str,
    c: &Corpus,
) -> String {
    let keep = dest.to_string();
    if dest.is_empty()
        || dest.starts_with('#')
        || dest.starts_with('<')
        || dest.starts_with("//")
        || dest.contains("://")
        || dest.starts_with("mailto:")
    {
        return keep;
    }
    let split = dest.find(['#', '?']).unwrap_or(dest.len());
    let (path, suffix) = dest.split_at(split);
    let resolved = if let Some(abs) = path.strip_prefix('/') {
        crate::model::join("", abs)
    } else {
        crate::model::join(old_dir, path)
    };
    let Some(resolved) = resolved else {
        return keep;
    };
    let target = if resolved == moved_from {
        moved_to.to_string()
    } else {
        resolved.clone()
    };
    if target == resolved && old_dir == new_dir {
        return keep;
    }
    if !(c.files.contains(&resolved) || c.dirs.contains(&resolved)) {
        return keep;
    }
    if path.starts_with('/') {
        return format!("/{target}{suffix}");
    }
    format!("{}{suffix}", relative(new_dir, &target))
}

/// The relative path from a directory to a repository path.
fn relative(from_dir: &str, to: &str) -> String {
    let a: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let b: Vec<&str> = to.split('/').collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<&str> = vec![".."; a.len() - common];
    parts.extend(&b[common..]);
    parts.join("/")
}

/// Whole lines covering the byte range.
fn line_span(text: &str, start: usize, end: usize) -> (usize, usize) {
    let a = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let end = end.max(start + 1).min(text.len());
    let b = if text[..end].ends_with('\n') {
        end
    } else {
        text[end..].find('\n').map_or(text.len(), |i| end + i + 1)
    };
    (a, b)
}

fn dedent(item: &str) -> String {
    let indent = item.len() - item.trim_start_matches(' ').len();
    let mut out: String = item
        .lines()
        .map(|l| format!("{}\n", l.get(indent..).unwrap_or(l.trim_start())))
        .collect();
    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

/// Adds an alias to the requirement's inline marker, on its first line.
pub fn add_req_alias(item: &str, alias: &str) -> String {
    if let Some(pos) = item.find("<!-- aliases:") {
        let close = pos + item[pos..].find("-->").unwrap_or(0);
        let list = item[pos + "<!-- aliases:".len()..close].trim();
        let mut all: Vec<&str> = list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !all.contains(&alias) {
            all.push(alias);
        }
        return format!(
            "{}<!-- aliases: {} -->{}",
            &item[..pos],
            all.join(", "),
            &item[close + 3..]
        );
    }
    let first_end = item.find('\n').unwrap_or(item.len());
    let (first, rest) = item.split_at(first_end);
    format!("{} <!-- aliases: {alias} -->{rest}", first.trim_end())
}

/// Adds aliases to the front matter, creating it if needed.
pub fn add_aliases(text: &str, add: &[String]) -> String {
    let Some((fm, body_at)) = frontmatter::split(text) else {
        return format!("---\n{}---\n{text}", alias_block(add));
    };
    let fm_start = text.len() - text[3..].len() + if text[3..].starts_with("\r\n") { 2 } else { 1 };
    let mut existing: Vec<String> = match frontmatter::parse(fm, 2)
        .ok()
        .and_then(|f| f.get("aliases").map(|v| v.0.clone()))
    {
        Some(Value::Str(s)) if !s.is_empty() => vec![s],
        Some(Value::List(l)) => l,
        _ => Vec::new(),
    };
    for a in add {
        if !existing.contains(a) {
            existing.push(a.clone());
        }
    }
    // Drop the old `aliases` entry (its key line and continuation lines).
    let mut kept = String::new();
    let mut skipping = false;
    let mut inserted = false;
    for line in fm.split_inclusive('\n') {
        let is_key = !line.starts_with([' ', '\t', '-']) && !line.trim().is_empty();
        if skipping && is_key {
            skipping = false;
        }
        if is_key && line.starts_with("aliases:") {
            skipping = true;
            kept.push_str(&alias_block(&existing));
            inserted = true;
            continue;
        }
        if !skipping {
            kept.push_str(line);
        }
    }
    if !inserted {
        if !kept.is_empty() && !kept.ends_with('\n') {
            kept.push('\n');
        }
        kept.push_str(&alias_block(&existing));
    }
    let fm_end = fm_start + fm.len();
    format!(
        "{}{}{}{}",
        &text[..fm_start],
        kept,
        &text[fm_end..body_at],
        &text[body_at..]
    )
}

fn alias_block(aliases: &[String]) -> String {
    let mut s = String::from("aliases:\n");
    for a in aliases {
        let quoted = a.contains(": ")
            || a.contains(" #")
            || a.starts_with(['"', '\'', '[', '{', '&', '*', '!', '-', '#']);
        if quoted {
            s.push_str(&format!(
                "  - \"{}\"\n",
                a.replace('\\', "\\\\").replace('"', "\\\"")
            ));
        } else {
            s.push_str(&format!("  - {a}\n"));
        }
    }
    s
}

fn read(repo: &Repo, path: &str) -> Result<String, Fatal> {
    std::fs::read_to_string(repo.root.join(path)).map_err(|e| Fatal::Io(format!("{path}: {e}")))
}

pub fn write(path: &Path, text: &str) -> Result<(), Fatal> {
    let tmp = path.with_extension("spec-dna.tmp");
    std::fs::write(&tmp, text)
        .and_then(|_| std::fs::rename(&tmp, path))
        .map_err(|e| Fatal::Io(format!("{}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_in_front_matter() {
        assert_eq!(
            add_aliases("# T\n", &["a/b".into()]),
            "---\naliases:\n  - a/b\n---\n# T\n"
        );
        assert_eq!(
            add_aliases(
                "---\nstatus: active\naliases: [x]\nowner: me\n---\n# T\n",
                &["a/b".into()]
            ),
            "---\nstatus: active\naliases:\n  - x\n  - a/b\nowner: me\n---\n# T\n"
        );
        assert_eq!(
            add_aliases(
                "---\naliases:\n  - x\n---\n# T\n",
                &["x".into(), "ADR-1".into()]
            ),
            "---\naliases:\n  - x\n  - ADR-1\n---\n# T\n"
        );
        assert_eq!(
            add_aliases("---\nstatus: draft\n---\n", &["a".into()]),
            "---\nstatus: draft\naliases:\n  - a\n---\n"
        );
    }

    #[test]
    fn requirement_alias_marker() {
        assert_eq!(
            add_req_alias("- **k**: The x SHALL y.\n", "a/b#k"),
            "- **k**: The x SHALL y. <!-- aliases: a/b#k -->\n"
        );
        assert_eq!(
            add_req_alias(
                "- **k**: The x SHALL y. <!-- aliases: c#k -->\n  more\n",
                "a/b#k"
            ),
            "- **k**: The x SHALL y. <!-- aliases: c#k, a/b#k -->\n  more\n"
        );
    }

    #[test]
    fn relative_paths() {
        assert_eq!(relative("docs/auth", "docs/auth/x.md"), "x.md");
        assert_eq!(
            relative("docs/auth", "docs/billing/x.md"),
            "../billing/x.md"
        );
        assert_eq!(relative("", "src/a.rs"), "src/a.rs");
        assert_eq!(relative("docs/a/b", "README.md"), "../../../README.md");
    }

    #[test]
    fn spans_and_dedent() {
        let t = "a\n  - **k**: x\n    - y\nb\n";
        let s = t.find("- **k**").unwrap();
        let e = t.find("b\n").unwrap();
        assert_eq!(line_span(t, s, e - 1), (2, e));
        assert_eq!(dedent("  - **k**: x\n    - y\n"), "- **k**: x\n  - y\n");
    }
}
