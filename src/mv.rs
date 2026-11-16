//! `spec-dna mv` moves a document, or requirements and sections (`split.rs`).
//! It records the old id as an alias, so that the references to the old id
//! still resolve (ADR 0001, "Identifiers" and "References and requirements").

use crate::config::Config;
use crate::frontmatter::{self, Value};
use crate::model::{Corpus, is_doc_path};
use crate::repo::{Fatal, Repo};
use std::collections::HashMap;
use std::path::Path;

pub struct Moved {
    pub kind: &'static str,
    pub from: String,
    pub to: String,
    pub alias: String,
    pub files: Vec<String>,
    /// For a move of requirements or sections: the details of the split.
    pub split: Option<crate::split::Split>,
    /// The path of the stub that `--stub` left at the old path.
    pub stub: Option<String>,
    /// For a move of a whole document: the alias that the move dropped,
    /// because it is the new id.
    pub alias_dropped: Option<String>,
    /// For a move of a whole document: the changes of the `type` and `status`
    /// lines that the new place needs, one text line for each.
    pub changes: Vec<String>,
}

/// Moves one document, or requirements and sections (`split.rs`). The last
/// argument is the destination.
pub fn run(
    repo: &Repo,
    config: &Config,
    c: &Corpus,
    base: Option<&Corpus>,
    args: &[String],
    opts: &crate::split::Opts,
    stub: bool,
) -> Result<Moved, Fatal> {
    let (sources, to) = args.split_at(args.len() - 1);
    let to = &to[0];
    if let [from] = sources
        && !from.contains('#')
    {
        let (d, _) = crate::context::resolve_target(c, from)
            .ok_or_else(|| Fatal::NotFound(format!("`{from}` names no document")))?;
        return move_document(repo, config, c, d, to, stub);
    }
    if stub {
        return Err(Fatal::Usage(
            "--stub is for the move of a whole document; a split keeps the source file".into(),
        ));
    }
    let s = crate::split::run(repo, config, c, base, sources, to, opts)?;
    let one = s.moved.len() == 1 && s.sections == 0;
    let (kind, from, to) = if one {
        let key = s.moved[0].rsplit('#').next().unwrap_or("");
        ("requirement", s.moved[0].clone(), format!("{}#{key}", s.to))
    } else {
        ("split", sources.join(", "), s.to.clone())
    };
    Ok(Moved {
        kind,
        from,
        to,
        alias: s.aliases.join(", "),
        files: s.files.clone(),
        split: Some(s),
        stub: None,
        alias_dropped: None,
        changes: Vec::new(),
    })
}

fn move_document(
    repo: &Repo,
    config: &Config,
    c: &Corpus,
    d: u32,
    to: &str,
    stub: bool,
) -> Result<Moved, Fatal> {
    let doc = &c.docs[d as usize];
    if doc.moved_to.is_some() {
        return Err(Fatal::Usage(format!(
            "`{}` is a stub; move the document that it points to",
            doc.path
        )));
    }
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
    // The new id can be a front matter alias of the moved document itself.
    // The move then drops that alias. A file at the path, the id of another
    // document, a legacy alias and an ambiguous alias stop the move.
    let canon_new = crate::model::canonical(&new_id);
    let file_exists = repo.root.join(&to_path).exists();
    let own_alias = c
        .lookup
        .get(&canon_new)
        .filter(|hits| hits.as_slice() == [d])
        .and_then(|_| {
            doc.aliases
                .iter()
                .find(|a| !a.contains('#') && crate::model::canonical(a) == canon_new)
                .cloned()
        })
        .filter(|_| !file_exists);
    if file_exists || (c.lookup.contains_key(&canon_new) && own_alias.is_none()) {
        return Err(Fatal::Usage(format!(
            "`{to_path}` (id `{new_id}`) already exists"
        )));
    }
    // Keep the old id, and the legacy aliases that the new file name would lose.
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
    // Everything that can stop the move runs before the first change.
    let (text, changes) = fit_front_matter(config, doc, new_root, &to_path, text)?;
    let old_dir = parent(&doc.path);
    let one: HashMap<String, String> = HashMap::from([(doc.path.clone(), to_path.clone())]);
    let updated = rewrite_links(
        &edit_aliases(&text, &add, own_alias.as_deref()),
        old_dir,
        parent(&to_path),
        &one,
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
            let nt = rewrite_links(&t, dir, dir, &one, c);
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
    let mut files = vec![doc.path.clone(), to_path.clone()];
    for (p, t) in others {
        write(&repo.root.join(&p), &t)?;
        files.push(p);
    }
    // A stub at the old path, for tools that read a file by its path.
    let stub_path = if stub {
        let title = doc.parsed.title.replace(['[', ']'], "");
        let text = format!(
            "---\nmoved-to: {new_id}\n---\n# {title}\n\nThis document moved to [{title}]({}).\n",
            relative(old_dir, &to_path)
        );
        let at = repo.root.join(&doc.path);
        if let Some(dir) = at.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Fatal::Io(e.to_string()))?;
        }
        write(&at, &text)?;
        Some(doc.path.clone())
    } else {
        None
    };
    // The folders that the move left empty go. The stub keeps its folder.
    if !stub {
        remove_empty_parents(repo, &root.path, &doc.path);
    }
    Ok(Moved {
        kind: "document",
        from: doc.id.clone(),
        to: new_id,
        alias: add.join(", "),
        files,
        split: None,
        stub: stub_path,
        alias_dropped: own_alias,
        changes,
    })
}

/// Removes each empty parent folder of an old path, from the deepest folder
/// to the highest. A folder with any file stays, an ignored file too. It
/// stops at the root folder of the document, which never goes.
pub(crate) fn remove_empty_parents(repo: &Repo, root_path: &str, old_path: &str) {
    let mut dir = parent(old_path);
    while !dir.is_empty()
        && dir != root_path
        && (root_path.is_empty() || dir.starts_with(&format!("{root_path}/")))
    {
        if std::fs::remove_dir(repo.root.join(dir)).is_err() {
            break;
        }
        dir = parent(dir);
    }
}

/// Fits the `type` and `status` lines of a document to its new place. It
/// applies to a mixed root only: a root of one type gives the type, and
/// nothing changes. Returns the new text and one text line for each change.
fn fit_front_matter(
    config: &Config,
    doc: &crate::model::Doc,
    new_root: usize,
    to_path: &str,
    mut text: String,
) -> Result<(String, Vec<String>), Fatal> {
    use crate::model::DocType;
    let root = &config.roots[new_root];
    if root.single_type.is_some() {
        return Ok((text, Vec::new()));
    }
    let rel = root.rel(to_path);
    // The type of the place, as `make_doc` reads it.
    let place = config
        .layers
        .place_type(rel)
        .filter(|&t| !t.is_intent_type() || config.layers.is_intent_place(t, rel));
    let raw = |text: &str, key: &str| -> Option<String> {
        let (fm, _) = frontmatter::split(text)?;
        match frontmatter::parse(fm, 2).ok()?.get(key)?.0 {
            Value::Str(s) => Some(s.clone()),
            _ => None,
        }
    };
    let has_line = |text: &str, key: &str| frontmatter::remove_field(text, key).is_some();
    let hand = |key: &str| {
        Fatal::Usage(format!(
            "cannot edit the `{key}` line of `{}`; edit it by hand and run `mv` again",
            doc.path
        ))
    };
    let mut changes = Vec::new();
    let Some(t) = place else {
        // The place gives no type. The old type came from the old place
        // unless the front matter or `default-type` of the new root has it.
        if !doc.type_failed && !has_line(&text, "type") && Some(doc.ty) != root.default_type {
            text = frontmatter::insert_field(&text, "type", doc.ty.name(), None);
            changes.push(format!(
                "type: {} written (the new place gives no type)",
                doc.ty.name()
            ));
        }
        return Ok((text, changes));
    };
    if !doc.reqs.is_empty() && !t.has_requirements() {
        return Err(Fatal::Usage(format!(
            "`{to_path}` gives the type `{}`, which holds no requirements; move the {} requirements first with `spec-dna mv {}#<key>... <to>`",
            t.name(),
            doc.reqs.len(),
            doc.id
        )));
    }
    if doc.ty == DocType::Decision
        && t != DocType::Decision
        && doc.status.as_deref() == Some("accepted")
    {
        return Err(Fatal::Usage(format!(
            "`{}` is an accepted decision, and `{to_path}` gives the type `{}`; a decision keeps its locked text, so it keeps its type",
            doc.path,
            t.name()
        )));
    }
    if let Some(s) = frontmatter::remove_field(&text, "type") {
        text = s;
    }
    if t != doc.ty {
        changes.push(format!(
            "type: {} -> {} (the place gives the type)",
            doc.ty.name(),
            t.name()
        ));
    }
    let line = raw(&text, "status");
    if line.is_none() && has_line(&text, "status") && t.has_status() {
        // A `status` line with a list or another form that `mv` cannot read.
        return Err(hand("status"));
    }
    if !t.has_status() {
        if let Some(old) = line {
            text = frontmatter::remove_field(&text, "status").ok_or_else(|| hand("status"))?;
            changes.push(format!(
                "status: {old} removed (the type {} has no status)",
                t.name()
            ));
        }
        return Ok((text, changes));
    }
    // The status that the document has now, with the `status-map` of the new
    // root. With no line, it is the status that the old place gave.
    let fits = |s: &str| {
        t.statuses().contains(&s)
            || root
                .status_map
                .iter()
                .any(|(from, to)| from == s && t.statuses().contains(&to.as_str()))
    };
    let current = line.clone().or_else(|| doc.status.clone());
    match current {
        Some(s) if fits(&s) => {
            // A status that only the old root gave needs a line in the new root.
            let given = Some(t) == root.default_type && root.status.as_deref() == Some(s.as_str());
            if line.is_none() && !given {
                text = frontmatter::insert_field(&text, "status", &s, Some("type"));
            }
        }
        other => {
            let first = crate::templates::first_status(t).unwrap_or("draft");
            let given = Some(t) == root.default_type && other.is_none() && root.status.is_some();
            if !given {
                text = match line {
                    Some(_) => frontmatter::set_field(&text, "status", first)
                        .ok_or_else(|| hand("status"))?,
                    None => frontmatter::insert_field(&text, "status", first, Some("type")),
                };
                changes.push(format!(
                    "status: {} -> {first}",
                    other.as_deref().unwrap_or("none")
                ));
            }
        }
    }
    Ok((text, changes))
}

pub(crate) fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(d, _)| d)
}

/// Rewrites relative Markdown link destinations of a document that moves
/// from `old_dir` to `new_dir`, or that links to a moved file. `moves` maps
/// the old path of each moved file, and each moved folder that a link can
/// name, to its new path. One pass reads the tree before any move (`c`), so a
/// batch of moves needs one call for each document. It does not change links
/// that do not resolve, external links or code blocks.
pub(crate) fn rewrite_links(
    text: &str,
    old_dir: &str,
    new_dir: &str,
    moves: &HashMap<String, String>,
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
            out.push_str(&new_dest(&line[lead..dest_end], old_dir, new_dir, moves, c));
            rest = &line[dest_end..];
        }
        while let Some(i) = rest.find("](") {
            out.push_str(&rest[..i + 2]);
            rest = &rest[i + 2..];
            let end = rest
                .find(|ch: char| ch == ')' || ch.is_whitespace())
                .unwrap_or(rest.len());
            out.push_str(&new_dest(&rest[..end], old_dir, new_dir, moves, c));
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
    moves: &HashMap<String, String>,
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
    let target = moves.get(&resolved).unwrap_or(&resolved).clone();
    if target == resolved && old_dir == new_dir {
        return keep;
    }
    if !(c.files.contains(&resolved) || c.dirs.contains(&resolved)) {
        return keep;
    }
    // A link to a folder keeps its final `/`.
    let slash = if path.ends_with('/') && !target.is_empty() {
        "/"
    } else {
        ""
    };
    if path.starts_with('/') {
        return format!("/{target}{slash}{suffix}");
    }
    format!("{}{slash}{suffix}", relative(new_dir, &target))
}

/// Returns the relative path from a directory to a repository path.
pub(crate) fn relative(from_dir: &str, to: &str) -> String {
    let a: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let b: Vec<&str> = to.split('/').collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut parts: Vec<&str> = vec![".."; a.len() - common];
    parts.extend(&b[common..]);
    parts.join("/")
}

/// Returns the whole lines that cover the byte range.
pub(crate) fn line_span(text: &str, start: usize, end: usize) -> (usize, usize) {
    let a = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let end = end.max(start + 1).min(text.len());
    let b = if text[..end].ends_with('\n') {
        end
    } else {
        text[end..].find('\n').map_or(text.len(), |i| end + i + 1)
    };
    (a, b)
}

pub(crate) fn dedent(item: &str) -> String {
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

/// Adds aliases to the front matter. If the front matter does not exist, it
/// creates it.
pub fn add_aliases(text: &str, add: &[String]) -> String {
    edit_aliases(text, add, None)
}

/// Adds aliases to the front matter, and drops the alias `drop` from it. It
/// compares the names in their canonical form, as the lookup does.
pub fn edit_aliases(text: &str, add: &[String], drop: Option<&str>) -> String {
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
    if let Some(d) = drop {
        let d = crate::model::canonical(d);
        existing.retain(|a| crate::model::canonical(a) != d);
    }
    for a in add {
        if !existing.contains(a) {
            existing.push(a.clone());
        }
    }
    // Remove the old `aliases` entry (its key line and its continuation lines).
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

pub(crate) fn read(repo: &Repo, path: &str) -> Result<String, Fatal> {
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
    fn an_alias_that_is_the_new_id_drops_out() {
        assert_eq!(
            edit_aliases(
                "---\naliases: [a/b, ADR-007]\n---\n# T\n",
                &["x/y".into()],
                Some("a/b")
            ),
            "---\naliases:\n  - ADR-007\n  - x/y\n---\n# T\n"
        );
        assert_eq!(
            edit_aliases(
                "---\naliases:\n  - ADR-007\n---\n# T\n",
                &["x/y".into()],
                Some("ADR-7")
            ),
            "---\naliases:\n  - x/y\n---\n# T\n"
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
        assert_eq!(relative("docs/irrigation", "docs/irrigation/x.md"), "x.md");
        assert_eq!(
            relative("docs/irrigation", "docs/harvest/x.md"),
            "../harvest/x.md"
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

    fn corpus(files: &[(&str, &str)]) -> Corpus {
        let cfg = Config::parse(&format!(
            "spec-dna = \"{}\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"research\"\n",
            crate::adapters::VERSION
        ))
        .unwrap();
        let docs = files
            .iter()
            .filter(|(p, _)| is_doc_path(p))
            .map(|(p, t)| (p.to_string(), std::sync::Arc::new(crate::parse::parse(t))))
            .collect();
        Corpus::build(
            &cfg,
            files.iter().map(|(p, _)| p.to_string()).collect(),
            docs,
        )
    }

    #[test]
    fn a_batch_rewrite_follows_each_move_in_one_pass() {
        let c = corpus(&[
            ("docs/a/x.md", "# X\n"),
            ("docs/a/y.md", "# Y\n"),
            ("docs/a/img/p.png", ""),
            ("docs/b/z.md", "# Z\n"),
        ]);
        let moves: std::collections::HashMap<String, String> = [
            ("docs/a/x.md", "docs/c/x.md"),
            ("docs/a/y.md", "docs/c/y.md"),
            ("docs/a/img", "docs/c/img"),
            ("docs/a/img/p.png", "docs/c/img/p.png"),
        ]
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
        // A document that does not move: only the moved targets change.
        let text = "[x](a/x.md#k) and [y](a/y.md), [z](b/z.md), [dir](a/img/), [self](#top)\n";
        assert_eq!(
            rewrite_links(text, "docs", "docs", &moves, &c),
            "[x](c/x.md#k) and [y](c/y.md), [z](b/z.md), [dir](c/img/), [self](#top)\n"
        );
        // A document that moves with them: the links to the other moved files
        // stay short, and the links to files that stay get a new path.
        let text = "[y](y.md), [z](../b/z.md), [p](img/p.png)\n";
        assert_eq!(
            rewrite_links(text, "docs/a", "docs/c", &moves, &c),
            "[y](y.md), [z](../b/z.md), [p](img/p.png)\n"
        );
        let text = "[z](../b/z.md)\n";
        assert_eq!(
            rewrite_links(text, "docs/a", "docs/d/e", &moves, &c),
            "[z](../../b/z.md)\n"
        );
    }
}
