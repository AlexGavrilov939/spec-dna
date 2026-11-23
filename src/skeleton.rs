//! The starter files of the evals answer `skeleton` (docs/design/0.6.0.md,
//! "Starter files of `skeleton`"). `init` writes each file once and never
//! again, so a project can edit it. No starter file is Markdown, and the
//! texts are static: the only values are the folder `<dir>` of `[evals]`, the
//! test pathspecs, the base branch and the install step of the CI job.

/// The version of Claude Code that the workflow installs. It is the version of
/// the paid eval runs of the release, so the CI job runs the version that the
/// recorded verdicts measured (0.8.0: 2.1.330). The probes of ADR 0004 ran on
/// 2.1.325, so the version differs from the version of those probes. The
/// agent probes of 0.8.0 (the advisor of a subagent) ran on 2.1.330.
/// `release-engineer` checks it at each release.
pub const CLAUDE_CODE_VERSION: &str = "2.1.330";

/// The text of `<dir>/fixtures/example/notes.txt`.
pub const NOTES: &str = "status: draft\n";

/// Joins lines with a newline after each. A line is a string of its own, so
/// the indent of the generated text is not the indent of this source file.
fn lines(items: &[&str]) -> String {
    items.iter().map(|l| format!("{l}\n")).collect()
}

/// The text of `<dir>/check.sh`. `pathspecs` has one git pathspec for each
/// glob of `[evidence] tests`. A pathspec is one word of the shell, so it
/// needs no quote of its own here: `init` quotes it.
pub fn check_sh(pathspecs: &[String]) -> String {
    let default = if pathspecs.is_empty() {
        "  [ \"$#\" -gt 0 ] || { echo \"tests_unchanged: give the test pathspecs\" >&2; return 2; }"
            .to_string()
    } else {
        format!("  [ \"$#\" -gt 0 ] || set -- {}", pathspecs.join(" "))
    };
    lines(&[
        "# The checks of the evals in this folder. spec-dna eval run sources this",
        "# file of the project, not a copy that the agent can change. It runs each",
        "# check with `sh -c` in the folder of the run. SPEC_DNA_EVAL_BASE is the",
        "# commit of the start state. spec-dna init writes this file once. Edit it.",
        "",
        "# tests_unchanged [pathspec...]: no test file of the start state changed or",
        "# was deleted. A new test file is allowed. The default pathspecs are the",
        "# globs of [evidence] tests at the time of init.",
        "tests_unchanged() {",
        &default,
        "  git diff --quiet \"$SPEC_DNA_EVAL_BASE\" -- \"$@\"",
        "}",
        "",
        "# file_has <file> <text>: the file contains the text.",
        "file_has() {",
        "  grep -q -F -- \"$2\" \"$1\"",
        "}",
        "",
        "# spec_dna_clean: spec-dna check --all finds no error. Use it with a fixture",
        "# that has a spec-dna.toml.",
        "spec_dna_clean() {",
        "  spec-dna check --all >/dev/null",
        "}",
    ])
}

/// The text of `<dir>/example.json`. The task edits the one file of the
/// fixture, and one check calls a function of `check.sh`.
pub fn example_json(dir: &str) -> String {
    lines(&[
        "{",
        "  \"id\": \"example\",",
        "  \"task\": \"In notes.txt, change the word draft to done. Do not change other files.\",",
        &format!("  \"fixture\": \"{dir}/fixtures/example\","),
        "  \"allowed-tools\": [\"Read\", \"Edit\"],",
        "  \"max-turns\": 5,",
        "  \"max-budget-usd\": 1,",
        "  \"runs\": 1,",
        "  \"pass-at\": 1,",
        "  \"checks\": [",
        "    { \"name\": \"notes-done\", \"run\": \"file_has notes.txt 'status: done'\" }",
        "  ]",
        "}",
    ])
}

/// A YAML double-quoted scalar with the same value as `text`. It escapes `\`
/// and `"`, and writes each control character, and the line separators
/// U+2028 and U+2029, as an escape. So a name such as `{x}`, `*dev` or
/// `rel #1` keeps its value.
fn yaml_quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\u{2028}' | '\u{2029}' => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c if c.is_control() => out.push_str(&format!("\\x{:02X}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The text of `.github/workflows/agent-evals.yml`. `checkout` is the step
/// that checks out the repository, and `install` is the step that installs
/// the pinned spec-dna (both with their indent, as `init` writes them in
/// `spec-dna.yml`). The workflow has no marker, so an update of a pin by
/// Dependabot gives no SD501.
pub fn workflow(dir: &str, base: &str, checkout: &str, install: &str) -> String {
    let dir_path = format!("      - {dir}/**");
    let paths = [
        "      - CLAUDE.md",
        "      - AGENTS.md",
        "      - .claude/agents/**",
        "      - .claude/skills/**",
        "      - .claude/settings.json",
        &dir_path,
    ];
    let base_line = format!("      - {}", yaml_quote(base));
    let version_line = format!("      CLAUDE_CODE_VERSION: \"{CLAUDE_CODE_VERSION}\"");
    let mut head = vec!["name: Agent evals", "on:", "  pull_request:", "    paths:"];
    head.extend(paths);
    head.extend(["  push:", "    branches:", &base_line, "    paths:"]);
    head.extend(paths);
    head.extend([
        "  schedule:",
        "    - cron: \"23 4 * * 1\"",
        "  workflow_dispatch:",
        "permissions: {}",
        "concurrency:",
        "  group: agent-evals-${{ github.ref }}",
        "  cancel-in-progress: true",
        "jobs:",
        "  evals:",
        "    # The job runs only when the repository variable SPEC_DNA_EVALS is true.",
        "    # A pull request from a fork gets no secrets, so the job skips it.",
        "    if: vars.SPEC_DNA_EVALS == 'true' && (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository)",
        "    runs-on: ubuntu-24.04",
        "    timeout-minutes: 60",
        "    permissions:",
        "      contents: read",
        "    env:",
        &version_line,
        "      DISABLE_AUTOUPDATER: \"1\"",
        "    steps:",
    ]);
    let tail = lines(&[
        "      - name: Install Claude Code",
        "        run: npm install --global \"@anthropic-ai/claude-code@$CLAUDE_CODE_VERSION\" # zizmor: ignore[adhoc-packages]",
        "      - name: Run the evals",
        "        env:",
        "          ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}",
        "        run: spec-dna eval run",
    ]);
    format!("{}{checkout}{install}{tail}", lines(&head))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_sh_has_one_branch_for_each_pathspec_list() {
        let some = check_sh(&["':(glob)**/tests/**'".into(), "':(glob)**/*_test.*'".into()]);
        assert!(
            some.contains(
                "  [ \"$#\" -gt 0 ] || set -- ':(glob)**/tests/**' ':(glob)**/*_test.*'\n"
            ),
            "{some}"
        );
        let none = check_sh(&[]);
        assert!(none.contains("return 2; }\n"), "{none}");
        assert!(!none.contains("set -- "), "{none}");
    }

    #[test]
    fn the_base_branch_is_a_double_quoted_yaml_scalar() {
        let cases = [
            ("main", "      - \"main\"\n"),
            ("{x}", "      - \"{x}\"\n"),
            ("*dev", "      - \"*dev\"\n"),
            ("rel #1", "      - \"rel #1\"\n"),
            ("a\\b\"c", "      - \"a\\\\b\\\"c\"\n"),
            ("a\u{1}b\u{85}c", "      - \"a\\x01b\\x85c\"\n"),
        ];
        for (base, line) in cases {
            let text = workflow("evals", base, "", "");
            assert!(
                text.contains(&format!("    branches:\n{line}    paths:\n")),
                "{base}: {text}"
            );
        }
    }

    #[test]
    fn no_starter_file_has_a_shebang_or_a_marker() {
        for t in [
            check_sh(&[]),
            example_json("evals"),
            NOTES.to_string(),
            workflow("evals", "main", "", ""),
        ] {
            assert!(!t.starts_with("#!"), "{t}");
            assert!(!t.contains("spec-dna harness"), "{t}");
            assert!(t.ends_with('\n') && !t.contains(" \n"), "{t}");
        }
    }

    #[test]
    fn the_workflow_is_the_only_text_with_a_secret_and_never_uses_the_target_event() {
        let w = workflow("evals", "main", "", "");
        assert!(w.contains("secrets.ANTHROPIC_API_KEY"));
        assert!(!w.contains("pull_request_target"));
        assert!(w.contains("\npermissions: {}\n"));
        assert!(w.contains("group: agent-evals-${{ github.ref }}\n"));
    }
}
