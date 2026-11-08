//! Generated agent adapters (ADR 0003, amendment of 2026-11-08): one line in
//! each root instruction file (`AGENTS.md`, `CLAUDE.md`) and one skill. Each
//! adapter contains the generator version and a hash of its normalized text.
//! Thus `check` can report a manual edit or an old version. The line never
//! imports another file, so it adds only its own bytes to the prefix of each
//! agent request.

use crate::diag::{Code, Diag};
use crate::hash::{hash_normalized, hex};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AGENTS: &str = "AGENTS.md";
pub const CLAUDE: &str = "CLAUDE.md";
pub const SKILL: &str = ".claude/skills/spec-dna/SKILL.md";
pub const PATHS: [&str; 3] = [AGENTS, CLAUDE, SKILL];

const LINE_BODY: &str = "If a task edits code or docs, run `spec-dna context <files>` once first. Before a commit, run `spec-dna check`.";

/// The skill description. An agent loads the description of each skill into
/// every request, so it must stay short.
const SKILL_DESCRIPTION: &str = "Loads and checks the specs for a task. Use it before you change code or docs, and before a commit.";

/// The skill body. The agent reads it once per session, and every later
/// request carries it, so each line must change what the agent does.
const SKILL_BODY: &str = "# spec-dna

1. If the task changes code or docs, run `spec-dna context <file>...` once, with all the files that you plan to change (or a document id). Skip it for a question without edits. The output gives the sections in priority order: invariants, the entry, the sections that cite the files, their decisions. Each section has an id.
2. Read a listed section with `spec-dna show <id>#<anchor>` only if the task needs it.
3. Keep the documents true in the same change. Write a requirement as `- **<key>**: <sentence>`, with a lowercase slug key, an EARS pattern and an uppercase BCP 14 keyword. Cite it as `<doc-id>#<key>`. In a test, `covers: <doc-id>#<key>` links the test to it.
4. For a new document, run `spec-dna new <type> <id>` and replace each placeholder. Types: spec, contract, guide, decision, change, note, invariant, runbook, incident, reference.
5. Do not change the text of an accepted decision outside its amendment sections (default `## Amendments`).
    - Amend the decision if its outcome stays: add the detail, the limit, the fix after an incident or the new code path under `## Amendments` of the same file. A task that says \"amend the decision\" means this.
    - Supersede the decision if its outcome changes: write a new decision with `supersedes: [<old id>]`, and set the status of the old one to `superseded`.
6. Before you commit, run `spec-dna check` once. Fix each error, and each SD606 or SD607 warning (a document cites code that your change removed). For an accepted decision, name the new path or identifier in its amendment section. The output gives the fix of each code. Run `check` again only after a fix.
7. Move a document or requirements only with `spec-dna mv <from>... <to>`, so that the old ids stay as aliases. When `check` reports SD702, split the document: move several keys or sections into a new spec with one `mv`.";

/// The generated line of a root instruction file.
pub fn line() -> String {
    format!(
        "{LINE_BODY} <!-- spec-dna v{VERSION} h:{} -->",
        hex(&hash_normalized(LINE_BODY))
    )
}

/// The skill block, between begin and end markers.
pub fn block(name: &str) -> String {
    format!(
        "<!-- spec-dna:begin {name} v{VERSION} h:{} -->\n{SKILL_BODY}\n<!-- spec-dna:end {name} -->\n",
        hex(&hash_normalized(SKILL_BODY))
    )
}

pub fn skill_file() -> String {
    format!(
        "---\nname: spec-dna\ndescription: {SKILL_DESCRIPTION}\n---\n\n{}",
        block("skill")
    )
}

/// The bytes that the skill adds to each request of an agent that lists its
/// skills as `- <name>: <description>` lines.
pub fn skill_prefix_bytes() -> usize {
    format!("- spec-dna: {SKILL_DESCRIPTION}\n").len()
}

/// Finds a generated block (the skill, or the multi-line block of an older
/// version). Returns (start, end, version, recorded hash, body).
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

/// Finds the generated line: a line that ends with the exact marker
/// `<!-- spec-dna v<version> h:<16 hex digits> -->`. Prose that quotes the
/// marker form does not match. Returns (start, end with the line break,
/// version, recorded hash, text before the marker).
fn find_line(text: &str) -> Option<(usize, usize, &str, &str, &str)> {
    const OPEN: &str = "<!-- spec-dna v";
    let mut start = 0;
    for l in text.split_inclusive('\n') {
        let end = start + l.len();
        let body = l.trim_end_matches(['\n', '\r']);
        if let Some(m) = body.rfind(OPEN)
            && let Some(inner) = body[m + OPEN.len()..].strip_suffix(" -->")
            && let Some((version, hash)) = inner.split_once(" h:")
            && !version.is_empty()
            && version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
            && hash.len() == 16
            && hash.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Some((start, end, version, hash, body[..m].trim_end()));
        }
        start = end;
    }
    None
}

/// Removes the generated line and the blank line before it.
pub fn remove_line(text: &str) -> String {
    match find_line(text) {
        Some((start, end, ..)) => {
            let start = if text[..start].ends_with("\n\n") {
                start - 1
            } else {
                start
            };
            format!("{}{}", &text[..start], &text[end..])
        }
        None => text.to_string(),
    }
}

/// Returns true if the text has content that spec-dna did not generate.
pub fn has_own_text(text: &str) -> bool {
    !remove_line(&remove_old_blocks(text)).trim().is_empty()
}

/// Replaces the `claude` block of an older version, which imported
/// `AGENTS.md`. If `keep_import` is true (`AGENTS.md` has text of its own),
/// the import stays as a plain line, so that the agent still reads that text.
/// Otherwise the block goes.
pub fn migrate_claude(text: &str, keep_import: bool) -> String {
    match find_block(text, "claude") {
        Some((s, e, ..)) if keep_import => format!("{}@AGENTS.md\n{}", &text[..s], &text[e..]),
        Some(_) => remove_old_blocks(text),
        None => text.to_string(),
    }
}

/// Removes the blocks of older versions: a multi-line `agents` block and the
/// `claude` block with an `@AGENTS.md` import.
fn remove_old_blocks(text: &str) -> String {
    let mut t = text.to_string();
    for name in ["agents", "claude"] {
        if let Some((s, e, ..)) = find_block(&t, name) {
            let s = if t[..s].ends_with("\n\n") { s - 1 } else { s };
            t = format!("{}{}", &t[..s], &t[e..]);
        }
    }
    t
}

/// Checks the adapters that are in a tree. `files` contains (path, text).
pub fn check(files: &[(&str, String)]) -> Vec<Diag> {
    let mut out = Vec::new();
    for (path, text) in files {
        let at = |pos: usize| text[..pos].matches('\n').count() as u32 + 1;
        let d = |code: Code, line: u32, target: &str, message: String| Diag {
            path: path.to_string(),
            line,
            code,
            doc: path.to_string(),
            target: target.to_string(),
            message,
            severity: code.severity(),
        };
        if *path == SKILL {
            let Some((start, _, version, hash, b)) = find_block(text, "skill") else {
                continue;
            };
            if hex(&hash_normalized(b)) != hash {
                out.push(d(
                    Code::AdapterModified,
                    at(start),
                    "skill",
                    "the generated `skill` block was edited by hand".into(),
                ));
            } else if text.as_str() != skill_file() {
                out.push(d(Code::AdapterOutdated, at(start), "skill", format!("the `skill` adapter (v{version}) differs from what spec-dna {VERSION} generates")));
            }
            continue;
        }
        for name in ["agents", "claude"] {
            if let Some((start, _, version, ..)) = find_block(text, name) {
                out.push(d(Code::AdapterOutdated, at(start), name, format!("the `{name}` block of spec-dna {version} is replaced by one line in spec-dna {VERSION}")));
            }
        }
        let Some((start, end, version, hash, b)) = find_line(text) else {
            continue;
        };
        if hex(&hash_normalized(b)) != hash {
            out.push(d(
                Code::AdapterModified,
                at(start),
                "line",
                "the generated spec-dna line was edited by hand".into(),
            ));
        } else if text[start..end].trim_end() != line() {
            out.push(d(
                Code::AdapterOutdated,
                at(start),
                "line",
                format!(
                    "the spec-dna line (v{version}) differs from what spec-dna {VERSION} generates"
                ),
            ));
        }
    }
    out
}

/// Puts the generated line into the text of a root instruction file. It
/// replaces an existing line or an old block. Otherwise, it adds the line at
/// the end, after a blank line.
pub fn upsert_line(text: &str) -> String {
    let t = remove_old_blocks(text);
    let new = line();
    if let Some((start, end, ..)) = find_line(&t) {
        return format!("{}{new}\n{}", &t[..start], &t[end..]);
    }
    if t.trim().is_empty() {
        return format!("{new}\n");
    }
    let sep = if t.ends_with("\n\n") {
        ""
    } else if t.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    format!("{t}{sep}{new}\n")
}

/// Returns true if `CLAUDE.md` imports `AGENTS.md` outside of a generated
/// block.
pub fn imports_agents(text: &str) -> bool {
    remove_old_blocks(text)
        .lines()
        .any(|l| l.trim() == "@AGENTS.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_line_passes_and_edits_fail() {
        let agents = upsert_line("# Rules\n\nOwn text.\n");
        assert_eq!(agents, format!("# Rules\n\nOwn text.\n\n{}\n", line()));
        assert_eq!(upsert_line(&agents), agents, "regenerating is idempotent");
        let files = vec![
            (AGENTS, agents.clone()),
            (CLAUDE, upsert_line("")),
            (SKILL, skill_file()),
        ];
        assert!(check(&files).is_empty());

        let edited = agents.replace("Before a commit", "Before a push");
        let d = check(&[(AGENTS, edited)]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, Code::AdapterModified);
        assert_eq!(d[0].line, 5);

        let old = agents.replace(&format!("v{VERSION} "), "v0.0.1 ");
        let d = check(&[(AGENTS, old)]);
        assert_eq!(d[0].code, Code::AdapterOutdated);
    }

    #[test]
    fn old_blocks_become_one_line() {
        let old_agents = format!(
            "# Rules\n\n<!-- spec-dna:begin agents v0.0.9 h:{h} -->\n## Specifications\n\nOld text.\n<!-- spec-dna:end agents -->\n",
            h = hex(&hash_normalized("## Specifications\n\nOld text.\n"))
        );
        let d = check(&[(AGENTS, old_agents.clone())]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, Code::AdapterOutdated);
        assert_eq!(upsert_line(&old_agents), format!("# Rules\n\n{}\n", line()));
        assert!(has_own_text(&old_agents));
        let old_claude =
            "<!-- spec-dna:begin claude v0.0.9 h:0 -->\n@AGENTS.md\n<!-- spec-dna:end claude -->\n";
        assert!(!imports_agents(old_claude));
        // The import stays when AGENTS.md has text of its own.
        assert_eq!(migrate_claude(old_claude, true), "@AGENTS.md\n");
        assert_eq!(migrate_claude(old_claude, false), "");
        assert_eq!(upsert_line(old_claude), format!("{}\n", line()));
    }

    #[test]
    fn prose_that_quotes_the_marker_is_not_the_line() {
        let prose = "# Notes\n\nThe line ends with `<!-- spec-dna v<version> h:<hash> -->`.\n";
        assert!(find_line(prose).is_none());
        assert!(check(&[(AGENTS, prose.to_string())]).is_empty());
        let both = upsert_line(prose);
        assert!(both.starts_with(prose), "{both}");
        assert_eq!(remove_line(&both), prose);
        assert!(!has_own_text(&upsert_line("")));
    }

    #[test]
    fn detects_an_existing_import() {
        assert!(imports_agents("# X\n@AGENTS.md\n"));
        assert!(!imports_agents("# X\n"));
    }

    #[test]
    fn the_skill_says_when_to_amend_and_when_to_supersede() {
        // An agent read "amend the decision" as a supersede (defect D2 of
        // 0.1.1), and it learned the amendment section only from SD405.
        let skill = skill_file();
        assert!(
            skill.contains("Amend the decision if its outcome stays"),
            "{skill}"
        );
        assert!(
            skill.contains("under `## Amendments` of the same file"),
            "{skill}"
        );
        assert!(
            skill.contains("Supersede the decision if its outcome changes"),
            "{skill}"
        );
        assert!(
            skill.contains("For an accepted decision, name the new path or identifier in its amendment section."),
            "{skill}"
        );
    }

    #[test]
    fn prefix_stays_small() {
        // The line in one instruction file plus the skill description.
        let bytes = line().len() + 1 + skill_prefix_bytes();
        assert!(bytes <= 300, "{bytes} bytes");
    }
}
