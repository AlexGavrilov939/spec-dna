//! Runs the scripts of `examples/` and checks the pages against their real
//! output. Thus a page cannot show output that the current version does not
//! give.
//!
//! The output of a script is a list of steps. A step starts with a line
//! `$ <command>` and holds the output lines of that command. A line that
//! starts with `== ` is an explanation of the script, and it ends the step.
//! A `console` block of a page is a list of steps in the same form.
//!
//! - Each step of a `console` block of an example page must match a step of
//!   its script with the same command. The steps keep the order of the page,
//!   also from one block to the next.
//! - Each `console` block of `README.md` must match the steps of one script.
//! - Each `md`, `toml` and `python` block of an example page must be a part of
//!   a file that the script of that example writes (a here-document).
//!
//! A step of a page matches when its lines are the output lines of the step
//! of the script, from the first line to the last line. A line `...` stands
//! for 0 or more lines that the page leaves out. The comparison masks each
//! version and each commit id (a word of 12 hexadecimal digits), because the
//! commit ids change with the version in `spec-dna.toml`. Thus the pages stay
//! true after a release.
#![cfg(unix)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const EXAMPLES: [&str; 3] = ["first-project", "existing-repo", "one-change"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &PathBuf) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The output of each example script, in the order of `EXAMPLES`. Each script
/// runs once for all tests of this file.
fn outputs() -> &'static Vec<String> {
    static OUTPUTS: OnceLock<Vec<String>> = OnceLock::new();
    OUTPUTS.get_or_init(|| EXAMPLES.iter().map(|name| run_example(name)).collect())
}

fn run_example(name: &str) -> String {
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_spec-dna"));
    let bin_dir = bin.parent().expect("the binary has a folder");
    let path = format!(
        "{}:{}",
        bin_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let work = std::env::temp_dir().join(format!("spec-dna-example-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    let script = root().join("examples").join(name).join("run.sh");
    let out = Command::new("bash")
        .arg("-c")
        .arg("\"$0\" \"$1\" 2>&1")
        .arg(&script)
        .arg(&work)
        .env("PATH", path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("bash runs");
    let _ = std::fs::remove_dir_all(&work);
    let text = String::from_utf8(out.stdout).expect("the output is UTF-8");
    assert!(
        out.status.success(),
        "examples/{name}/run.sh failed:\n{text}"
    );
    text
}

/// Replaces each version `<n>.<n>.<n>` with `<version>`, and each word of
/// exactly 12 lowercase hexadecimal digits (a commit id) with `<commit>`.
fn mask(text: &str) -> String {
    let c: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < c.len() {
        let starts_word = i == 0 || !c[i - 1].is_ascii_alphanumeric();
        let starts_version = starts_word && (i == 0 || c[i - 1] != '.');
        if starts_version && let Some(end) = version_end(&c, i) {
            out.push_str("<version>");
            i = end;
            continue;
        }
        let mut j = i;
        while j < c.len() && matches!(c[j], '0'..='9' | 'a'..='f') {
            j += 1;
        }
        let ends_word = j == c.len() || !c[j].is_ascii_alphanumeric();
        if starts_word && ends_word && j - i == 12 {
            out.push_str("<commit>");
            i = j;
        } else if j > i {
            out.extend(&c[i..j]);
            i = j;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

/// The end of a version `<n>.<n>.<n>` that starts at `i`, if one starts there.
fn version_end(c: &[char], mut i: usize) -> Option<usize> {
    for part in 0..3 {
        let start = i;
        while i < c.len() && c[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
        if part < 2 {
            if i < c.len() && c[i] == '.' {
                i += 1;
            } else {
                return None;
            }
        }
    }
    (i == c.len() || !c[i].is_ascii_alphanumeric()).then_some(i)
}

/// The fenced code blocks of a Markdown text with one of the language tags.
fn blocks(markdown: &str, langs: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in markdown.lines() {
        match current.as_mut() {
            Some(lines) if line.trim_start() == "```" => {
                found.push(lines.join("\n"));
                current = None;
            }
            Some(lines) => lines.push(line),
            None => {
                if let Some(lang) = line.trim_start().strip_prefix("```")
                    && langs.contains(&lang.trim())
                {
                    current = Some(Vec::new());
                }
            }
        }
    }
    found
}

#[derive(Debug)]
struct Step {
    command: String,
    lines: Vec<String>,
}

/// The steps of a script output or of a `console` block. The empty lines at
/// the end of a step are not part of it.
fn steps(text: &str) -> Vec<Step> {
    let mut found: Vec<Step> = Vec::new();
    let mut open = false;
    for line in text.lines() {
        if let Some(command) = line.strip_prefix("$ ") {
            found.push(Step {
                command: command.to_string(),
                lines: Vec::new(),
            });
            open = true;
        } else if line.starts_with("== ") {
            open = false;
        } else if open && let Some(step) = found.last_mut() {
            step.lines.push(line.to_string());
        }
    }
    for step in &mut found {
        while step.lines.last().is_some_and(|line| line.is_empty()) {
            step.lines.pop();
        }
    }
    found
}

/// True if `want` equals `have`. A line `...` of `want` stands for 0 or more
/// lines of `have`.
fn lines_match(want: &[String], have: &[String]) -> bool {
    match want.split_first() {
        None => have.is_empty(),
        Some((first, rest)) if first == "..." => {
            (0..=have.len()).any(|skip| lines_match(rest, &have[skip..]))
        }
        Some((first, rest)) => have.first() == Some(first) && lines_match(rest, &have[1..]),
    }
}

/// Checks that the steps of the blocks occur in the steps of the output, in
/// their order. Gives the step of a block that has no match.
fn check_blocks(blocks: &[String], output: &str) -> Result<(), String> {
    let have = steps(&mask(output));
    let mut from = 0;
    for block in blocks {
        let masked = mask(block);
        if !masked.starts_with("$ ") {
            return Err(format!(
                "a console block must start with a line `$ <command>`:\n{block}"
            ));
        }
        for want in steps(&masked) {
            let found = (from..have.len()).find(|&i| {
                have[i].command == want.command && lines_match(&want.lines, &have[i].lines)
            });
            match found {
                Some(i) => from = i + 1,
                None => {
                    return Err(format!(
                        "no later step of the script gives this output:\n$ {}\n{}",
                        want.command,
                        want.lines.join("\n")
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Gives the first part of `block` that is not in `text`, with the parts in
/// their order. A line `...` separates the parts.
fn missing_part(block: &str, text: &str) -> Option<String> {
    let mut from = 0;
    for part in block.split("\n...\n") {
        let part = part.trim_matches('\n');
        match text[from..].find(part) {
            Some(at) => from += at + part.len(),
            None => return Some(part.to_string()),
        }
    }
    None
}

#[test]
fn each_example_page_shows_the_real_output_and_the_files_of_its_script() {
    for (name, output) in EXAMPLES.iter().zip(outputs()) {
        let dir = root().join("examples").join(name);
        let page = read(&dir.join("README.md"));
        let script = read(&dir.join("run.sh"));
        let consoles = blocks(&page, &["console"]);
        assert!(
            !consoles.is_empty(),
            "examples/{name}/README.md has no console block"
        );
        if let Err(problem) = check_blocks(&consoles, output) {
            panic!("examples/{name}/README.md: {problem}\n\nThe output of run.sh:\n{output}");
        }
        let files = heredocs(&script);
        for block in blocks(&page, &["md", "toml", "python"]) {
            let found = files
                .iter()
                .any(|file| missing_part(&block, file).is_none());
            assert!(
                found,
                "examples/{name}/README.md: no file that run.sh writes has this text:\n{block}"
            );
        }
    }
}

/// The bodies of the here-documents of a script: the text of the files that
/// the script writes with `cat > <file> <<'EOF'` or `cat >> <file> <<'EOF'`.
fn heredocs(script: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in script.lines() {
        match current.as_mut() {
            Some(lines) if line == "EOF" => {
                found.push(lines.join("\n"));
                current = None;
            }
            Some(lines) => lines.push(line),
            None if line.contains("<<'EOF'") => current = Some(Vec::new()),
            None => {}
        }
    }
    found
}

/// A console block with another tag, such as `Console` or `console {.x}`,
/// escapes the checks. Thus each page must use the exact tag `console`.
#[test]
fn each_console_block_has_the_exact_tag() {
    let mut pages = vec![root().join("README.md")];
    pages.extend(
        EXAMPLES
            .iter()
            .map(|name| root().join("examples").join(name).join("README.md")),
    );
    for page in pages {
        for line in read(&page).lines() {
            let fence = line.trim_start();
            let Some(tag) = fence
                .strip_prefix("```")
                .or_else(|| fence.strip_prefix("~~~"))
            else {
                continue;
            };
            let tag = tag.trim();
            assert!(
                !tag.to_lowercase().starts_with("console") || tag == "console",
                "{}: use the exact tag `console`, not `{tag}`",
                page.display()
            );
        }
    }
}

#[test]
fn each_console_block_of_the_readme_comes_from_an_example() {
    let readme = read(&root().join("README.md"));
    let consoles = blocks(&readme, &["console"]);
    assert!(!consoles.is_empty(), "README.md has no console block");
    for block in consoles {
        let found = outputs()
            .iter()
            .any(|output| check_blocks(std::slice::from_ref(&block), output).is_ok());
        assert!(
            found,
            "README.md: this console block is not in the output of an example:\n{block}"
        );
    }
}

#[test]
fn mask_hides_versions_and_commit_ids_only() {
    assert_eq!(
        mask("context 0.6.1: against 81cc7208f3b8 (index 2bad2b0..cd695f8) h:c70424c66f956daa"),
        "context <version>: against <commit> (index 2bad2b0..cd695f8) h:c70424c66f956daa"
    );
    assert_eq!(
        mask("cancelled 2026-11-16 SD405 abc1234x"),
        "cancelled 2026-11-16 SD405 abc1234x"
    );
}

#[test]
fn a_block_matches_whole_steps_in_their_order() {
    let output = "== start\n\n$ check\nok: 0 errors\n\n== edit\n\n$ check\nerror E1\nok: 1 error\n\n$ show\nA\nB\nC\n";
    let ok = |block: &str| check_blocks(&[block.to_string()], output).is_ok();
    assert!(ok("$ check\nerror E1\nok: 1 error"));
    assert!(ok("$ check\nok: 0 errors\n\n$ show\nA\n...\nC"));
    assert!(ok("$ show\n...\nC"));
    // A part of a line, a cut with no `...`, and a wrong order do not match.
    assert!(!ok("$ check\nok: 1"));
    assert!(!ok("$ show\nA\nB"));
    assert!(!ok("$ show\nA\n\n$ check\nok: 0 errors"));
    // The output of the first `check` does not prove a page that shows it
    // after the second `check`.
    let page = [
        "$ check\nerror E1\nok: 1 error".to_string(),
        "$ check\nok: 0 errors".to_string(),
    ];
    assert!(check_blocks(&page, output).is_err());
}
