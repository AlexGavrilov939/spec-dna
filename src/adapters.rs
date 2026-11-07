//! Generated agent adapters (ADR 0003): an `AGENTS.md` block, a `CLAUDE.md`
//! import line and one skill. Each carries the generator version and a hash
//! of its normalized text, so `check` can report a hand edit or an old
//! version.

use crate::diag::{Code, Diag};
use crate::hash::{hash_normalized, hex};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AGENTS: &str = "AGENTS.md";
pub const CLAUDE: &str = "CLAUDE.md";
pub const SKILL: &str = ".claude/skills/spec-dna/SKILL.md";
pub const PATHS: [&str; 3] = [AGENTS, CLAUDE, SKILL];

const AGENTS_BODY: &str = "## Specifications

This repository keeps its specifications with spec-dna. The document roots are listed in `spec-dna.toml`.

- Before you change behavior that a spec, contract, guide or decision covers, run `spec-dna context <id or path>` and read what it prints. Fetch a section it lists as not loaded with `spec-dna show <id>#<anchor>`.
- Cite a requirement as `<doc-id>#<key>`. Move a document or a requirement with `spec-dna mv`, so its old id keeps resolving.
- Run `spec-dna check` before you commit, and fix every error it reports.";

const CLAUDE_BODY: &str = "@AGENTS.md";

const SKILL_FRONT: &str = "---
name: spec-dna
description: Loads the specifications that apply to a task and checks edits to them with the spec-dna CLI. Use before changing behavior that a spec, contract, guide or decision covers, when citing or moving a requirement, and before committing a change to a document under a spec-dna root.
---
";

const SKILL_BODY: &str = "# spec-dna

1. Find the entry: a document id (`<namespace>/<slug>`), a legacy alias such as `ADR-12`, or a file you will change.
2. Run `spec-dna context <id> --path <file>`. It prints the entry document first, then the documents whose `applies-to` matches, then the documents the entry depends on, then linked accepted decisions as summaries. Every section carries its id and hash.
3. A section listed as not loaded is one call away: `spec-dna show <id>#<anchor>`.
4. Cite requirements as `<doc-id>#<key>`. Write a new requirement as `- **<key>**: <sentence>`, where the key is a lowercase slug and the sentence uses an EARS pattern with an uppercase BCP 14 keyword.
5. After editing documents, run `spec-dna check` and fix every error. Move a document with `spec-dna mv <old> <new>`, and a requirement with `spec-dna mv <doc-id>#<key> <doc-id>`.";

fn body(name: &str) -> &'static str {
    match name {
        "agents" => AGENTS_BODY,
        "claude" => CLAUDE_BODY,
        _ => SKILL_BODY,
    }
}

fn name_of(path: &str) -> &'static str {
    match path {
        AGENTS => "agents",
        CLAUDE => "claude",
        _ => "skill",
    }
}

pub fn block(name: &str) -> String {
    let b = body(name);
    format!(
        "<!-- spec-dna:begin {name} v{VERSION} h:{} -->\n{b}\n<!-- spec-dna:end {name} -->\n",
        hex(&hash_normalized(b))
    )
}

pub fn skill_file() -> String {
    format!("{SKILL_FRONT}\n{}", block("skill"))
}

/// The generated block in a file: (start, end, version, recorded hash, body).
fn find_block<'a>(text: &'a str, name: &str) -> Option<(usize, usize, &'a str, &'a str, &'a str)> {
    let begin = format!("<!-- spec-dna:begin {name} ");
    let start = text.find(&begin)?;
    let header_end = start + text[start..].find("-->")? + 3;
    let header = &text[start + begin.len()..header_end - 3];
    let mut it = header.split_whitespace();
    let version = it.next()?.strip_prefix('v')?;
    let hash = it.next()?.strip_prefix("h:")?;
    let end_marker = format!("<!-- spec-dna:end {name} -->");
    let body_start = header_end + text[header_end..].find('\n').map_or(0, |i| i + 1);
    let end_rel = text[body_start..].find(&end_marker)?;
    let body = &text[body_start..body_start + end_rel];
    let mut end = body_start + end_rel + end_marker.len();
    if text[end..].starts_with('\n') {
        end += 1;
    }
    Some((start, end, version, hash, body))
}

/// Checks the adapters present in a tree. `files` holds (path, text).
pub fn check(files: &[(&str, String)]) -> Vec<Diag> {
    let mut out = Vec::new();
    for (path, text) in files {
        let name = name_of(path);
        let Some((start, end, version, hash, b)) = find_block(text, name) else {
            continue;
        };
        let d = |code: Code, message: String| Diag {
            path: path.to_string(),
            line: text[..start].matches('\n').count() as u32 + 1,
            code,
            doc: path.to_string(),
            target: name.to_string(),
            message,
        };
        if hex(&hash_normalized(b)) != hash {
            out.push(d(
                Code::AdapterModified,
                format!("the generated `{name}` block was edited by hand"),
            ));
            continue;
        }
        let current = if *path == SKILL {
            text.as_str() == skill_file()
        } else {
            text[start..end] == block(name)
        };
        if !current {
            out.push(d(Code::AdapterOutdated, format!("the `{name}` adapter (v{version}) differs from what spec-dna {VERSION} generates")));
        }
    }
    out
}

/// Inserts or replaces the block in a file's text.
pub fn upsert(text: &str, name: &str) -> String {
    let new = block(name);
    if let Some((start, end, ..)) = find_block(text, name) {
        return format!("{}{}{}", &text[..start], new, &text[end..]);
    }
    if text.is_empty() {
        return new;
    }
    let sep = if text.ends_with("\n\n") {
        ""
    } else if text.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    format!("{text}{sep}{new}")
}

/// True if `CLAUDE.md` already imports `AGENTS.md` outside a generated block.
pub fn imports_agents(text: &str) -> bool {
    let stripped = match find_block(text, "claude") {
        Some((s, e, ..)) => format!("{}{}", &text[..s], &text[e..]),
        None => text.to_string(),
    };
    stripped.lines().any(|l| l.trim() == "@AGENTS.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_adapters_pass_and_edits_fail() {
        let agents = upsert("# Rules\n\nOwn text.\n", "agents");
        assert!(agents.starts_with("# Rules\n\nOwn text.\n\n<!-- spec-dna:begin agents v"));
        assert_eq!(
            upsert(&agents, "agents"),
            agents,
            "regenerating is idempotent"
        );
        let claude = upsert("", "claude");
        let files = vec![
            (AGENTS, agents.clone()),
            (CLAUDE, claude),
            (SKILL, skill_file()),
        ];
        assert!(check(&files).is_empty());

        let edited = agents.replace("before you commit", "before you push");
        let d = check(&[(AGENTS, edited)]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, Code::AdapterModified);

        let old = agents.replace(&format!("v{VERSION} "), "v0.0.1 ");
        let d = check(&[(AGENTS, old)]);
        assert_eq!(d[0].code, Code::AdapterOutdated);
    }

    #[test]
    fn detects_an_existing_import() {
        assert!(imports_agents("# X\n@AGENTS.md\n"));
        assert!(!imports_agents(&upsert("# X\n", "claude")));
    }
}
