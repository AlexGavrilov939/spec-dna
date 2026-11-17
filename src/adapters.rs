//! Generated agent adapters (ADR 0003, amendment of 2026-11-08): one line in
//! each root instruction file (`AGENTS.md`, `CLAUDE.md`) and one skill. Each
//! adapter contains the generator version and a hash of its normalized text.
//! Thus `check` can report a manual edit or an old version. The line never
//! imports another file, so it adds only its own bytes to the prefix of each
//! agent request.

use crate::diag::{Code, Diag};
use crate::hash::{hash_normalized, hex};
use crate::layer::Graph;
use crate::model::DocType;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AGENTS: &str = "AGENTS.md";
pub const CLAUDE: &str = "CLAUDE.md";
pub const SKILL: &str = ".claude/skills/spec-dna/SKILL.md";
pub const PATHS: [&str; 3] = [AGENTS, CLAUDE, SKILL];

/// The sentence of the approval rule (Q5). The line of `AGENTS.md` and the
/// body of each skill (`harness_text::APPROVAL_RULE`) both use it, so they
/// state the same words. A macro, because `concat!` needs a literal.
macro_rules! approval_sentence {
    () => {
        "An agent writes an approval commit only when the owner says so explicitly in the chat."
    };
}
pub(crate) use approval_sentence;

/// The text of the line (ADR 0003, ADR 0010, ADR 0011). The whole line, with
/// its marker, has 400 bytes or fewer (D6), and each request carries it. It
/// names the 5 disciplines (D3) with the skill of each, the command that
/// prints the detail (D4), and the approval rule of the owner (Q5).
const LINE_BODY: &str = concat!(
    "If a task edits code or docs, run `spec-dna context <files>` once. Before a commit, run `spec-dna check`. Rules: test first (tdd), cause first (debug), proof before done (verify), review is data (review), isolated work (execute). Detail: `spec-dna skill show <name>`. ",
    approval_sentence!()
);

/// The skill description. An agent loads the description of each skill into
/// every request, so it must stay short.
const SKILL_DESCRIPTION: &str = "Loads and checks the specs for a task. Use it before you change code or docs, and before a commit.";

/// The skill body. The agent reads it once per session, and every later
/// request carries it, so each line must change what the agent does.
const SKILL_BODY: &str = "# spec-dna

1. If the task changes code or docs, run `spec-dna context <file>...` once. Give all the files that you plan to change (or a document id). Do not run it for a question without edits. The output gives the sections in priority order: rules, the entry, the sections that cite the files, their decisions. Each section has an id.
2. Read a listed section with `spec-dna show <id>#<anchor>` only if the task needs it.
3. Before you edit a document, run `spec-dna show --conventions <type>` once for its type. Follow the project conventions that it prints. `context` serves them first when a target is a document of that type.
4. Keep the documents true in the same change. Write a requirement as `- **<key>**: <sentence>`. Use a lowercase slug key, an EARS pattern and an uppercase BCP 14 keyword. Cite it as `<doc-id>#<key>`. To link a test to a requirement, write `covers: <doc-id>#<key>` in the test.
5. For a new document, run `spec-dna new <type> <id>`. The folder of a layer gives the type, so write no `type:` line there. A top file has no id (`spec-dna new terms`), and a change has a folder (`spec-dna new intent <slug>`). Then replace each placeholder. Types of this project: {types}.
6. Do not change the text of an accepted decision outside its amendment and live sections (`show --conventions decision` names them. The default is `## Amendments`).
    - Update a live section, such as a progress section, in place when your change makes it stale.
    - Amend the decision if its outcome stays. Add the detail, the limit, the fix after an incident or the new code path under `## Amendments` of the same file. A task that says \"amend the decision\" means this.
    - Supersede the decision if its outcome changes. Write a new decision with `supersedes: [<old id>]`. Set the status of the old decision to `superseded`.
7. Before you finish, run `spec-dna check` once (or the project target that runs it). Fix each error, and each SD606 or SD607 warning (a document cites code that your change removed). For an accepted decision, name the new path or identifier in its amendment section. The output gives the fix of each code. Run `check` again only after a fix. Other warnings need no change.
8. Do not split, move or shorten a document in a task for a size warning (SD701, SD703, SD704). For a size error (SD702), do one of these actions: put your new text into a new document and link it, or move whole sections with one `spec-dna mv <doc-id>#<section>... <to>`. Then the old ids stay as aliases.
9. Do not edit `spec-dna.toml`. A configuration change is a change of its own, by the project owner. `check` reports a change of `spec-dna.toml` together with other files as SD503.

{disciplines}

{replace_rule}";

/// The block of the 5 disciplines (D3). Each rule is one line of 140 bytes or
/// fewer, and it names the skill that holds the detail. The block has 700
/// bytes or fewer (D6). A rule works when its skill is off, because
/// `spec-dna skill show <name>` prints the detail.
pub const DISCIPLINES: &str = "## Disciplines

Each rule names the skill with the detail. Run `spec-dna skill show <name>` to read it, also when the skill is off.

- Test first: write a test that fails before the code (`tdd`).
- Cause first: find the cause of a failure before the fix. After 3 failed fixes, stop and ask (`debug`).
- Proof before done: run the proof and read its output before you say \"done\" (`verify`).
- Review is data: check each review comment against the code before you act on it (`review`).
- Isolated work: do each change in its own git worktree or branch (`execute`).";

/// The rule for a project that also has another workflow plugin (D5).
const REPLACE_RULE: &str = "A spec-dna skill replaces a general skill of the same stage. For `write-plan` it replaces `writing-plans`.";

/// The generated line of a root instruction file.
pub fn line() -> String {
    format!(
        "{LINE_BODY} <!-- spec-dna v{VERSION} h:{} -->",
        hex(&hash_normalized(LINE_BODY))
    )
}

/// The document types of a project for the skill: the types that a layer of
/// the graph has. If the `types` list of the file is not empty, only the
/// types of that list. `init` and `check` both use it, so they agree on the text.
pub fn project_types(graph: &Graph, types: &[DocType]) -> Vec<DocType> {
    DocType::ALL
        .into_iter()
        .filter(|t| graph.layer_of_type(*t).is_some() && (types.is_empty() || types.contains(t)))
        .collect()
}

/// The text of the skill body for a project. `types` are the types of the
/// project ([`project_types`]). An empty list allows every type.
fn skill_body(types: &[DocType]) -> String {
    let names: Vec<&str> = DocType::ALL
        .into_iter()
        .filter(|t| types.is_empty() || types.contains(t))
        .map(DocType::name)
        .collect();
    SKILL_BODY
        .replace("{types}", &names.join(", "))
        .replace("{disciplines}", DISCIPLINES)
        .replace("{replace_rule}", REPLACE_RULE)
}

/// The skill block, between begin and end markers.
pub fn block(name: &str, types: &[DocType]) -> String {
    let body = skill_body(types);
    format!(
        "<!-- spec-dna:begin {name} v{VERSION} h:{} -->\n{body}\n<!-- spec-dna:end {name} -->\n",
        hex(&hash_normalized(&body))
    )
}

/// The skill file of a project with the document types `types`.
pub fn skill_file(types: &[DocType]) -> String {
    format!(
        "---\nname: spec-dna\ndescription: {SKILL_DESCRIPTION}\n---\n\n{}",
        block("skill", types)
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

/// The 5 disciplines of D3: the skill, the item in the `Rules:` list of the
/// line, and the start of the rule in the block `Disciplines` of the skill.
const DISCIPLINE_ITEMS: [(&str, &str, &str); 5] = [
    ("tdd", "test first (tdd)", "- Test first:"),
    ("debug", "cause first (debug)", "- Cause first:"),
    (
        "verify",
        "proof before done (verify)",
        "- Proof before done:",
    ),
    ("review", "review is data (review)", "- Review is data:"),
    ("execute", "isolated work (execute)", "- Isolated work:"),
];

/// The line without the discipline of `skill`, with its own marker. The
/// `eval run --ab` arm `without` uses it. For a skill with no discipline, it
/// is the line of `line()`.
pub fn line_without(skill: &str) -> String {
    let body = match DISCIPLINE_ITEMS.iter().find(|d| d.0 == skill) {
        Some((_, item, _)) => body_without(LINE_BODY, item),
        None => LINE_BODY.to_string(),
    };
    format!(
        "{body} <!-- spec-dna v{VERSION} h:{} -->",
        hex(&hash_normalized(&body))
    )
}

/// The text of the line without one item of its list `Rules: a, b, c.`. If
/// the list has no other item, the sentence goes too.
fn body_without(body: &str, item: &str) -> String {
    const START: &str = "Rules: ";
    const END: &str = ". Detail:";
    let (Some(a), Some(b)) = (body.find(START), body.find(END)) else {
        return body.to_string();
    };
    let kept: Vec<&str> = body[a + START.len()..b]
        .split(", ")
        .filter(|i| *i != item)
        .collect();
    if kept.is_empty() {
        format!("{}{}", &body[..a], &body[b + ". ".len()..])
    } else {
        format!("{}{START}{}{}", &body[..a], kept.join(", "), &body[b..])
    }
}

/// The text of a generated file without the name of the skill `skill`: the
/// discipline in the generated line of a root instruction file, and in the
/// block `Disciplines` of the adapter skill, and the rule for `write-plan`
/// (`REPLACE_RULE`). A text with none of them stays as it is. The marker of
/// each changed text holds the new hash.
pub fn without_skill(text: &str, skill: &str) -> String {
    let mut out = text.to_string();
    if let Some((start, end, ..)) = find_line(&out) {
        let line = format!("{}\n", line_without(skill));
        out = format!("{}{line}{}", &out[..start], &out[end..]);
    }
    if let Some((start, end, _, _, body)) = find_block(&out, "skill") {
        let mut new = body.to_string();
        if let Some((_, _, bullet)) = DISCIPLINE_ITEMS.iter().find(|d| d.0 == skill) {
            new = new
                .lines()
                .filter(|l| !l.starts_with(bullet))
                .collect::<Vec<_>>()
                .join("\n");
            new.push('\n');
        }
        if skill == "write-plan" {
            new = new.replace(&format!("\n\n{REPLACE_RULE}"), "");
        }
        if new != body {
            let new = new.trim_end_matches('\n');
            let block = format!(
                "<!-- spec-dna:begin skill v{VERSION} h:{} -->\n{new}\n<!-- spec-dna:end skill -->\n",
                hex(&hash_normalized(new))
            );
            out = format!("{}{block}{}", &out[..start], &out[end..]);
        }
    }
    out
}

/// Returns true if the text has content that spec-dna did not generate.
pub fn has_own_text(text: &str) -> bool {
    !remove_line(&remove_old_blocks(text)).trim().is_empty()
}

/// Replaces the `claude` block of an older version, which imported
/// `AGENTS.md`. If `keep_import` is true (`AGENTS.md` has text of its own),
/// the import stays as a plain line, so that the agent still reads that text.
/// Otherwise, it removes the block.
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
pub fn check(files: &[(&str, String)], types: &[DocType]) -> Vec<Diag> {
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
            } else if text.as_str() != skill_file(types) {
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
/// replaces the current line or an old block. Otherwise, it adds the line at
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
            (SKILL, skill_file(&[])),
        ];
        assert!(check(&files, &[]).is_empty());

        let edited = agents.replace("Before a commit", "Before a push");
        let d = check(&[(AGENTS, edited)], &[]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, Code::AdapterModified);
        assert_eq!(d[0].line, 5);

        let old = agents.replace(&format!("v{VERSION} "), "v0.0.1 ");
        let d = check(&[(AGENTS, old)], &[]);
        assert_eq!(d[0].code, Code::AdapterOutdated);
    }

    #[test]
    fn old_blocks_become_one_line() {
        let old_agents = format!(
            "# Rules\n\n<!-- spec-dna:begin agents v0.0.9 h:{h} -->\n## Specifications\n\nOld text.\n<!-- spec-dna:end agents -->\n",
            h = hex(&hash_normalized("## Specifications\n\nOld text.\n"))
        );
        let d = check(&[(AGENTS, old_agents.clone())], &[]);
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
        assert!(check(&[(AGENTS, prose.to_string())], &[]).is_empty());
        let both = upsert_line(prose);
        assert!(both.starts_with(prose), "{both}");
        assert_eq!(remove_line(&both), prose);
        assert!(!has_own_text(&upsert_line("")));
    }

    #[test]
    fn the_line_and_the_discipline_block_keep_their_limits() {
        // D6: the line has 400 bytes or fewer with its marker, and the block
        // of disciplines has 700 bytes or fewer.
        let l = line();
        assert!(l.len() <= 400, "{} bytes: {l}", l.len());
        assert!(DISCIPLINES.len() <= 700, "{} bytes", DISCIPLINES.len());
        let rules: Vec<&str> = DISCIPLINES
            .lines()
            .filter(|l| l.starts_with("- "))
            .collect();
        assert_eq!(rules.len(), 5);
        for r in &rules {
            assert!(r.len() <= 140, "{} bytes: {r}", r.len());
        }
        // The line names the 5 skills, the command and the approval rule.
        for w in ["tdd", "debug", "verify", "review", "execute"] {
            assert!(l.contains(w), "{w}");
        }
        assert!(l.contains("`spec-dna skill show <name>`"));
        assert!(l.contains(approval_sentence!()));
        // The skill body holds the block and the rule of D5.
        let skill = skill_file(&[]);
        assert!(skill.contains(DISCIPLINES));
        assert!(skill.contains(REPLACE_RULE));
        assert!(!skill.contains("{disciplines}") && !skill.contains("{replace_rule}"));
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
        let skill = skill_file(&[]);
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
    fn the_skill_names_live_sections_and_conventions() {
        let skill = skill_file(&[]);
        assert!(
            skill.contains("Update a live section, such as a progress section, in place"),
            "{skill}"
        );
        assert!(
            skill.contains("run `spec-dna show --conventions <type>` once for its type"),
            "{skill}"
        );
    }

    #[test]
    fn the_skill_lists_the_types_of_the_project() {
        let all = skill_file(&[]);
        for t in DocType::ALL {
            assert!(all.contains(t.name()), "{}", t.name());
        }
        assert_eq!(
            skill_file(&DocType::ALL),
            all,
            "all types and no list agree"
        );
        let few = skill_file(&[DocType::Decision, DocType::Requirements]);
        assert!(
            few.contains("Types of this project: requirements, decision."),
            "{few}"
        );
        assert!(!few.contains("procedure, incident"), "{few}");
        // The hash follows the text, so each list has its own valid block.
        let files = vec![(SKILL, few)];
        assert!(check(&files, &[DocType::Decision, DocType::Requirements]).is_empty());
        let d = check(&files, &[]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, Code::AdapterOutdated);
    }

    #[test]
    fn prefix_stays_small() {
        // The line in one instruction file plus the skill description. The
        // line has 400 bytes or fewer (D6), and a line feed ends it.
        let bytes = line().len() + 1 + skill_prefix_bytes();
        assert!(bytes <= 401 + 111, "{bytes} bytes");
    }

    #[test]
    fn each_discipline_item_is_in_the_line_and_in_the_block() {
        for (skill, item, bullet) in DISCIPLINE_ITEMS {
            assert!(LINE_BODY.contains(item), "{skill}");
            assert!(DISCIPLINES.contains(bullet), "{skill}");
            assert!(DISCIPLINES.contains(&format!("(`{skill}`)")), "{skill}");
        }
    }

    #[test]
    fn the_line_without_a_discipline_loses_one_item_and_keeps_a_valid_marker() {
        for (skill, item, _) in DISCIPLINE_ITEMS {
            let l = line_without(skill);
            assert!(!l.contains(item), "{skill}: {l}");
            assert!(!l.contains(&format!("({skill})")), "{skill}: {l}");
            let (_, _, version, hash, body) = find_line(&l).expect("a generated line");
            assert_eq!(version, VERSION);
            assert_eq!(hash, hex(&hash_normalized(body)));
            assert!(l.contains("Rules: "), "{l}");
            assert!(l.contains(". Detail: `spec-dna skill show <name>`."), "{l}");
        }
        assert!(line_without("tdd").contains("Rules: cause first (debug), proof"));
        assert!(line_without("execute").contains("review is data (review). Detail:"));
        // A skill with no discipline keeps the line.
        assert_eq!(line_without("specify"), line());
        // The last item of the list: the sentence goes too.
        assert_eq!(
            body_without(
                "Run it. Rules: test first (tdd). Detail: x.",
                "test first (tdd)"
            ),
            "Run it. Detail: x."
        );
    }

    #[test]
    fn a_text_without_a_skill_loses_the_discipline_in_the_line_and_in_the_block() {
        let agents = upsert_line("# Rules\n\nOwn text.\n");
        let skill = skill_file(&[]);
        for (name, _, bullet) in DISCIPLINE_ITEMS {
            let a = without_skill(&agents, name);
            assert!(a.starts_with("# Rules\n\nOwn text.\n\n"), "{a}");
            assert!(!a.contains(&format!("({name})")), "{a}");
            assert!(find_line(&a).is_some(), "{a}");
            let k = without_skill(&skill, name);
            assert!(!k.contains(bullet), "{name}");
            assert!(!k.contains(&format!("(`{name}`)")), "{name}");
            let (_, _, _, hash, body) = find_block(&k, "skill").expect("the block");
            assert_eq!(hash, hex(&hash_normalized(body.trim_end_matches('\n'))));
            // Only one bullet goes.
            assert_eq!(k.matches("\n- ").count() + 1, skill.matches("\n- ").count());
        }
        // The rule of `write-plan` goes with its name.
        let k = without_skill(&skill, "write-plan");
        assert!(!k.contains("write-plan"), "{k}");
        assert!(skill.contains("write-plan"));
        // A skill with no discipline and no rule: the text stays.
        assert_eq!(without_skill(&agents, "specify"), agents);
        assert_eq!(without_skill(&skill, "specify"), skill);
        assert_eq!(without_skill("plain text\n", "tdd"), "plain text\n");
    }
}
