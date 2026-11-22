//! A lint test over each Markdown file that spec-dna generates (design 0.6.1,
//! section 7). The pre-commit hook of the repository runs markdownlint on the
//! files in git only, and the generated files are not in git. This test runs
//! the commands that write them, then reads each `.md` file with `md_lint` of
//! tests/common. The helper keeps the rules of `.markdownlint-cli2.yaml` that
//! a generated file can break (see the comment of the helper).

mod common;

use common::{Repo, md_lint};
use std::fs;

/// The types of the top files of the default graph. `new` takes no id for
/// them.
const TOP_FILE_TYPES: [&str; 5] = ["product", "system", "terms", "roadmap", "questions"];

/// The answers of `init` for the 3 levels of the harness. Each level has the
/// 2 adapters and the skill adapter.
fn answers(level: &str) -> String {
    let mut text = format!(
        "graph = \"default\"\nadapters = [\"agents-md\", \"claude-md\", \"skill\"]\nharness = \"{level}\"\n"
    );
    // The level `none` takes no part of the harness.
    if level != "none" {
        text.push_str("harness-skills = [\"brainstorm\", \"specify\", \"clarify\", \"write-plan\", \"execute\", \"tdd\", \"review\", \"verify\", \"finish\", \"debug\", \"post-mortem\"]\n");
        text.push_str("harness-review-md = true\n");
    }
    if level == "full" {
        text.push_str("evals = \"skeleton\"\n");
    }
    text
}

/// A new repository after `init --answers` for `level`.
fn repo_after_init(level: &str) -> Repo {
    let repo = Repo::new();
    // The answers file is outside the repository, so `init` does not see it.
    let file = format!("{}-answers.toml", repo.dir.display());
    fs::write(&file, answers(level)).unwrap();
    let (code, out, err) = repo.run(&["init", "--answers", &file]);
    let _ = fs::remove_file(&file);
    assert_eq!(code, 0, "init {level}: {out}{err}");
    repo
}

/// Runs a command of the binary and asserts exit code 0.
fn run_ok(repo: &Repo, args: &[&str]) {
    let (code, out, err) = repo.run(args);
    assert_eq!(code, 0, "{args:?}: {out}{err}");
}

/// The embedded templates of `src/templates.rs`: the name of each and the
/// type of its document. The test reads the source text, so a new template is
/// in the test with no change of the test.
fn embedded_templates() -> Vec<(String, String)> {
    let source = include_str!("../src/templates.rs");
    let start = source.find("const EMBEDDED").expect("the table exists");
    let lines: Vec<&str> = source[start..]
        .lines()
        .take_while(|l| !l.starts_with("];"))
        .collect();
    let mut out = Vec::new();
    for window in lines.windows(3) {
        let name = window[1]
            .strip_prefix("        \"")
            .and_then(|l| l.strip_suffix("\","));
        let ty = window[2]
            .strip_prefix("        DocType::")
            .and_then(|l| l.strip_suffix(','));
        if let (true, Some(name), Some(ty)) = (window[0] == "    (", name, ty) {
            out.push((name.to_string(), ty.to_lowercase()));
        }
    }
    out
}

/// Writes each embedded template with `new`, then a stub and an index file.
fn write_documents(repo: &Repo) {
    let templates = embedded_templates();
    assert!(templates.len() >= 18, "the template table: {templates:?}");
    for (name, ty) in &templates {
        // A spec and a plan belong to the change `intent`, which the template
        // `intent` wrote before them.
        let id = match ty.as_str() {
            "spec" | "plan" => "intent".to_string(),
            "intent" => name.clone(),
            _ => format!("{name}-doc"),
        };
        // The summary of an area needs the folder of that area under
        // `requirements/` (design 0.8.0, D50), so the test writes one.
        if ty == "summary" {
            repo.write("docs/requirements/summary-doc/keep.md", "# Keep\n");
        }
        let mut args = vec!["new", ty.as_str()];
        if !TOP_FILE_TYPES.contains(&ty.as_str()) {
            args.push(&id);
        }
        args.extend(["--template", name]);
        run_ok(repo, &args);
    }
    run_ok(
        repo,
        &[
            "mv",
            "docs/rules/rule-doc.md",
            "docs/rules/rule-moved.md",
            "--stub",
        ],
    );
    run_ok(repo, &["index", "docs/research"]);
}

/// Each Markdown file of the repository, as a path with `/` and its text. The
/// list is sorted by path. The folder `.git` is not in it.
fn markdown_files(repo: &Repo) -> Vec<(String, String)> {
    common::tree_bytes(&repo.dir)
        .into_iter()
        .filter(|(path, _)| path.ends_with(".md"))
        .map(|(path, bytes)| (path, String::from_utf8(bytes).expect("UTF-8 text")))
        .collect()
}

/// The rows of a table with an empty cell that has 2 spaces (`|  |`). The rule
/// MD060 of markdownlint (compact style) gives a problem for them, and
/// `md_lint` does not check MD060. A cell with no text is `| |`.
fn empty_cell_rows(text: &str) -> Vec<u32> {
    let mut in_code = false;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
        }
        if !in_code && line.starts_with('|') && line.contains("|  |") {
            out.push(i as u32 + 1);
        }
    }
    out
}

/// The problems of each Markdown file as `[level] path:line rule`.
fn problems(level: &str, repo: &Repo) -> Vec<String> {
    let mut out = Vec::new();
    for (path, text) in markdown_files(repo) {
        for (line, rule) in md_lint(&text) {
            out.push(format!("[{level}] {path}:{line} {rule}"));
        }
        for line in empty_cell_rows(&text) {
            out.push(format!("[{level}] {path}:{line} MD060"));
        }
    }
    out
}

#[test]
fn an_empty_cell_with_2_spaces_is_found() {
    assert_eq!(empty_cell_rows("| a | b |  |\n| c | d | |\n"), [1]);
    assert_eq!(empty_cell_rows("```\n|  |\n```\n"), Vec::<u32>::new());
}

#[test]
fn every_generated_markdown_file_passes_the_lint() {
    let mut found = Vec::new();
    let mut linted: Vec<String> = Vec::new();
    for level in ["none", "lean", "full"] {
        let repo = repo_after_init(level);
        // The documents of `new` come with the level `full`, which has the
        // most files.
        if level == "full" {
            write_documents(&repo);
        }
        found.extend(problems(level, &repo));
        linted.extend(markdown_files(&repo).into_iter().map(|(p, _)| p));
    }
    // The test must read the files that it names. A level that wrote less
    // would pass with no proof.
    for want in [
        "AGENTS.md",
        "CLAUDE.md",
        "REVIEW.md",
        ".claude/skills/spec-dna/SKILL.md",
        ".claude/skills/spec-dna-review/SKILL.md",
        ".claude/agents/Explore.md",
        ".claude/agents/fallback-advisor.md",
        ".claude/agents/implementer.md",
        ".claude/agents/model-team.md",
        "docs/intent/prd/intent.md",
        "docs/intent/rfc/intent.md",
        "docs/rules/rule-doc.md",
        "docs/rules/rule-moved.md",
        "docs/research/README.md",
        "docs/terms.md",
    ] {
        assert!(linted.iter().any(|p| p == want), "not linted: {want}");
    }
    assert!(
        found.is_empty(),
        "the generated files break the lint:\n{}",
        found.join("\n")
    );
}
