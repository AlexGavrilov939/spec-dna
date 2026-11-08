//! Generated agent adapters (ADR 0003): an `AGENTS.md` block, a `CLAUDE.md`
//! import line and one skill. Each adapter contains the generator version and
//! a hash of its normalized text. Thus `check` can report a manual edit or an
//! old version.

use crate::diag::{Code, Diag};
use crate::hash::{hash_normalized, hex};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AGENTS: &str = "AGENTS.md";
pub const CLAUDE: &str = "CLAUDE.md";
pub const SKILL: &str = ".claude/skills/spec-dna/SKILL.md";
pub const PATHS: [&str; 3] = [AGENTS, CLAUDE, SKILL];

const AGENTS_BODY: &str = "## Specifications

This repository uses spec-dna for its specifications. `spec-dna.toml` lists the document roots.

- Before you change behavior that a spec, contract, guide or decision covers, run `spec-dna context <id or path>`. Read what it prints. If it lists a section as not loaded, run `spec-dna show <id>#<anchor>` to read that section.
- Cite a requirement as `<doc-id>#<key>`. Use `spec-dna mv` to move a document or a requirement, so that its old id still resolves.
- Run `spec-dna check` before you commit. Fix every error that it reports.";

const CLAUDE_BODY: &str = "@AGENTS.md";

const SKILL_FRONT: &str = "---
name: spec-dna
description: Uses the spec-dna CLI to load the specifications that apply to a task and to check changes to them. Use it before you change behavior that a spec, contract, guide or decision covers. Use it when you cite or move a requirement. Use it before you commit a change to a document under a spec-dna root.
---
";

const SKILL_BODY: &str = "# spec-dna

1. Find the entry. The entry is a document id (`<namespace>/<slug>`), a legacy alias such as `ADR-12`, or a file that you will change.
2. Run `spec-dna context <id> --path <file>`. It prints the documents in this sequence:
    - the entry document
    - the documents whose `applies-to` matches
    - the documents that the entry depends on
    - the linked accepted decisions, as summaries

    The output shows the id and the hash of each section. If the entry is a long document without requirements, add `--query \"<the task in plain words>\"`. This puts the sections that match the task first.
3. If the output lists a section as not loaded, run `spec-dna show <id>#<anchor>` to read that section.
4. Cite requirements as `<doc-id>#<key>`. Write a new requirement as `- **<key>**: <sentence>`. Use a lowercase slug as the key. In the sentence, use an EARS pattern with an uppercase BCP 14 keyword.
5. After you edit documents, run `spec-dna check`. Fix every error. To move a document, run `spec-dna mv <old> <new>`. To move a requirement, run `spec-dna mv <doc-id>#<key> <doc-id>`.";

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

/// Finds the generated block in a file. Returns (start, end, version, recorded hash, body).
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

/// Checks the adapters that are in a tree. `files` contains (path, text).
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
            severity: code.severity(),
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

/// Returns true if `CLAUDE.md` already imports `AGENTS.md` outside of a generated block.
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
